//! Web sitesi (Cloudflare Worker: sahibindenfethiye.com.tr) bağlantısı:
//! etiketler, web yayın onayı, ilan/fotoğraf eşitleme, blog yazısı gönderme, yönetici girişi (SSO).
use crate::common::*;
use crate::db::*;
use crate::server::Resp;
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

static LAST: Mutex<String> = Mutex::new(String::new());
static GEN: Mutex<Option<HashMap<i64, u64>>> = Mutex::new(None);
/// "ilan:sıra" -> son gönderilen fotoğraf kimliği (değişmeyenleri tekrar yükleme)
static SENT: Mutex<Option<HashMap<String, i64>>> = Mutex::new(None);
static COUNTS: Mutex<Option<HashMap<i64, i64>>> = Mutex::new(None);

fn set_last(s: String) {
    *LAST.lock().unwrap() = format!("{} — {}", chrono::Local::now().format("%d.%m.%Y %H:%M:%S"), s);
}

pub fn ensure_columns(db: &Db) -> R<()> {
    for (t, c, def) in [
        ("portfolios", "tags", "TEXT NOT NULL DEFAULT ''"),
        ("portfolios", "web_status", "TEXT NOT NULL DEFAULT ''"),
        ("portfolios", "web_at", "TEXT NOT NULL DEFAULT ''"),
        ("portfolios", "web_by", "TEXT NOT NULL DEFAULT ''"),
        ("goals", "t_web", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        let has: bool = {
            let mut st = db.conn.prepare(&format!("PRAGMA table_info({t})"))?;
            let names: Vec<String> = st.query_map([], |r| r.get::<_, String>(1))?.filter_map(|x| x.ok()).collect();
            names.iter().any(|n| n == c)
        };
        if !has {
            db.conn.execute_batch(&format!("ALTER TABLE {t} ADD COLUMN {c} {def};"))?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------ ayarlar / etiketler
pub fn site(db: &Db) -> (String, String) {
    (db.kv_get("web_url").unwrap_or_default().trim().trim_end_matches('/').to_string(), db.kv_get("web_token").unwrap_or_default().trim().to_string())
}

pub fn tag_list(db: &Db) -> Vec<Value> {
    db.kv_get("tag_list")
        .and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok())
        .unwrap_or_else(|| {
            ["Müstakil", "Villa", "Fırsat", "Yeni", "Deniz Manzaralı", "Eşyalı", "Havuzlu", "Yatırımlık", "Takas", "Krediye Uygun"]
                .iter()
                .map(|n| json!({"name": n, "name_en": "", "name_ru": "", "color": "#d4af37"}))
                .collect()
        })
}

pub fn save_tag_list(db: &Db, list: &[Value]) -> R<()> {
    let mut seen = std::collections::HashSet::new();
    let clean: Vec<Value> = list
        .iter()
        .filter_map(|t| {
            let n = t["name"].as_str()?.trim().chars().take(40).collect::<String>();
            if n.is_empty() || !seen.insert(n.to_lowercase()) {
                return None;
            }
            let col = t["color"].as_str().filter(|c| c.len() == 7 && c.starts_with('#')).unwrap_or("#d4af37");
            Some(json!({"name": n, "name_en": t["name_en"].as_str().unwrap_or("").trim(), "name_ru": t["name_ru"].as_str().unwrap_or("").trim(), "color": col}))
        })
        .collect();
    db.kv_set("tag_list", &serde_json::to_string(&clean).unwrap_or_default())
}

/// Portföyün etiketlerini kaydeder; listede olmayan etiketler listeye eklenir.
pub fn set_tags(db: &Db, id: i64, tags: &[String]) {
    let clean: Vec<String> = tags.iter().map(|t| t.trim().chars().take(40).collect::<String>()).filter(|t| !t.is_empty() && !t.contains(';')).collect();
    let _ = db.conn.execute("UPDATE portfolios SET tags=?2 WHERE id=?1", params![id, clean.join(";")]);
    let mut list = tag_list(db);
    let mut changed = false;
    for t in &clean {
        if !list.iter().any(|x| x["name"].as_str().map(|n| n.eq_ignore_ascii_case(t)).unwrap_or(false)) {
            list.push(json!({"name": t, "name_en": "", "name_ru": "", "color": "#d4af37"}));
            changed = true;
        }
    }
    if changed {
        let _ = save_tag_list(db, &list);
        tags_bg(db);
    }
}

pub fn get_tags(db: &Db, id: i64) -> Vec<String> {
    let s: String = db.conn.query_row("SELECT tags FROM portfolios WHERE id=?1", params![id], |r| r.get(0)).unwrap_or_default();
    s.split(';').filter(|x| !x.is_empty()).map(String::from).collect()
}

// ------------------------------------------------------------ HTTP
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(60)).build()
}

fn req_json(method: &str, url: &str, token: &str, body: &Value) -> Result<Value, String> {
    let r = agent().request(method, url).set("Authorization", &format!("Bearer {token}")).send_json(body.clone());
    finish(r)
}
fn req_bytes(method: &str, url: &str, token: &str, ctype: &str, data: &[u8]) -> Result<Value, String> {
    let r = agent().request(method, url).set("Authorization", &format!("Bearer {token}")).set("Content-Type", ctype).send_bytes(data);
    finish(r)
}
fn finish(r: Result<ureq::Response, ureq::Error>) -> Result<Value, String> {
    match r {
        Ok(resp) => Ok(resp.into_json::<Value>().unwrap_or(json!({"ok": true}))),
        Err(ureq::Error::Status(c, resp)) => Err(format!("Site HTTP {c}: {}", resp.into_string().unwrap_or_default().chars().take(200).collect::<String>())),
        Err(e) => Err(format!("Siteye ulaşılamadı: {e}")),
    }
}

// ------------------------------------------------------------ ilan eşitleme
struct Job {
    id: i64,
    meta: Value,
    photos: Vec<(i64, Vec<u8>)>,
}

fn collect(db: &Db, id: i64) -> Option<Job> {
    let rows = db
        .rows(
            &format!("SELECT p.*, {} AS agent_name, (SELECT phone FROM users u WHERE u.id=p.creator_id) AS agent_phone FROM portfolios p WHERE p.id=?1", user_name_sql("p.creator_id")),
            &[&id as &dyn rusqlite::ToSql],
        )
        .ok()?;
    let p = rows.into_iter().next()?;
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
    let b = |k: &str| p[k].as_i64().unwrap_or(0) != 0;
    let mut st = db.conn.prepare("SELECT id,full FROM photos WHERE portfolio_id=?1 ORDER BY position,id").ok()?;
    let photos: Vec<(i64, Vec<u8>)> = st.query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?))).ok()?.filter_map(|x| x.ok()).collect();
    let tags: Vec<String> = s("tags").split(';').filter(|x| !x.is_empty()).map(String::from).collect();
    let meta = json!({
        "id": id, "title": s("title"), "title_en": "", "title_ru": "",
        "listing_type": s("listing_type"), "category": s("category"), "district": s("district"), "neighborhood": s("neighborhood"), "address": s("address"),
        "lat": f("lat"), "lon": f("lon"), "price": f("price"), "currency": s("currency"),
        "gross_m2": f("gross_m2"), "net_m2": f("net_m2"), "rooms": s("rooms"), "building_age": s("building_age"), "floor": s("floor"), "total_floors": s("total_floors"),
        "heating": s("heating"), "deed_status": s("deed_status"), "furnished": b("furnished"), "swap_ok": b("swap_ok"), "credit_ok": b("credit_ok"),
        "desc_tr": s("description"), "desc_en": s("description_en"), "desc_ru": s("description_ru"),
        "tags": tags, "photo_count": photos.len(), "agent_name": s("agent_name"), "agent_phone": s("agent_phone"),
        "prog_status": s("status"), "published_at": s("web_at"),
    });
    Some(Job { id, meta, photos })
}

