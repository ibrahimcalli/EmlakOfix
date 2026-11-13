//! Ofis sunucusu: SQLite + basit HTTP/JSON API. Aynı program hem sunucu hem istemci olarak çalışır.
use crate::common::*;
use crate::db::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct State {
    pub db: Mutex<Db>,
    pub sessions: Mutex<HashMap<String, (i64, Instant)>>,
    pub rev: AtomicU64,
    pub port: u16,
    pub data_dir: PathBuf,
    /// HTTPS açıksa sertifika parmak izi (sha256 hex)
    pub tls_fp: Option<String>,
}

pub enum Resp {
    Json(Value),
    Bytes(Vec<u8>, &'static str),
}

fn ok_json<T: Serialize>(v: &T) -> R<Resp> {
    Ok(Resp::Json(serde_json::to_value(v).map_err(|e| ApiErr(500, e.to_string()))?))
}
fn done() -> R<Resp> {
    Ok(Resp::Json(json!({"ok": true})))
}
fn parse<T: DeserializeOwned>(b: &[u8]) -> R<T> {
    serde_json::from_slice(b).map_err(|e| ApiErr::bad(format!("Geçersiz istek: {e}")))
}

pub fn local_ips() -> Vec<String> {
    let mut v = Vec::new();
    for target in ["8.8.8.8:80", "192.168.1.1:80", "192.168.0.1:80", "10.0.0.1:80"] {
        if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
            if s.connect(target).is_ok() {
                if let Ok(a) = s.local_addr() {
                    let ip = a.ip().to_string();
                    if !ip.starts_with("127.") && !ip.starts_with("0.") && !v.contains(&ip) {
                        v.push(ip);
                    }
                }
            }
        }
    }
    v
}

/// Portu bağlar; güncelleme/geri yükleme sonrası eski süreç kapanırken kısa süre yeniden dener.
/// Port gerçekten başka bir Emlak Ofis penceresinde doluysa anlaşılır bir mesaj döner.
fn bind_one(port: u16, tls: &Option<crate::tls::ServerCert>) -> Result<tiny_http::Server, String> {
    match tls {
        Some(c) => tiny_http::Server::https(("0.0.0.0", port), tiny_http::SslConfig { certificate: c.cert_pem.clone(), private_key: c.key_pem.clone() }).map_err(|e| e.to_string()),
        None => tiny_http::Server::http(("0.0.0.0", port)).map_err(|e| e.to_string()),
    }
}

fn bind_with_retry(port: u16, tls: &Option<crate::tls::ServerCert>) -> Result<tiny_http::Server, String> {
    let mut last = String::new();
    for i in 0..12 {
        match bind_one(port, tls) {
            Ok(s) => return Ok(s),
            Err(e) => last = e,
        }
        if i == 0 && already_running(port) {
            // Yeniden başlatma yarışı olabilir: kısa bekle, hâlâ açıksa gerçekten ikinci kopyadır.
            std::thread::sleep(std::time::Duration::from_secs(3));
            if let Ok(s) = bind_one(port, tls) { return Ok(s); }
            if already_running(port) {
                return Err(format!("Emlak Ofis bu bilgisayarda zaten açık ({port} portu kullanımda). Lütfen açık olan diğer Emlak Ofis penceresini kullanın veya önce onu kapatın."));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Err(format!("{port} portu açılamadı (başka bir program kullanıyor olabilir): {last}"))
}

fn already_running(port: u16) -> bool {
    crate::api::ping_any(&format!("127.0.0.1:{port}")).map(|p| p.app == "emlak-ofis").unwrap_or(false)
}

#[allow(dead_code)]
pub fn start(data_dir: PathBuf, port: u16) -> Result<Arc<State>, String> {
    start_with(data_dir, port, false)
}

pub fn start_with(data_dir: PathBuf, port: u16, https: bool) -> Result<Arc<State>, String> {
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("Veri klasörü oluşturulamadı: {e}"))?;
    let tls = if https { Some(crate::tls::ensure_cert(&data_dir)?) } else { None };
    // Önce portu bağla: ikinci kopya veritabanına dokunmadan net hata versin.
    let server = bind_with_retry(port, &tls)?;
    let db = Db::open(&data_dir.join("emlak.db")).map_err(|e| e.1)?;
    let _ = db.make_backup();
    let state = Arc::new(State {
        db: Mutex::new(db),
        sessions: Mutex::new(HashMap::new()),
        rev: AtomicU64::new(chrono::Local::now().timestamp() as u64),
        port,
        data_dir,
        tls_fp: tls.as_ref().map(|c| c.fingerprint.clone()),
    });
    let server = Arc::new(server);
    for _ in 0..8 {
        let s = server.clone();
        let st = state.clone();
        std::thread::spawn(move || {
            while let Ok(req) = s.recv() {
                handle_request(&st, req);
            }
        });
    }
    // Otomatik günlük yedek
    {
        let st = state.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(6 * 3600));
            if let Ok(db) = st.db.lock() {
                let _ = db.make_backup();
            }
        });
    }
    crate::server_ext::start_background(state.clone());
    // Ağda sunucuyu bulma (UDP)
    std::thread::spawn(move || {
        if let Ok(sock) = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)) {
            let mut buf = [0u8; 64];
            while let Ok((n, from)) = sock.recv_from(&mut buf) {
                if &buf[..n] == b"EMLAK-OFIS?" {
                    let _ = sock.send_to(format!("EMLAK-OFIS!{port}").as_bytes(), from);
                }
            }
        }
    });
    Ok(state)
}

fn header(req: &tiny_http::Request, name: &str) -> Option<String> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str().to_string())
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                if let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn handle_request(st: &State, mut req: tiny_http::Request) {
    let method = req.method().as_str().to_uppercase();
    let url = req.url().to_string();
    let (path, query_s) = match url.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (url.clone(), String::new()),
    };
    let mut query = HashMap::new();
    for kv in query_s.split('&') {
        if let Some((k, v)) = kv.split_once('=') {
            query.insert(url_decode(k), url_decode(v));
        }
    }
    let token = header(&req, "Authorization").map(|v| v.trim_start_matches("Bearer ").trim().to_string());
    let mut body = Vec::new();
    if method == "POST" || method == "PUT" {
        let _ = req.as_reader().take(80 * 1024 * 1024).read_to_end(&mut body);
    }
    let result = route(st, &method, &path, &query, token, body);
    let (status, ctype, data) = match result {
        Ok(Resp::Json(v)) => (200u16, "application/json; charset=utf-8", serde_json::to_vec(&v).unwrap_or_default()),
        Ok(Resp::Bytes(b, ct)) => (200, ct, b),
        Err(ApiErr(code, msg)) => (code, "application/json; charset=utf-8", serde_json::to_vec(&json!({"error": msg})).unwrap_or_default()),
    };
    if status < 300 && method != "GET" && path != "/api/login" && path != "/api/logout" && path != "/api/notifications/read_all" {
        st.rev.fetch_add(1, Ordering::SeqCst);
    }
    let mut resp = tiny_http::Response::new(
        tiny_http::StatusCode(status),
        vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap()],
        Cursor::new(data.clone()),
        Some(data.len()),
        None,
    );
    resp.add_header(tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap());
    let _ = req.respond(resp);
}