fn send(job: Job, url: String, token: String) {
    let id = job.id;
    let r: Result<(), String> = (|| {
        for (n, (_pid, data)) in job.photos.iter().enumerate() {
            let key = format!("{id}:{n}");
            let h = i64::from_le_bytes(Sha256::digest(data)[..8].try_into().unwrap());
            let same = SENT.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) == Some(&h);
            if !same {
                req_bytes("PUT", &format!("{url}/api/sync/photo/{id}/{n}"), &token, "image/jpeg", data)?;
                SENT.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, h);
            }
        }
        let old = COUNTS.lock().unwrap().get_or_insert_with(HashMap::new).get(&id).copied().unwrap_or(0);
        for n in job.photos.len() as i64..old {
            let _ = req_bytes("DELETE", &format!("{url}/api/sync/photo/{id}/{n}"), &token, "application/octet-stream", b"");
            SENT.lock().unwrap().get_or_insert_with(HashMap::new).remove(&format!("{id}:{n}"));
        }
        COUNTS.lock().unwrap().get_or_insert_with(HashMap::new).insert(id, job.photos.len() as i64);
        req_json("PUT", &format!("{url}/api/sync/listing/{id}"), &token, &job.meta)?;
        Ok(())
    })();
    match r {
        Ok(()) => set_last(format!("P-{id:05} siteye gönderildi.")),
        Err(e) => set_last(format!("P-{id:05} gönderilemedi: {e}")),
    }
}