fn new_token() -> String {
    use rand::RngCore;
    let mut b = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn auth(st: &State, db: &Db, token: &Option<String>) -> R<UserInfo> {
    let tok = token.as_ref().ok_or_else(ApiErr::unauthorized)?;
    let uid = {
        let mut s = st.sessions.lock().unwrap();
        match s.get_mut(tok) {
            Some((uid, last)) if last.elapsed() < Duration::from_secs(12 * 3600) => {
                *last = Instant::now();
                *uid
            }
            _ => return Err(ApiErr::unauthorized()),
        }
    };
    match db.user_by_id(uid)? {
        Some(u) if u.active => Ok(u),
        _ => Err(ApiErr::unauthorized()),
    }
}

fn need_admin(u: &UserInfo) -> R<()> {
    if u.is_admin() { Ok(()) } else { Err(ApiErr::forbidden()) }
}
fn need_lead(u: &UserInfo) -> R<()> {
    if u.is_lead() { Ok(()) } else { Err(ApiErr::forbidden()) }
}
fn aud(db: &Db, me: &UserInfo, action: &str, detail: &str) {
    crate::server_ext::aud(db, me, action, detail);
}

/// Yüklenen resmi yönlendirmesini düzeltip yeniden boyutlandırır; (tam, küçük) JPEG döner.
fn process_image(bytes: &[u8], max_full: u32, max_thumb: u32) -> R<(Vec<u8>, Vec<u8>)> {
    use image::{DynamicImage, ImageDecoder, ImageReader};
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| ApiErr::bad(e.to_string()))?;
    let mut decoder = reader.into_decoder().map_err(|_| ApiErr::bad("Desteklenmeyen resim biçimi (jpg, png, webp, gif kullanın)."))?;
    let orient = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ApiErr::bad("Resim okunamadı."))?;
    img.apply_orientation(orient);
    let enc = |im: &DynamicImage, q: u8| -> R<Vec<u8>> {
        let rgb = DynamicImage::ImageRgb8(im.to_rgb8());
        let mut out = Vec::new();
        let e = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q);
        rgb.write_with_encoder(e).map_err(|e| ApiErr(500, e.to_string()))?;
        Ok(out)
    };
    let full = if img.width() > max_full || img.height() > max_full {
        img.resize(max_full, max_full, image::imageops::FilterType::Lanczos3)
    } else {
        img.clone()
    };
    let thumb = img.resize(max_thumb, max_thumb, image::imageops::FilterType::Triangle);
    Ok((enc(&full, 85)?, enc(&thumb, 78)?))
}