/// Portföyü (5 sn içinde başka değişiklik gelmezse) siteye gönderir; yalnızca web'de yayındaki portföyler.
pub fn push_bg(db: &Db, id: i64) {
    let (url, token) = site(db);
    if url.is_empty() || token.is_empty() {
        return;
    }
    let live: String = db.conn.query_row("SELECT web_status FROM portfolios WHERE id=?1", params![id], |r| r.get(0)).unwrap_or_default();
    if live != "Yayında" {
        return;
    }
    let Some(job) = collect(db, id) else { return };
    let gen = {
        let mut g = GEN.lock().unwrap();
        let m = g.get_or_insert_with(HashMap::new);
        let e = m.entry(id).or_insert(0);
        *e += 1;
        *e
    };
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        if GEN.lock().unwrap().as_ref().and_then(|m| m.get(&job.id)).copied() != Some(gen) {
            return; // daha yeni bir değişiklik var
        }
        send(job, url, token);
    });
}

pub fn delete_bg(db: &Db, id: i64) {
    let (url, token) = site(db);
    if url.is_empty() || token.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        match req_bytes("DELETE", &format!("{url}/api/sync/listing/{id}"), &token, "application/octet-stream", b"") {
            Ok(_) => set_last(format!("P-{id:05} siteden kaldırıldı.")),
            Err(e) => set_last(format!("P-{id:05} kaldırılamadı: {e}")),
        }
        SENT.lock().unwrap().get_or_insert_with(HashMap::new).retain(|k, _| !k.starts_with(&format!("{id}:")));
        COUNTS.lock().unwrap().get_or_insert_with(HashMap::new).remove(&id);
    });
}

pub fn tags_bg(db: &Db) {
    let (url, token) = site(db);
    if url.is_empty() || token.is_empty() {
        return;
    }
    let body = json!({"tags": tag_list(db)});
    std::thread::spawn(move || {
        if let Err(e) = req_json("PUT", &format!("{url}/api/sync/tags"), &token, &body) {
            set_last(format!("Etiketler gönderilemedi: {e}"));
        }
    });
}

// ------------------------------------------------------------ SSO
fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut k = if key.len() > 64 { Sha256::digest(key).to_vec() } else { key.to_vec() };
    k.resize(64, 0);
    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    let inner = Sha256::new().chain_update(&ipad).chain_update(msg).finalize();
    Sha256::new().chain_update(&opad).chain_update(inner).finalize().to_vec()
}
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
/// Site yönetim paneline şifresiz, kısa ömürlü (5 dk) giriş bağlantısı
pub fn sso_url(url: &str, token: &str, username: &str, display: &str, now_unix: i64) -> String {
    let exp = now_unix + 300;
    let sig = hex(&hmac_sha256(token.as_bytes(), format!("{username}|{exp}").as_bytes()));
    format!("{url}/admin/sso?u={}&n={}&exp={exp}&sig={sig}", url_encode(username), url_encode(display))
}

// ------------------------------------------------------------ blog metni
pub fn text_to_html(t: &str) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut out = String::new();
    let mut list = false;
    for block in t.replace("\r\n", "\n").split("\n\n") {
        for line in block.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if let Some(x) = l.strip_prefix("- ").or_else(|| l.strip_prefix("• ")) {
                if !list {
                    out.push_str("<ul>");
                    list = true;
                }
                out.push_str(&format!("<li>{}</li>", esc(x)));
                continue;
            }
            if list {
                out.push_str("</ul>");
                list = false;
            }
            if let Some(x) = l.strip_prefix("### ") {
                out.push_str(&format!("<h3>{}</h3>", esc(x)));
            } else if let Some(x) = l.strip_prefix("## ") {
                out.push_str(&format!("<h2>{}</h2>", esc(x)));
            } else {
                out.push_str(&format!("<p>{}</p>", esc(l)));
            }
        }
        if list {
            out.push_str("</ul>");
            list = false;
        }
    }
    out
}

// ------------------------------------------------------------ uçlar
fn vs(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").trim().to_string()
}