fn route(st: &State, method: &str, path: &str, q: &HashMap<String, String>, token: Option<String>, body: Vec<u8>) -> R<Resp> {
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    let db_guard = st.db.lock().map_err(|_| ApiErr(500, "Veritabanı kilitlendi".into()))?;
    let db: &Db = &db_guard;

    // ---- Kimlik gerektirmeyen uçlar ----
    match (method, segs.as_slice()) {
        ("GET", ["api", "ping"]) => {
            return Ok(Resp::Json(json!({"app": "emlak-ofis", "version": env!("CARGO_PKG_VERSION"), "needs_setup": db.count_users()? == 0})));
        }
        ("POST", ["api", "setup"]) => {
            if db.count_users()? > 0 {
                return Err(ApiErr::forbidden());
            }
            let v: Value = parse(&body)?;
            let g = |k: &str| v[k].as_str().unwrap_or("").to_string();
            db.create_user(&g("username"), &g("password"), &g("first_name"), &g("last_name"), "admin")?;
            return done();
        }
        ("POST", ["api", "login"]) => {
            let v: Value = parse(&body)?;
            let uname = v["username"].as_str().unwrap_or("");
            let pw = v["password"].as_str().unwrap_or("");
            if let Some((u, hash)) = db.user_login_row(uname)? {
                if u.active && verify_password(pw, &hash) {
                    let tok = new_token();
                    st.sessions.lock().unwrap().insert(tok.clone(), (u.id, Instant::now()));
                    db.audit(u.id, &u.full_name(), "Giriş", "");
                    return ok_json(&LoginReply { token: tok, user: u });
                }
            }
            std::thread::sleep(Duration::from_millis(600));
            return Err(ApiErr(401, "Kullanıcı adı veya şifre hatalı.".into()));
        }
        _ => {}
    }

    let me = auth(st, db, &token)?;

    match (method, segs.as_slice()) {
        ("POST", ["api", "logout"]) => {
            if let Some(t) = &token {
                st.sessions.lock().unwrap().remove(t);
            }
            done()
        }
        ("GET", ["api", "rev"]) => ok_json(&RevReply { rev: st.rev.load(Ordering::SeqCst), unread: db.unread_count(me.id)? }),
        ("GET", ["api", "me"]) => ok_json(&me),
        ("PUT", ["api", "me"]) => {
            let u: UserInfo = parse(&body)?;
            db.update_profile(me.id, &u.first_name, &u.last_name, &u.phone, &u.email)?;
            ok_json(&db.user_by_id(me.id)?)
        }
        ("PUT", ["api", "me", "password"]) => {
            let v: Value = parse(&body)?;
            if !db.check_password(me.id, v["old"].as_str().unwrap_or(""))? {
                return Err(ApiErr::bad("Mevcut şifre hatalı."));
            }
            db.set_password(me.id, v["new"].as_str().unwrap_or(""))?;
            done()
        }
        ("PUT", ["api", "me", "photo"]) => {
            let (full, _) = process_image(&body, 480, 120)?;
            db.set_user_photo(me.id, Some(full))?;
            done()
        }
        ("DELETE", ["api", "me", "photo"]) => {
            db.set_user_photo(me.id, None)?;
            done()
        }
        ("GET", ["api", "users", id, "photo"]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            match db.user_photo(id)? {
                Some(b) => Ok(Resp::Bytes(b, "image/jpeg")),
                None => Err(ApiErr::not_found()),
            }
        }
        ("GET", ["api", "users"]) => ok_json(&db.list_users()?),
        ("POST", ["api", "users"]) => {
            need_admin(&me)?;
            let v: Value = parse(&body)?;
            let g = |k: &str| v[k].as_str().unwrap_or("").to_string();
            let id = db.create_user(&g("username"), &g("password"), &g("first_name"), &g("last_name"), &g("role"))?;
            aud(db, &me, "Kullanıcı eklendi", &format!("{} ({})", g("username"), g("role")));
            Ok(Resp::Json(json!({"id": id})))
        }
        ("PUT", ["api", "users", id]) => {
            need_admin(&me)?;
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let u: UserInfo = parse(&body)?;
            db.admin_update_user(id, &u.first_name, &u.last_name, &u.role, u.active)?;
            aud(db, &me, "Kullanıcı güncellendi", &format!("#{} rol: {} aktif: {}", id, u.role, u.active));
            if !u.active {
                st.sessions.lock().unwrap().retain(|_, (uid, _)| *uid != id);
            }
            done()
        }
        ("POST", ["api", "users", id, "reset_password"]) => {
            need_admin(&me)?;
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let v: Value = parse(&body)?;
            db.set_password(id, v["password"].as_str().unwrap_or(""))?;
            aud(db, &me, "Şifre sıfırlandı", &format!("kullanıcı #{id}"));
            st.sessions.lock().unwrap().retain(|_, (uid, _)| *uid != id);
            done()
        }

        // ---- Konumlar ----
        ("GET", ["api", "locations"]) => ok_json(&db.list_locations()?),
        ("POST", ["api", "locations", "district"]) => {
            need_admin(&me)?;
            let v: Value = parse(&body)?;
            db.add_district(v["name"].as_str().unwrap_or(""))?;
            done()
        }
        ("DELETE", ["api", "locations", "district", id]) => {
            need_admin(&me)?;
            db.delete_district(id.parse().unwrap_or(0))?;
            done()
        }
        ("POST", ["api", "locations", "neighborhood"]) => {
            need_admin(&me)?;
            let v: Value = parse(&body)?;
            db.add_neighborhood(v["district_id"].as_i64().unwrap_or(0), v["name"].as_str().unwrap_or(""))?;
            done()
        }
        ("DELETE", ["api", "locations", "neighborhood", id]) => {
            need_admin(&me)?;
            db.delete_neighborhood(id.parse().unwrap_or(0))?;
            done()
        }
        ("POST", ["api", "locations", "import"]) => {
            need_admin(&me)?;
            let text = String::from_utf8_lossy(&body).to_string();
            let n = db.import_locations(&text)?;
            Ok(Resp::Json(json!({"added": n})))
        }

        // ---- Portföyler ----
        ("GET", ["api", "portfolios"]) => ok_json(&db.list_portfolios(q, &me)?),
        ("POST", ["api", "portfolios"]) => {
            let p: Portfolio = parse(&body)?;
            let id = db.create_portfolio(&me, &p)?;
            crate::server_ext::after_portfolio_save(db, &me, id, None);
            Ok(Resp::Json(json!({"id": id})))
        }
        ("GET", ["api", "portfolios", id]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            ok_json(&db.get_portfolio(id, &me)?)
        }
        ("PUT", ["api", "portfolios", id]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            if !me.is_admin() && db.portfolio_creator(id)? != me.id {
                return Err(ApiErr(403, "Portföyü yalnızca kaydı oluşturan çalışan veya yönetici düzenleyebilir.".into()));
            }
            let p: Portfolio = parse(&body)?;
            let before = crate::server_ext::snapshot(db, id);
            db.update_portfolio(&me, id, &p)?;
            crate::server_ext::after_portfolio_save(db, &me, id, before);
            done()
        }
        ("DELETE", ["api", "portfolios", id]) => {
            need_admin(&me)?;
            let pid: i64 = id.parse().unwrap_or(0);
            let title: String = db.conn.query_row("SELECT title FROM portfolios WHERE id=?1", rusqlite::params![pid], |r| r.get(0)).unwrap_or_default();
            db.delete_portfolio(pid)?;
            aud(db, &me, "Portföy silindi", &format!("P-{:05} {}", pid, title));
            done()
        }
        ("POST", ["api", "portfolios", id, "photos"]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            if !me.is_admin() && db.portfolio_creator(id)? != me.id {
                return Err(ApiErr::forbidden());
            }
            drop(db_guard); // resim işlerken veritabanını kilitli tutma
            let (full, thumb) = process_image(&body, 1600, 320)?;
            let db_guard = st.db.lock().map_err(|_| ApiErr(500, "Veritabanı kilitlendi".into()))?;
            let pid = db_guard.add_photo(id, &full, &thumb)?;
            Ok(Resp::Json(json!({"id": pid})))
        }
        ("GET", ["api", "photos", id]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let thumb = q.get("size").map(|s| s == "thumb").unwrap_or(false);
            Ok(Resp::Bytes(db.photo_data(id, thumb)?, "image/jpeg"))
        }
        ("DELETE", ["api", "photos", id]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let pid = db.photo_portfolio(id)?;
            if !me.is_admin() && db.portfolio_creator(pid)? != me.id {
                return Err(ApiErr::forbidden());
            }
            db.delete_photo(id)?;
            done()
        }
        ("PUT", ["api", "photos", id, "cover"]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let pid = db.photo_portfolio(id)?;
            if !me.is_admin() && db.portfolio_creator(pid)? != me.id {
                return Err(ApiErr::forbidden());
            }
            db.set_cover(id)?;
            done()
        }

        // ---- CRM (yalnızca yönetici) ----
        ("GET", ["api", "crm"]) => {
            need_admin(&me)?;
            ok_json(&db.list_contacts()?)
        }
        ("PUT", ["api", "crm", id]) => {
            need_admin(&me)?;
            let c: CrmContact = parse(&body)?;
            db.update_contact(id.parse().unwrap_or(0), &c.full_name, &c.phone, &c.email, &c.note)?;
            done()
        }
        ("DELETE", ["api", "crm", id]) => {
            need_admin(&me)?;
            db.delete_contact(id.parse().unwrap_or(0))?;
            aud(db, &me, "CRM kaydı silindi", &format!("#{id}"));
            done()
        }

        // ---- Görevler ----
        ("GET", ["api", "tasks"]) => ok_json(&db.list_tasks(&me, q.get("all").map(|v| v == "1").unwrap_or(false))?),
        ("POST", ["api", "tasks"]) => {
            need_lead(&me)?;
            let t: NewTask = parse(&body)?;
            let n = db.create_tasks(&me, &t)?;
            Ok(Resp::Json(json!({"created": n})))
        }
        ("PUT", ["api", "tasks", id, "status"]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            if !me.is_lead() && db.task_assignee(id)? != me.id {
                return Err(ApiErr::forbidden());
            }
            let v: Value = parse(&body)?;
            db.set_task_status(id, v["status"].as_str().unwrap_or(""))?;
            done()
        }
        ("DELETE", ["api", "tasks", id]) => {
            need_lead(&me)?;
            db.delete_task(id.parse().unwrap_or(0))?;
            done()
        }

        // ---- Bildirimler ----
        ("GET", ["api", "notifications"]) => ok_json(&db.list_notifications(me.id)?),
        ("POST", ["api", "notifications", "read_all"]) => {
            db.mark_all_read(me.id)?;
            done()
        }

        // ---- Mesai ----
        ("POST", ["api", "attendance", "in"]) => {
            db.check_in(me.id)?;
            done()
        }
        ("POST", ["api", "attendance", "out"]) => {
            db.check_out(me.id)?;
            done()
        }
        ("GET", ["api", "attendance"]) => {
            let month = q.get("month").cloned().unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
            let uid = if me.is_lead() {
                q.get("user_id").and_then(|s| s.parse::<i64>().ok()).filter(|v| *v > 0)
            } else {
                Some(me.id)
            };
            ok_json(&db.list_attendance(uid, &month)?)
        }
        ("PUT", ["api", "attendance", id]) => {
            need_admin(&me)?;
            let a: AttInfo = parse(&body)?;
            db.update_attendance(id.parse().unwrap_or(0), &a.check_in, &a.check_out, &a.note)?;
            done()
        }
        ("DELETE", ["api", "attendance", id]) => {
            need_admin(&me)?;
            db.delete_attendance(id.parse().unwrap_or(0))?;
            done()
        }

        // ---- Hedefler ----
        ("GET", ["api", "goals"]) => ok_json(&db.list_goals(&me)?),
        ("POST", ["api", "goals"]) => {
            need_lead(&me)?;
            let g: GoalInfo = parse(&body)?;
            db.create_goal(&me, &g)?;
            done()
        }
        ("DELETE", ["api", "goals", id]) => {
            need_lead(&me)?;
            db.delete_goal(id.parse().unwrap_or(0))?;
            done()
        }

        // ---- Çıktı ayarları ----
        ("GET", ["api", "settings"]) => ok_json(&db.get_settings()?),
        ("PUT", ["api", "settings"]) => {
            need_admin(&me)?;
            let s: OutputSettings = parse(&body)?;
            db.set_settings(&s)?;
            aud(db, &me, "Çıktı ayarları değişti", "");
            done()
        }
        ("GET", ["api", "settings", "logo"]) => match db.get_blob("logo")? {
            Some(b) => Ok(Resp::Bytes(b, "image/png")),
            None => Err(ApiErr::not_found()),
        },
        ("PUT", ["api", "settings", "logo"]) => {
            need_admin(&me)?;
            // Logoyu PNG olarak sakla (şeffaflık korunur)
            let img = image::load_from_memory(&body).map_err(|_| ApiErr::bad("Logo okunamadı."))?;
            let img = img.resize(600, 600, image::imageops::FilterType::Lanczos3);
            let mut out = Vec::new();
            img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).map_err(|e| ApiErr(500, e.to_string()))?;
            db.set_blob("logo", Some(&out))?;
            done()
        }
        ("DELETE", ["api", "settings", "logo"]) => {
            need_admin(&me)?;
            db.set_blob("logo", None)?;
            done()
        }

        // ---- Sunucu / yedek ----
        ("GET", ["api", "server_info"]) => {
            need_admin(&me)?;
            let size = std::fs::metadata(&db.path).map(|m| m.len() as f64 / 1048576.0).unwrap_or(0.0);
            ok_json(&ServerInfo {
                data_dir: st.data_dir.to_string_lossy().to_string(),
                db_size_mb: size,
                backups: db.list_backups(),
                addresses: local_ips().into_iter().map(|ip| format!("{}:{}", ip, st.port)).collect(),
                port: st.port,
            })
        }
        ("POST", ["api", "backup"]) => {
            need_admin(&me)?;
            let p = db.make_backup()?;
            aud(db, &me, "Yedek alındı", "");
            Ok(Resp::Json(json!({"path": p.to_string_lossy()})))
        }
        _ => match crate::server_ext::route(st, db, &me, method, segs.as_slice(), q, &body) {
            Some(r) => r,
            None => Err(ApiErr(404, "Bilinmeyen istek.".into())),
        },
    }
}