pub fn route(db: &Db, me: &UserInfo, method: &str, segs: &[&str], body: &[u8]) -> R<Option<Resp>> {
    let j = |b: &[u8]| -> R<Value> { if b.is_empty() { Ok(json!({})) } else { serde_json::from_slice(b).map_err(|e| ApiErr::bad(format!("Geçersiz istek: {e}"))) } };
    let out = |v: Value| Ok(Some(Resp::Json(v)));
    match (method, segs) {
        ("GET", ["api", "tags"]) => out(Value::Array(tag_list(db))),
        ("PUT", ["api", "tags"]) => {
            if !me.is_lead() {
                return Err(ApiErr::forbidden());
            }
            let v = j(body)?;
            save_tag_list(db, v.as_array().map(|a| a.as_slice()).unwrap_or(&[]))?;
            tags_bg(db);
            out(json!({"ok": true}))
        }
        ("GET", ["api", "web", "settings"]) => {
            if !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            let (url, token) = site(db);
            out(json!({"url": url, "token_set": !token.is_empty(), "last": LAST.lock().unwrap().clone(),
                "live": db.conn.query_row("SELECT COUNT(*) FROM portfolios WHERE web_status='Yayında'", [], |r| r.get::<_, i64>(0)).unwrap_or(0),
                "pending": db.conn.query_row("SELECT COUNT(*) FROM portfolios WHERE web_status='Bekliyor'", [], |r| r.get::<_, i64>(0)).unwrap_or(0)}))
        }
        ("PUT", ["api", "web", "settings"]) => {
            if !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            let v = j(body)?;
            let url = vs(&v, "url");
            if !url.is_empty() && !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(ApiErr::bad("Site adresi https:// ile başlamalı."));
            }
            db.kv_set("web_url", &url)?;
            if !vs(&v, "token").is_empty() {
                db.kv_set("web_token", &vs(&v, "token"))?;
            }
            out(json!({"ok": true}))
        }
        ("POST", ["api", "web", "ping"]) => {
            if !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            let (url, token) = site(db);
            if url.is_empty() || token.is_empty() {
                return Err(ApiErr::bad("Önce site adresini ve anahtarı kaydedin."));
            }
            let r = agent().get(&format!("{url}/api/sync/ping")).set("Authorization", &format!("Bearer {token}")).call();
            match finish(r) {
                Ok(v) => out(v),
                Err(e) => Err(ApiErr::bad(e)),
            }
        }
        ("POST", ["api", "web", "sso"]) => {
            if !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            let (url, token) = site(db);
            if url.is_empty() || token.is_empty() {
                return Err(ApiErr::bad("Önce Ayarlar > Web sitesi bölümüne site adresini ve anahtarı girin."));
            }
            out(json!({"url": sso_url(&url, &token, &me.username, &me.full_name(), chrono::Utc::now().timestamp())}))
        }
        ("POST", ["api", "web", "resync"]) => {
            if !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            let ids: Vec<i64> = db.conn.prepare("SELECT id FROM portfolios WHERE web_status='Yayında'")?.query_map([], |r| r.get(0))?.filter_map(|x| x.ok()).collect();
            tags_bg(db);
            for id in &ids {
                SENT.lock().unwrap().get_or_insert_with(HashMap::new).retain(|k, _| !k.starts_with(&format!("{id}:")));
                push_bg(db, *id);
            }
            out(json!({"queued": ids.len()}))
        }
        ("POST", ["api", "portfolios", id, "web"]) => {
            let id: i64 = id.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))?;
            let v = j(body)?;
            let action = vs(&v, "action");
            let (creator, title, cur): (i64, String, String) = db
                .conn
                .query_row("SELECT creator_id,title,web_status FROM portfolios WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|_| ApiErr::not_found())?;
            let label = format!("P-{id:05} {title}");
            let now = now_str();
            match action.as_str() {
                "request" => {
                    if !me.is_lead() && creator != me.id {
                        return Err(ApiErr::forbidden());
                    }
                    if cur == "Yayında" {
                        return Err(ApiErr::bad("Bu portföy zaten web sitesinde yayında."));
                    }
                    db.conn.execute("UPDATE portfolios SET web_status='Bekliyor', web_by=?2 WHERE id=?1", params![id, me.full_name()])?;
                    for u in db.list_users()?.iter().filter(|u| u.active && u.is_lead() && u.id != me.id) {
                        let _ = db.notify(u.id, "Web yayın isteği", &format!("{} web sitesinde yayınlanmak üzere onayınızı bekliyor: {label}", me.full_name()));
                    }
                    db.audit(me.id, &me.full_name(), "Web yayın isteği", &label);
                }
                "approve" => {
                    if !me.is_lead() {
                        return Err(ApiErr::forbidden());
                    }
                    let st: String = db.conn.query_row("SELECT status FROM portfolios WHERE id=?1", params![id], |r| r.get(0)).unwrap_or_default();
                    if st == "Taslak" {
                        return Err(ApiErr::bad("Taslak portföy web sitesinde yayınlanamaz; önce durumunu 'Yayında' yapın."));
                    }
                    db.conn.execute("UPDATE portfolios SET web_status='Yayında', web_at=?2 WHERE id=?1", params![id, now])?;
                    if creator != me.id {
                        let _ = db.notify(creator, "Web yayını", &format!("{label} web sitesinde yayına alındı."));
                    }
                    db.audit(me.id, &me.full_name(), "Web yayın onayı", &label);
                    push_bg(db, id);
                }
                "reject" => {
                    if !me.is_lead() {
                        return Err(ApiErr::forbidden());
                    }
                    db.conn.execute("UPDATE portfolios SET web_status='' WHERE id=?1", params![id])?;
                    if creator != me.id {
                        let _ = db.notify(creator, "Web yayını", &format!("{label} için web yayın isteği reddedildi."));
                    }
                    db.audit(me.id, &me.full_name(), "Web yayın reddi", &label);
                }
                "unpublish" => {
                    if !me.is_lead() {
                        return Err(ApiErr::forbidden());
                    }
                    db.conn.execute("UPDATE portfolios SET web_status='' WHERE id=?1", params![id])?;
                    delete_bg(db, id);
                    if creator != me.id {
                        let _ = db.notify(creator, "Web yayını", &format!("{label} web sitesinden kaldırıldı."));
                    }
                    db.audit(me.id, &me.full_name(), "Web yayından kaldırma", &label);
                }
                _ => return Err(ApiErr::bad("Bilinmeyen işlem.")),
            }
            out(json!({"ok": true}))
        }
        ("POST", ["api", "web", "post"]) => {
            if !me.is_lead() {
                return Err(ApiErr::forbidden());
            }
            let (url, token) = site(db);
            if url.is_empty() || token.is_empty() {
                return Err(ApiErr::bad("Önce Ayarlar > Web sitesi bölümüne site adresini ve anahtarı girin."));
            }
            let v = j(body)?;
            if vs(&v, "title").is_empty() || vs(&v, "body").is_empty() {
                return Err(ApiErr::bad("Başlık ve metin boş olamaz."));
            }
            let mut image_name = String::new();
            if let Some(b64) = v["image_b64"].as_str().filter(|s| !s.is_empty()) {
                use base64::Engine;
                let data = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| ApiErr::bad("Görsel okunamadı."))?;
                let ext = if data.starts_with(&[0x89, b'P']) { "png" } else { "jpg" };
                image_name = format!("blog-{}.{ext}", chrono::Utc::now().timestamp_millis());
                req_bytes("PUT", &format!("{url}/api/sync/media/{image_name}"), &token, if ext == "png" { "image/png" } else { "image/jpeg" }, &data).map_err(ApiErr::bad)?;
            }
            let post = json!({
                "slug": vs(&v, "slug"), "title": vs(&v, "title"), "category": vs(&v, "category"), "excerpt": vs(&v, "excerpt"),
                "body": text_to_html(&vs(&v, "body")), "image_name": image_name,
                "status": if vs(&v, "status") == "taslak" { "taslak" } else { "yayinda" }, "author": me.full_name(),
            });
            let r = req_json("PUT", &format!("{url}/api/sync/post"), &token, &post).map_err(ApiErr::bad)?;
            db.audit(me.id, &me.full_name(), "Blog yazısı gönderildi", &vs(&v, "title"));
            out(r)
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hmac_known_vector() {
        // RFC 4231 test case 2
        assert_eq!(hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }
    #[test]
    fn html_conversion_escapes() {
        let h = text_to_html("## Başlık\n\nİlk <b>satır</b>\n- bir\n- iki");
        assert!(h.contains("<h2>Başlık</h2>") && h.contains("&lt;b&gt;") && h.contains("<ul><li>bir</li><li>iki</li></ul>"));
    }
    #[test]
    fn sso_link_shape() {
        let u = sso_url("https://x.tr", "tok", "ali", "Ali Veli", 1000);
        assert!(u.starts_with("https://x.tr/admin/sso?u=ali&n=") && u.contains("exp=1300") && u.contains("&sig="));
    }
}
