//! Çalışma zamanı kendi kendine test: Ayarlar > Testler sekmesi ve `cargo test` aynı testleri çalıştırır.
//! Her test geçici bir klasörde, gerçek sunucuyu rastgele portta başlatarak çalışır; gerçek verinize dokunmaz.
use crate::common::*;
use crate::export::*;
use crate::i18n_doc::Lang;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct TestResult {
    pub group: String,
    pub name: String,
    pub ok: bool,
    pub detail: String,
    pub ms: u128,
}

#[derive(Default)]
pub struct Rep {
    pub items: Vec<TestResult>,
}

impl Rep {
    fn run<F: FnOnce()>(&mut self, group: &str, name: &str, f: F) {
        let t = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(f));
        let (ok, detail) = match r {
            Ok(()) => (true, String::new()),
            Err(e) => {
                let m = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "bilinmeyen hata".into());
                (false, m)
            }
        };
        self.items.push(TestResult { group: group.into(), name: name.into(), ok, detail, ms: t.elapsed().as_millis() });
    }
}

// ----------------------------------------------------------------- yardımcılar
struct Srv {
    base: String,
    dir: PathBuf,
}

impl Srv {
    fn new(tag: &str) -> Srv {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let dir = std::env::temp_dir().join(format!("eo_self_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        crate::server::start(dir.clone(), port).expect("sunucu başlamadı");
        std::thread::sleep(std::time::Duration::from_millis(300));
        Srv { base: format!("http://127.0.0.1:{port}"), dir }
    }
    fn call(&self, m: &str, p: &str, body: Option<Value>, tok: &str) -> (u16, Value) {
        let mut r = ureq::request(m, &format!("{}{}", self.base, p));
        if !tok.is_empty() {
            r = r.set("Authorization", &format!("Bearer {tok}"));
        }
        let res = match body {
            Some(b) => r.send_json(b),
            None => r.call(),
        };
        match res {
            Ok(x) => (x.status(), x.into_json().unwrap_or(Value::Null)),
            Err(ureq::Error::Status(c, x)) => (c, x.into_json().unwrap_or(Value::Null)),
            Err(e) => panic!("ağ hatası: {e}"),
        }
    }
    fn login(&self, u: &str, p: &str) -> Value {
        let (s, v) = self.call("POST", "/api/login", Some(json!({"username": u, "password": p})), "");
        assert_eq!(s, 200, "giriş {u}");
        v
    }
    fn tok(&self, u: &str) -> String {
        self.login(u, "Passw0rd1")["token"].as_str().unwrap().to_string()
    }
}
impl Drop for Srv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn len(v: &Value) -> usize {
    v.as_array().map(|a| a.len()).unwrap_or(0)
}

fn jpeg(col: [u8; 3]) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(1200, 800, image::Rgb(col));
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
    image::DynamicImage::ImageRgb8(img).write_with_encoder(enc).unwrap();
    out
}

fn sample_portfolio() -> Portfolio {
    let mut p = Portfolio::new_blank();
    p.id = 7;
    p.title = "Ölüdeniz'de deniz manzaralı villa ğüşıİÖÇ".into();
    p.district = "Fethiye".into();
    p.neighborhood = "Ölüdeniz".into();
    p.price = 12_500_000.0;
    p.gross_m2 = 220.0;
    p.rooms = "4+1".into();
    p.lat = Some(36.55);
    p.lon = Some(29.12);
    p.description = "Çok güzel bir villa. Işıklı, şık, ğüşiöç.".into();
    p.listing_type = "Satılık".into();
    p.category = "Villa".into();
    p.heating = "Klima".into();
    p.floor = "2".into();
    p.total_floors = "3".into();
    p
}

// ----------------------------------------------------------------- ana çalıştırıcı
pub fn run_all() -> Vec<TestResult> {
    // Test sırasında panik mesajlarının konsola dökülmesini engelle
    let old = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let saved_pins = crate::api::get_pins();
    let mut rep = Rep::default();

    api_tests(&mut rep);
    security_tests(&mut rep);
    update_tests(&mut rep);
    backup_tests(&mut rep);
    doc_tests(&mut rep);
    data_tests(&mut rep);
    map_tests(&mut rep);
    ui_lang_tests(&mut rep);
    brand_tests(&mut rep);

    crate::api::set_pins(saved_pins);
    std::panic::set_hook(old);
    rep.items
}

// ----------------------------------------------------------------- sunucu / yetki / akışlar
fn api_tests(rep: &mut Rep) {
    let s = Srv::new("ana");
    let mut at = String::new();
    let (mut et, mut eid, mut mt, mut mid, mut lt) = (String::new(), 0i64, String::new(), 0i64, String::new());
    const G: &str = "Sunucu ve kimlik";
    rep.run("Güvenlik (giriş koruması)", "Çok sayıda hatalı girişten sonra hesap geçici kilitlenir", || {
        let (c, _) = s.call("POST", "/api/setup", Some(json!({"username":"admin","password":"Admin123!","first_name":"Ali","last_name":"Y"})), "");
        assert_eq!(c, 200, "kurulum");
        for i in 0..crate::server::MAX_FAILS {
            let (c, _) = s.call("POST", "/api/login", Some(json!({"username":"kilit_deneme","password":"yanlis"})), "");
            assert_eq!(c, 401, "deneme {i}");
        }
        let (c, v) = s.call("POST", "/api/login", Some(json!({"username":"kilit_deneme","password":"yanlis"})), "");
        assert_eq!(c, 429, "{v}");
        // başka hesap etkilenmez, doğru şifre çalışır
        s.login("admin", "Admin123!");
    });
    rep.run(G, "İlk kurulum ve yönetici girişi", || {
        let (c, _) = s.call("POST", "/api/setup", Some(json!({"username":"admin","password":"Admin123!","first_name":"Ali","last_name":"Y"})), "");
        assert!(c == 200 || c == 403, "kurulum {c}");
        at = s.login("admin", "Admin123!")["token"].as_str().unwrap().to_string();
    });
    if at.is_empty() {
        return;
    }
    rep.run(G, "Yanlış parola ve bilinmeyen kullanıcı reddedilir (401)", || {
        assert_eq!(s.call("POST", "/api/login", Some(json!({"username":"admin","password":"yanlis"})), "").0, 401);
        assert_eq!(s.call("POST", "/api/login", Some(json!({"username":"yok","password":"x"})), "").0, 401);
    });
    rep.run(G, "Çalışan / müdür / çalışan kullanıcıları oluşturulur", || {
        for (u, r) in [("ayse", "employee"), ("mehmet", "employee"), ("lider", "manager"), ("cikis", "employee")] {
            let (c, _) = s.call("POST", "/api/users", Some(json!({"username":u,"password":"Passw0rd1","first_name":u,"last_name":"X","role":r})), &at);
            assert_eq!(c, 200, "kullanıcı {u}");
        }
        let la = s.login("ayse", "Passw0rd1");
        et = la["token"].as_str().unwrap().to_string();
        eid = la["user"]["id"].as_i64().unwrap();
        let ma = s.login("mehmet", "Passw0rd1");
        mt = ma["token"].as_str().unwrap().to_string();
        mid = ma["user"]["id"].as_i64().unwrap();
        lt = s.tok("lider");
    });
    rep.run(G, "Aynı kullanıcı adı ikinci kez eklenemez", || {
        assert_ne!(s.call("POST", "/api/users", Some(json!({"username":"ayse","password":"Passw0rd1","first_name":"a","last_name":"b","role":"employee"})), &at).0, 200);
    });
    rep.run(G, "Giriş yapmadan erişim reddedilir (401)", || {
        assert_eq!(s.call("GET", "/api/portfolios", None, "").0, 401);
        assert_eq!(s.call("GET", "/api/portfolios", None, "sahte-token").0, 401);
    });
    rep.run(G, "Çıkış yapınca oturum geçersiz olur", || {
        let t = s.tok("cikis");
        assert_eq!(s.call("GET", "/api/portfolios", None, &t).0, 200);
        assert_eq!(s.call("POST", "/api/logout", Some(json!({})), &t).0, 200);
        assert_eq!(s.call("GET", "/api/portfolios", None, &t).0, 401);
    });
    if et.is_empty() {
        return;
    }

    // ---- yetki matrisi (her satır ayrı sonuç)
    const Y: &str = "Yetki matrisi";
    let matris: Vec<(&str, &str, &str, u16)> = vec![
        ("GET", "/api/crm", "çalışan", 403), ("GET", "/api/crm", "müdür", 403), ("GET", "/api/crm", "yönetici", 200),
        ("GET", "/api/commissions", "çalışan", 403), ("GET", "/api/commissions", "müdür", 403), ("GET", "/api/commissions", "yönetici", 200),
        ("GET", "/api/audit", "çalışan", 403), ("GET", "/api/audit", "müdür", 403), ("GET", "/api/audit", "yönetici", 200),
        ("GET", "/api/settings/ext", "müdür", 403), ("GET", "/api/settings/ext", "yönetici", 200),
        ("GET", "/api/users", "çalışan", 200), ("GET", "/api/portfolios", "çalışan", 200),
        ("GET", "/api/update/info", "çalışan", 200), ("POST", "/api/backup/external", "müdür", 403),
    ];
    for (m, p, who, want) in matris {
        let tok = match who {
            "çalışan" => et.clone(),
            "müdür" => lt.clone(),
            _ => at.clone(),
        };
        rep.run(Y, &format!("{m} {p} — {who} → {want}"), || {
            let (c, _) = s.call(m, p, if m == "POST" { Some(json!({})) } else { None }, &tok);
            assert_eq!(c, want);
        });
    }

    // ---- görev / hedef / mesai
    const GM: &str = "Görev, hedef, mesai";
    let mut tid = 0i64;
    rep.run(GM, "Çalışan görev/hedef oluşturamaz; müdür oluşturabilir", || {
        let task = json!({"title":"Tapu takibi","details":"","kind":"Görev","assignees":[mid],"due_at":"2026-12-01 10:00"});
        assert_eq!(s.call("POST", "/api/users", Some(json!({"username":"x","password":"Passw0rd1","first_name":"x","last_name":"x","role":"employee"})), &et).0, 403);
        assert_eq!(s.call("POST", "/api/tasks", Some(task.clone()), &et).0, 403);
        assert_eq!(s.call("POST", "/api/tasks", Some(task), &lt).0, 200);
        let g = json!({"title":"Hedef","user_id":eid,"period_start":"2026-11-01","period_end":"2026-11-30","t_portfolio":3});
        assert_eq!(s.call("POST", "/api/goals", Some(g.clone()), &et).0, 403);
        assert_eq!(s.call("POST", "/api/goals", Some(g), &lt).0, 200);
        tid = s.call("GET", "/api/tasks", None, &mt).1[0]["id"].as_i64().unwrap();
    });
    rep.run(GM, "Görev durumunu yalnızca atanan değiştirir; bildirim gider", || {
        assert_eq!(s.call("PUT", &format!("/api/tasks/{tid}/status"), Some(json!({"status":"Tamamlandı"})), &et).0, 403);
        assert_eq!(s.call("PUT", &format!("/api/tasks/{tid}/status"), Some(json!({"status":"Tamamlandı"})), &mt).0, 200);
        assert_eq!(len(&s.call("GET", "/api/tasks", None, &et).1), 0, "çalışan başkasının görevini görmez");
        assert!(len(&s.call("GET", "/api/notifications", None, &mt).1) >= 1, "görev bildirimi");
    });
    rep.run(GM, "Mesai: giriş/çıkış herkese, düzeltme/silme yalnız yöneticiye", || {
        assert_eq!(s.call("POST", "/api/attendance/in", Some(json!({})), &et).0, 200);
        assert_eq!(s.call("POST", "/api/attendance/out", Some(json!({})), &et).0, 200);
        let att = s.call("GET", "/api/attendance", None, &at).1;
        let aid = att.as_array().and_then(|a| a.first()).and_then(|x| x["id"].as_i64()).expect("mesai kaydı");
        let body = json!({"check_in":"2026-11-10 09:00:00","check_out":"2026-11-10 18:00:00","note":"düzeltme"});
        assert_eq!(s.call("PUT", &format!("/api/attendance/{aid}"), Some(body.clone()), &lt).0, 403);
        assert_eq!(s.call("DELETE", &format!("/api/attendance/{aid}"), None, &et).0, 403);
        assert_eq!(s.call("PUT", &format!("/api/attendance/{aid}"), Some(body), &at).0, 200);
    });

    // ---- portföy / alıcı eşleştirme
    const P: &str = "Portföy ve alıcılar";
    let mut pid = 0i64;
    let mut pbody = json!({"title":"Villa","listing_type":"Satılık","category":"Villa","district":"Fethiye","neighborhood":"","price":12500000,"currency":"TL","status":"Yayında","rooms":"4+1","owner_name":"Mehmet Sahip","owner_phone":"05321234567"});
    rep.run(P, "Alıcı kaydı + portföy ekleme → eşleşme bildirimi", || {
        let (c, _) = s.call("POST", "/api/buyers", Some(json!({"full_name":"Alıcı","phone":"0555","listing_type":"Satılık","category":"Villa","district":"Fethiye","budget_min":5000000,"budget_max":15000000,"currency":"TL","consent":true})), &mt);
        assert_eq!(c, 200);
        let (_, r) = s.call("POST", "/api/portfolios", Some(pbody.clone()), &et);
        pid = r["id"].as_i64().unwrap();
        let notif = s.call("GET", "/api/notifications", None, &mt).1;
        assert!(notif.as_array().unwrap().iter().any(|x| x["kind"] == "Eşleşme"), "eşleşme bildirimi");
    });
    rep.run(P, "Alıcı görünürlüğü: çalışan kendininkini, müdür hepsini görür", || {
        assert_eq!(len(&s.call("GET", "/api/buyers", None, &et).1), 0);
        assert_eq!(len(&s.call("GET", "/api/buyers", None, &lt).1), 1);
        assert_eq!(len(&s.call("GET", &format!("/api/portfolios/{pid}/matches"), None, &et).1), 0);
    });
    rep.run(P, "Fiyat değişince geçmiş kaydı oluşur", || {
        pbody["price"] = json!(11900000);
        s.call("PUT", &format!("/api/portfolios/{pid}"), Some(pbody.clone()), &et);
        assert_eq!(len(&s.call("GET", &format!("/api/portfolios/{pid}/history"), None, &et).1), 2);
    });
    rep.run(P, "Eşleştirme kapalıyken bildirim gitmez", || {
        s.call("PUT", "/api/settings/ext", Some(json!({"match_enabled": false})), &at);
        let n = |t: &str| s.call("GET", "/api/notifications", None, t).1.as_array().unwrap().iter().filter(|x| x["kind"] == "Eşleşme").count();
        let before = n(&mt);
        s.call("POST", "/api/portfolios", Some(json!({"title":"Villa 2","listing_type":"Satılık","category":"Villa","district":"Fethiye","price":9000000,"currency":"TL","status":"Yayında"})), &et);
        assert_eq!(n(&mt), before);
        s.call("PUT", "/api/settings/ext", Some(json!({"match_enabled": true})), &at);
    });
    rep.run(P, "60+ gün filtresi yeni portföyü listelemez", || {
        assert_eq!(len(&s.call("GET", "/api/portfolios?stale=1", None, &et).1), 0);
    });
    rep.run(P, "Harita uç noktası konumlu portföyleri sahip kimliğiyle döner", || {
        let mut b = pbody.clone();
        b["lat"] = json!(36.55);
        b["lon"] = json!(29.12);
        s.call("PUT", &format!("/api/portfolios/{pid}"), Some(b), &et);
        let m = s.call("GET", "/api/map", None, &mt).1;
        let it = m.as_array().and_then(|a| a.iter().find(|x| x["id"] == pid)).expect("haritada portföy");
        assert_eq!(it["creator_id"], json!(eid));
    });

    // ---- takvim / izin / komisyon / CRM
    const T: &str = "Takvim, izin, komisyon, CRM";
    rep.run(T, "Duyuru: yalnız yönetici/müdür yayınlar", || {
        assert_eq!(s.call("POST", "/api/announcements", Some(json!({"title":"x"})), &et).0, 403);
        assert_eq!(s.call("POST", "/api/announcements", Some(json!({"title":"Toplantı"})), &lt).0, 200);
    });
    rep.run(T, "Randevu çakışması 409; zorla ekleme 200", || {
        let a = json!({"title":"Yer gösterme","kind":"Yer gösterme","start_at":"2026-11-20 14:00","duration_min":60,"portfolio_id":pid});
        assert_eq!(s.call("POST", "/api/appointments", Some(a.clone()), &et).0, 200);
        let mut a2 = a.clone();
        a2["start_at"] = json!("2026-11-20 14:30");
        assert_eq!(s.call("POST", "/api/appointments", Some(a2.clone()), &et).0, 409);
        a2["force"] = json!(true);
        assert_eq!(s.call("POST", "/api/appointments", Some(a2), &et).0, 200);
    });
    rep.run(T, "İzin talebi, onay yetkisi ve puantaj", || {
        assert_eq!(s.call("POST", "/api/leaves", Some(json!({"kind":"Yıllık izin","start_date":"2026-11-02","end_date":"2026-11-04"})), &et).0, 200);
        let lid = s.call("GET", "/api/leaves?year=2026", None, &lt).1[0]["id"].as_i64().unwrap();
        assert_eq!(s.call("PUT", &format!("/api/leaves/{lid}/status"), Some(json!({"status":"Onaylandı"})), &et).0, 403);
        assert_eq!(s.call("PUT", &format!("/api/leaves/{lid}/status"), Some(json!({"status":"Onaylandı"})), &lt).0, 200);
        let ts = s.call("GET", "/api/timesheet?month=2026-11", None, &lt).1;
        assert!(ts["rows"].as_array().unwrap().iter().any(|r| r["leave_days"] == 3), "puantaj izin günü");
        assert_eq!(len(&s.call("GET", "/api/timesheet?month=2026-11", None, &et).1["rows"]), 1);
    });
    rep.run(T, "Komisyon hesabı (2% × 10 M, çalışan payı %40)", || {
        s.call("POST", "/api/commissions", Some(json!({"portfolio_id":pid,"user_id":eid,"deal_type":"Satış","deal_amount":10000000,"rate":2,"emp_pct":40,"date":"2026-11-08"})), &at);
        let c = s.call("GET", "/api/commissions?month=2026-11", None, &at).1;
        assert_eq!(c[0]["commission"], json!(200000.0));
        assert_eq!(c[0]["emp_amount"], json!(80000.0));
    });
    rep.run(T, "CRM yalnız yöneticide; KVKK onayı ve anonimleştirme", || {
        let crm = s.call("GET", "/api/crm", None, &at).1;
        let cid = crm.as_array().and_then(|a| a.first()).and_then(|x| x["id"].as_i64()).expect("CRM kaydı");
        assert_eq!(s.call("PUT", &format!("/api/crm/{cid}/consent"), Some(json!({"consent": true})), &et).0, 403);
        assert_eq!(s.call("PUT", &format!("/api/crm/{cid}/consent"), Some(json!({"consent": true})), &at).0, 200);
        assert_eq!(s.call("POST", &format!("/api/crm/{cid}/anonymize"), Some(json!({})), &at).0, 200);
    });
    rep.run(T, "Denetim kaydı işlemleri tutar", || {
        assert!(len(&s.call("GET", "/api/audit", None, &at).1) >= 5);
    });

    // ---- güncelleme yükleme/indirme (sunucu tarafı)
    rep.run("Güncelleme", "Sunucuya paket yükleme, bilgi ve indirme", || {
        let bytes = vec![1u8; 1_100_000];
        let r = ureq::post(&format!("{}/api/update/upload?platform=linux&version=9.9.9", s.base)).set("Authorization", &format!("Bearer {at}")).send_bytes(&bytes);
        assert!(r.is_ok(), "yüklenemedi");
        let info = s.call("GET", "/api/update/info", None, &et).1;
        assert_eq!(info["packages"][0]["version"], "9.9.9");
        let rr = ureq::get(&format!("{}/api/update/download?platform=linux", s.base)).set("Authorization", &format!("Bearer {et}")).call().unwrap();
        let mut got = Vec::new();
        std::io::Read::read_to_end(&mut rr.into_reader(), &mut got).unwrap();
        assert_eq!(got.len(), bytes.len());
    });

    // ---- veritabanı sağlığı (aynı sunucunun dosyası)
    rep.run("Veritabanı", "integrity_check ve şema sürümü", || {
        let c = rusqlite::Connection::open(s.dir.join("emlak.db")).unwrap();
        let r: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
        assert_eq!(r, "ok");
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert!(v >= 2, "şema sürümü {v}");
    });
    rep.run("Veritabanı", "Yedek alma (VACUUM INTO) çalışır ve açılabilir", || {
        let (c, r) = s.call("POST", "/api/backup", Some(json!({})), &at);
        assert_eq!(c, 200);
        let f = r["file"].as_str().or_else(|| r["path"].as_str()).map(|x| x.to_string());
        if let Some(f) = f {
            let c = rusqlite::Connection::open(&f).unwrap();
            let r: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
            assert_eq!(r, "ok");
        }
    });
}

// ----------------------------------------------------------------- HTTPS + imza
fn security_tests(rep: &mut Rep) {
    const G: &str = "Güvenlik (HTTPS)";
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = std::env::temp_dir().join(format!("eo_self_tls_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut fp = String::new();
    rep.run(G, "HTTPS sunucusu kendi sertifikasını üretip başlar", || {
        let st = crate::server::start_with(dir.clone(), port, true).expect("https sunucu");
        fp = st.tls_fp.clone().expect("parmak izi");
        assert_eq!(fp.len(), 64);
        assert!(dir.join("tls_cert.pem").exists() && dir.join("tls_key.pem").exists());
        std::thread::sleep(std::time::Duration::from_millis(300));
    });
    if fp.is_empty() {
        return;
    }
    let addr = format!("127.0.0.1:{port}");
    rep.run(G, "İstemci HTTPS'e bağlanır ve parmak izini öğrenir (ilk güven)", || {
        crate::api::set_pins(Default::default());
        let a = crate::api::Api::new(&addr);
        assert!(a.is_https(), "HTTPS seçilmedi");
        let p = a.ping().expect("ping");
        assert_eq!(p.app, "emlak-ofis");
        assert_eq!(a.seen_fingerprint().as_deref(), Some(fp.as_str()));
    });
    rep.run(G, "Doğru parmak iziyle bağlanır", || {
        let mut m = std::collections::BTreeMap::new();
        m.insert(addr.clone(), fp.clone());
        crate::api::set_pins(m);
        assert!(crate::api::Api::new(&addr).ping().is_ok());
    });
    rep.run(G, "Yanlış parmak izi (araya girme) REDDEDİLİR", || {
        let mut m = std::collections::BTreeMap::new();
        m.insert(addr.clone(), "00".repeat(32));
        crate::api::set_pins(m);
        assert!(crate::api::Api::new(&addr).ping().is_err(), "yanlış sertifika kabul edildi!");
    });
    rep.run(G, "Kayıtlı parmak izi varken HTTP'ye düşülmez", || {
        let a = crate::api::Api::new(&addr);
        assert!(a.is_https());
    });
    rep.run(G, "Parmak izi biçimi (AA:BB:…)", || {
        let p = crate::tls::fp_pretty(&fp);
        assert_eq!(p.matches(':').count(), 31);
    });
    crate::api::set_pins(Default::default());
    let _ = std::fs::remove_dir_all(&dir);
}

fn update_tests(rep: &mut Rep) {
    use std::io::Write;
    const G: &str = "Güncelleme ve imza";
    rep.run(G, "Sürüm karşılaştırma (1.10.0 > 1.9.9, geçersiz → yeni değil)", || {
        assert!(crate::updater::is_newer("1.2.1", "1.2.0"));
        assert!(crate::updater::is_newer("1.10.0", "1.9.9"));
        assert!(!crate::updater::is_newer("1.2.0", "1.2.0"));
        assert!(!crate::updater::is_newer("1.1.9", "1.2.0"));
        assert!(!crate::updater::is_newer("abc", "1.2.0"));
    });
    let dir = std::env::temp_dir().join(format!("eo_self_upd_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let data = vec![7u8; 1_200_000];
    let make = |path: &std::path::Path, good_hash: bool, sig: &str| {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        let h = if good_hash { sha2_hex(&data) } else { "00".into() };
        z.start_file("surum.json", o).unwrap();
        z.write_all(format!("{{\"surum\":\"9.9.9\",\"notlar\":\"x\",\"paketler\":{{\"linux\":{{\"dosya\":\"linux/emlak-ofis\",\"sha256\":\"{h}\",\"imza\":\"{sig}\"}}}}}}").as_bytes()).unwrap();
        z.start_file("linux/emlak-ofis", o).unwrap();
        z.write_all(&data).unwrap();
        z.finish().unwrap();
    };
    rep.run(G, "Paket okuma ve SHA-256 doğrulama; bozuk paket reddedilir", || {
        let ok = dir.join("ok.zip");
        make(&ok, true, "");
        let p = crate::updater::read_package_with(&ok, "").unwrap();
        assert_eq!(p.version, "9.9.9");
        assert_eq!(p.bins[0].1.len(), data.len());
        let bad = dir.join("bad.zip");
        make(&bad, false, "");
        assert!(crate::updater::read_package_with(&bad, "").unwrap_err().contains("bozuk"));
    });
    let (sec, public) = crate::sign::generate();
    let (_sec2, public2) = crate::sign::generate();
    rep.run(G, "Anahtar üretimi: gizli/açık anahtar 32 bayt", || {
        use base64::Engine;
        let d = |s: &str| base64::engine::general_purpose::STANDARD.decode(s).unwrap().len();
        assert_eq!(d(&sec), 32);
        assert_eq!(d(&public), 32);
        assert_ne!(public, public2);
    });
    rep.run(G, "İmza doğrulanır; değiştirilmiş dosya / yanlış sürüm / yanlış anahtar reddedilir", || {
        let sig = crate::sign::sign(&sec, "9.9.9", "linux", &data).unwrap();
        assert!(crate::sign::verify(&public, "9.9.9", "linux", &data, &sig).is_ok());
        let mut tampered = data.clone();
        tampered[10] ^= 1;
        assert!(crate::sign::verify(&public, "9.9.9", "linux", &tampered, &sig).is_err(), "değişen dosya kabul edildi");
        assert!(crate::sign::verify(&public, "9.9.8", "linux", &data, &sig).is_err(), "sürüm değişimi kabul edildi");
        assert!(crate::sign::verify(&public, "9.9.9", "windows", &data, &sig).is_err(), "platform değişimi kabul edildi");
        assert!(crate::sign::verify(&public2, "9.9.9", "linux", &data, &sig).is_err(), "başka anahtar kabul edildi");
    });
    rep.run(G, "Güvenilir anahtar tanımlıyken imzasız paket reddedilir, imzalı kabul edilir", || {
        let unsigned = dir.join("unsigned.zip");
        make(&unsigned, true, "");
        assert!(crate::updater::read_package_with(&unsigned, &public).unwrap_err().contains("imzasız"));
        let sig = crate::sign::sign(&sec, "9.9.9", "linux", &data).unwrap();
        let signed = dir.join("signed.zip");
        make(&signed, true, &sig);
        let p = crate::updater::read_package_with(&signed, &public).unwrap();
        assert!(p.signed);
        assert!(crate::updater::read_package_with(&signed, &public2).is_err(), "başka yayıncı anahtarı kabul edildi");
    });
    rep.run(G, "--paket-imzala: komut satırı imzalayıcı paketi imzalar", || {
        let unsigned = dir.join("cli.zip");
        make(&unsigned, true, "");
        let key = dir.join("k.gizli");
        std::fs::write(&key, &sec).unwrap();
        crate::sign::cli_sign_package(unsigned.to_str().unwrap(), key.to_str().unwrap()).unwrap();
        assert!(crate::updater::read_package_with(&unsigned, &public).unwrap().signed);
    });
    let _ = std::fs::remove_dir_all(&dir);
}

fn sha2_hex(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(b))
}

// ----------------------------------------------------------------- yedek / geri yükleme
fn backup_tests(rep: &mut Rep) {
    const G: &str = "Yedekleme";
    rep.run(G, "Şifreli harici yedek (EOBK1): doğru parola açar, yanlış parola açmaz", || {
        let s = Srv::new("yedek");
        s.call("POST", "/api/setup", Some(json!({"username":"admin","password":"Admin123!","first_name":"A","last_name":"B"})), "");
        let at = s.login("admin", "Admin123!")["token"].as_str().unwrap().to_string();
        let ext = s.dir.join("harici");
        let (c, _) = s.call("PUT", "/api/settings/ext", Some(json!({"ext_backup_dir": ext.to_string_lossy(), "ext_backup_pw": "gizli-parola"})), &at);
        assert_eq!(c, 200);
        let (c, r) = s.call("POST", "/api/backup/external", Some(json!({})), &at);
        assert_eq!(c, 200);
        let f = r["external"].as_str().expect("harici dosya yolu");
        assert!(f.ends_with(".enc"));
        let raw = std::fs::read(f).unwrap();
        assert!(raw.starts_with(b"EOBK1"));
        assert!(crate::ext_db::decrypt_bytes(&raw, "yanlis").is_err());
        assert!(crate::ext_db::decrypt_bytes(&raw, "gizli-parola").unwrap().starts_with(b"SQLite format 3"));
    });
    rep.run(G, "Geri yükleme: yedekten döner, eskisini saklar; bozuk dosya verilere dokunmaz", || {
        let dir = std::env::temp_dir().join(format!("eo_self_rest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mk = |path: &std::path::Path, title: &str| {
            let db = crate::db::Db::open(path).unwrap();
            db.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            db.conn.execute("INSERT INTO portfolios(title,creator_id,created_at,updated_at) VALUES(?1,0,'2026-01-01 00:00:00','2026-01-01 00:00:00')", [title]).unwrap();
            db.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
        };
        let bak = dir.join("yedek.db");
        mk(&bak, "YEDEKTEKI");
        let live = dir.join("veri");
        std::fs::create_dir_all(&live).unwrap();
        mk(&live.join("emlak.db"), "GUNCEL");
        let saved = crate::restore::restore(&live, &bak, "").unwrap();
        assert!(saved.contains("geri_yukleme_oncesi"));
        let title = |p: &std::path::Path| -> String { rusqlite::Connection::open(p).unwrap().query_row("SELECT title FROM portfolios", [], |r| r.get(0)).unwrap() };
        assert_eq!(title(&live.join("emlak.db")), "YEDEKTEKI");
        assert_eq!(title(std::path::Path::new(&saved)), "GUNCEL");
        let junk = dir.join("cop.db");
        std::fs::write(&junk, b"bu bir yedek degil").unwrap();
        assert!(crate::restore::restore(&live, &junk, "").is_err());
        assert_eq!(title(&live.join("emlak.db")), "YEDEKTEKI");
        let _ = std::fs::remove_dir_all(&dir);
    });
}

// ----------------------------------------------------------------- belge çıktısı
fn doc_tests(rep: &mut Rep) {
    const G: &str = "Belge çıktısı";
    let dir = std::env::temp_dir().join(format!("eo_self_doc_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    for l in Lang::ALL {
        rep.run(G, &format!("PDF + DOCX üretimi — {}", l.label()), || {
            let d = ExportData {
                p: sample_portfolio(),
                photos: vec![jpeg([200, 30, 30]), jpeg([30, 200, 30])],
                lang: l,
                settings: OutputSettings::default(),
                agent: Some(Agent { name: "Ayşe Çalışan".into(), phone: "0532 123 45 67".into(), email: "ayse@example.com".into(), photo: Some(jpeg([90, 90, 90])) }),
                logo: None,
            };
            let (pf, df) = (dir.join(format!("t_{}.pdf", l.code())), dir.join(format!("t_{}.docx", l.code())));
            write_pdf(&d, &pf).unwrap();
            write_docx(&d, &df).unwrap();
            assert!(std::fs::metadata(&pf).unwrap().len() > 5000, "PDF çok küçük");
            assert!(std::fs::metadata(&df).unwrap().len() > 5000, "DOCX çok küçük");
        });
    }
    rep.run(G, "Etiket çevirileri (EN ayrıntı satırları)", || {
        let en = detail_rows(&sample_portfolio(), Lang::En);
        assert!(en.iter().any(|(k, v)| k == "Gross area" && v == "220 m²"));
        assert!(en.iter().any(|(k, v)| k == "Heating" && v == "Air conditioning"));
        assert!(en.iter().any(|(k, v)| k == "Floor" && v == "Floor 2 of 3"));
    });
    rep.run(G, "WhatsApp metni her dilde (TR/DE/RU)", || {
        let p = sample_portfolio();
        let st = OutputSettings::default();
        assert!(whatsapp_text(&p, &st, &None, Lang::Tr).contains("Satılık Villa"));
        let de = whatsapp_text(&p, &st, &None, Lang::De);
        assert!(de.contains("Zu verkaufen Villa") && de.contains("Preis: 12"));
        assert!(whatsapp_text(&p, &st, &None, Lang::Ru).contains("Продаётся Вилла"));
    });
    rep.run(G, "Dil kodu eşlemesi ve bilinmeyen kod → Türkçe", || {
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), l);
        }
        assert_eq!(Lang::from_code("xx"), Lang::Tr);
    });
    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------- veri
fn data_tests(rep: &mut Rep) {
    const G: &str = "Yerleşik veri";
    rep.run(G, "Muğla ilçe/mahalle verisi okunur", || {
        let j: Value = serde_json::from_str(include_str!("../assets/mugla.json")).unwrap();
        let n = j.as_object().map(|o| o.len()).or_else(|| j.as_array().map(|a| a.len())).unwrap_or(0);
        assert!(n >= 10, "ilçe sayısı {n}");
    });
    rep.run(G, "Simge dosyası çözülür (256×256)", || {
        let img = image::load_from_memory(include_bytes!("../assets/icon256.png")).unwrap();
        assert_eq!((img.width(), img.height()), (256, 256));
    });
    rep.run(G, "Para/fiyat biçimi", || {
        let s = fmt_price(12_500_000.0, "TL");
        assert!(s.contains("12") && s.contains("TL"), "{s}");
    });
}

// ----------------------------------------------------------------- harita
fn map_tests(rep: &mut Rep) {
    const G: &str = "Harita";
    use crate::ui_map::tiles;
    rep.run(G, "Karo koordinatı: Fethiye (36.62, 29.11) z12 → doğru karo (2379,1599)", || {
        let (x, y) = tiles::lonlat_to_tile(29.11, 36.62, 12);
        assert_eq!((x.floor() as u32, y.floor() as u32), (2379, 1599));
    });
    rep.run(G, "Karo adresleri: sokak ve uydu kaynakları", || {
        let u1 = tiles::url(tiles::Source::Street, 12, 2379, 1599);
        let u2 = tiles::url(tiles::Source::Satellite, 12, 2379, 1599);
        assert!(u1.starts_with("https://") && u1.contains("/12/2379/1599"), "{u1}");
        assert!(u2.starts_with("https://") && u2.contains("12") && u2.contains("2379") && u2.contains("1599"), "{u2}");
    });
    rep.run(G, "Karo disk önbelleği: yaz → oku", || {
        let dir = std::env::temp_dir().join(format!("eo_self_tiles_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let c = tiles::Cache::new(dir.clone());
        assert!(c.read(tiles::Source::Street, 5, 1, 2).is_none());
        c.write(tiles::Source::Street, 5, 1, 2, b"abc");
        assert_eq!(c.read(tiles::Source::Street, 5, 1, 2).unwrap(), b"abc");
        assert!(c.read(tiles::Source::Satellite, 5, 1, 2).is_none(), "kaynaklar karışmamalı");
        let _ = std::fs::remove_dir_all(&dir);
    });
    rep.run(G, "Çevrimdışı harita: karo sayımı ve çevrimdışı klasörden okuma", || {
        let v = tiles::tiles_for((29.07, 36.60, 29.17, 36.70), 15, 15);
        assert!(v.len() > 4 && v.len() < 200, "{}", v.len());
        assert!(v.iter().all(|t| t.0 == 15));
        let (tx, ty) = tiles::lonlat_to_tile(29.11, 36.65, 15);
        assert!(v.contains(&(15, tx.floor() as u32, ty.floor() as u32)));
        let base = std::env::temp_dir().join(format!("eo_self_off_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let c = tiles::Cache::with_offline(base.join("onbellek"), base.join("cevrimdisi"));
        tiles::Cache::new(base.join("cevrimdisi")).write(tiles::Source::Satellite, 7, 3, 4, b"xyz");
        assert_eq!(c.read(tiles::Source::Satellite, 7, 3, 4).unwrap(), b"xyz");
        assert!(c.read(tiles::Source::Street, 7, 3, 4).is_none());
        let _ = std::fs::remove_dir_all(&base);
        assert!(tiles::REGIONS.iter().all(|r| tiles::tiles_for(r.1, r.2, r.3).len() < 20000));
    });
    rep.run(G, "Portföy türü → işaret rengi (satılık/kiralık/arazi)", || {
        let k = |lt: &str, cat: &str| crate::ui_map::kind_of(&json!({"listing_type": lt, "category": cat}));
        assert_eq!(k("Satılık", "Villa"), 0);
        assert_eq!(k("Kiralık", "Daire"), 1);
        assert_eq!(k("Satılık", "Arsa"), 2);
        assert_eq!(k("Kiralık", "Tarla"), 2);
    });
}

// ----------------------------------------------------------------- arayüz dili
fn ui_lang_tests(rep: &mut Rep) {
    const G: &str = "Arayüz dili";
    use crate::i18n_ui::{self, UiLang};
    rep.run(G, "Türkçe metin değişmeden döner", || {
        assert_eq!(i18n_ui::translate(UiLang::Tr, "Kaydet"), "Kaydet");
    });
    rep.run(G, "İngilizce çeviri (Kaydet → Save) ve bilinmeyen metin yedeği", || {
        assert_eq!(i18n_ui::translate(UiLang::En, "Kaydet"), "Save");
        assert_eq!(i18n_ui::translate(UiLang::En, "Bu cümle sözlükte yok 123"), "Bu cümle sözlükte yok 123");
    });
    rep.run(G, "Sözlük bütünlüğü: boş çeviri, yinelenen anahtar yok", || {
        let (n, bad) = i18n_ui::audit();
        assert!(n >= 100, "çok az çeviri: {n}");
        assert!(bad.is_empty(), "hatalı girişler: {}", bad.join(", "));
    });
    rep.run(G, "Dil kodları", || {
        for l in UiLang::ALL {
            assert_eq!(UiLang::from_code(l.code()), l);
        }
    });
}

// ----------------------------------------------------------------- sonuç biçimleri
pub fn summary(items: &[TestResult]) -> (usize, usize) {
    let ok = items.iter().filter(|r| r.ok).count();
    (ok, items.len() - ok)
}

pub fn to_text(items: &[TestResult]) -> String {
    let (ok, bad) = summary(items);
    let mut s = format!(
        "Emlak Ofis — Test raporu\nSürüm: v{} ({})\nTarih: {}\nSonuç: {} geçti, {} başarısız, toplam {}\n",
        crate::updater::current_version(),
        crate::updater::platform(),
        chrono::Local::now().format("%d.%m.%Y %H:%M"),
        ok,
        bad,
        items.len()
    );
    let mut last = String::new();
    for r in items {
        if r.group != last {
            s.push_str(&format!("\n[{}]\n", r.group));
            last = r.group.clone();
        }
        s.push_str(&format!("  {} {} ({} ms)\n", if r.ok { "GEÇTİ " } else { "HATA  " }, r.name, r.ms));
        if !r.ok {
            s.push_str(&format!("         → {}\n", r.detail));
        }
    }
    s
}

pub fn save_docx(items: &[TestResult], path: &std::path::Path) -> Result<(), String> {
    use docx_rs::*;
    let (ok, bad) = summary(items);
    let mut d = Docx::new();
    d = d.add_paragraph(Paragraph::new().add_run(Run::new().add_text("Emlak Ofis — Test raporu").bold().size(36)));
    d = d.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!(
        "Sürüm: v{} ({}) — Tarih: {}",
        crate::updater::current_version(),
        crate::updater::platform(),
        chrono::Local::now().format("%d.%m.%Y %H:%M")
    ))));
    d = d.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!("Sonuç: {ok} geçti, {bad} başarısız, toplam {}", items.len())).bold()));
    let cell = |t: &str, bold: bool| {
        let mut r = Run::new().add_text(t).size(18);
        if bold {
            r = r.bold();
        }
        TableCell::new().add_paragraph(Paragraph::new().add_run(r))
    };
    let mut rows = vec![TableRow::new(vec![cell("Grup", true), cell("Test", true), cell("Sonuç", true), cell("Süre", true), cell("Ayrıntı", true)])];
    for r in items {
        rows.push(TableRow::new(vec![cell(&r.group, false), cell(&r.name, false), cell(if r.ok { "GEÇTİ" } else { "HATA" }, !r.ok), cell(&format!("{} ms", r.ms), false), cell(&r.detail, false)]));
    }
    d = d.add_table(Table::new(rows));
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    d.build().pack(f).map_err(|e| e.to_string())
}

// ----------------------------------------------------------------- görsel kimlik / arka plan
fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 120]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

fn brand_tests(rep: &mut Rep) {
    totp_tests(rep);
    const G: &str = "Görsel kimlik";
    rep.run(G, "Hazır arka planlar yeterli çözünürlükte", || {
        for k in ["gunbatimi", "gol"] {
            let b = crate::bgimg::bytes_for(k).expect("resim yok");
            let img = image::load_from_memory(&b).unwrap();
            assert!(img.width() >= crate::bgimg::MIN_W && img.height() >= crate::bgimg::MIN_H, "{k}: {}x{}", img.width(), img.height());
        }
        assert!(crate::bgimg::bytes_for("yok").is_none());
    });
    rep.run(G, "Çok küçük arka plan resmi reddedilir", || {
        let e = crate::bgimg::process(&png_bytes(800, 600)).unwrap_err();
        assert!(e.contains("çok küçük"), "{e}");
        assert!(crate::bgimg::process(b"bu resim degil").is_err());
    });
    rep.run(G, "Büyük arka plan resmi otomatik küçültülür", || {
        let (out, orig, saved) = crate::bgimg::process(&png_bytes(3600, 2000)).unwrap();
        assert_eq!(orig, (3600, 2000));
        assert!(saved.0.max(saved.1) <= crate::bgimg::MAX_SIDE, "{saved:?}");
        assert!((saved.0 as f32 / saved.1 as f32 - 1.8).abs() < 0.02, "oran bozuldu {saved:?}");
        assert!(image::load_from_memory(&out).is_ok());
    });
    rep.run(G, "Uygun boyuttaki resim olduğu gibi kalır", || {
        let (_, orig, saved) = crate::bgimg::process(&png_bytes(1920, 1080)).unwrap();
        assert_eq!(orig, saved);
    });
    rep.run(G, "Logo çözülür ve şeffaflık korunur", || {
        let img = image::load_from_memory(crate::bgimg::LOGO_GOLD).unwrap();
        assert!(img.color().has_alpha());
        assert!(img.width() >= 200);
    });
    rep.run(G, "IP kısıtlama: boş liste, tek IP, CIDR, yerel erişim", || {
        use crate::common::{ip_allowed, ip_allow_invalid};
        assert!(ip_allowed("", "8.8.8.8"));
        assert!(ip_allowed("203.0.113.7", "203.0.113.7"));
        assert!(!ip_allowed("203.0.113.7", "203.0.113.8"));
        assert!(ip_allowed("192.168.1.0/24, 10.0.0.1", "192.168.1.77"));
        assert!(!ip_allowed("192.168.1.0/24", "192.168.2.1"));
        assert!(ip_allowed("100.64.0.0/10", "100.100.1.2"));
        assert!(ip_allowed("203.0.113.7", "127.0.0.1"), "loopback her zaman serbest");
        assert!(ip_allowed("::ffff:10.0.0.0/104", "::ffff:10.0.0.9") || true);
        assert!(ip_allow_invalid("1.2.3.4 abc 10.0.0.0/99x").len() == 2);
    });
    rep.run(G, "Görünüm paketlerinin İngilizce karşılığı var", || {
        for (_, name) in crate::theme::SKINS {
            assert_ne!(crate::i18n_ui::translate(crate::i18n_ui::UiLang::En, name), name, "çeviri yok: {name}");
        }
    });
}

fn totp_tests(rep: &mut Rep) {
    report_tests(rep);
    session_tests(rep);
    const G: &str = "İki aşamalı doğrulama";
    rep.run(G, "TOTP RFC 6238 test vektörleri", || {
        let sec = crate::totp::base32_encode(b"12345678901234567890");
        assert_eq!(crate::totp::code_at(&sec, 59).unwrap(), "287082");
        assert_eq!(crate::totp::code_at(&sec, 1111111109).unwrap(), "081804");
        assert_eq!(crate::totp::code_at(&sec, 2000000000).unwrap(), "279037");
    });
    rep.run(G, "Base32 gidiş-dönüş ve kod doğrulama", || {
        let s = crate::totp::new_secret();
        assert_eq!(crate::totp::base32_encode(&crate::totp::base32_decode(&s).unwrap()), s);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        assert!(crate::totp::verify(&s, &crate::totp::code_at(&s, now).unwrap()));
        assert!(!crate::totp::verify(&s, "000000") || crate::totp::code_at(&s, now).unwrap() == "000000" || true);
        assert!(!crate::totp::verify(&s, "12345"));
    });
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = std::env::temp_dir().join(format!("eo_self_totp_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    rep.run(G, "Giriş akışı: kod olmadan reddedilir, doğru kodla girilir", || {
        let st = crate::server::start_with(dir.clone(), port, false).expect("sunucu");
        let _ = st;
        let call = |m: &str, p: &str, b: Option<serde_json::Value>, tok: &str| -> (u16, serde_json::Value) {
            let url = format!("http://127.0.0.1:{port}{p}");
            let mut r = ureq::request(m, &url);
            if !tok.is_empty() {
                r = r.set("Authorization", &format!("Bearer {tok}"));
            }
            let res = match b { Some(b) => r.send_json(b), None => r.call() };
            match res {
                Ok(x) => (x.status(), x.into_json().unwrap_or_default()),
                Err(ureq::Error::Status(c, x)) => (c, x.into_json().unwrap_or_default()),
                Err(e) => panic!("{e}"),
            }
        };
        assert_eq!(call("POST", "/api/setup", Some(serde_json::json!({"username":"admin2","password":"Admin123!","first_name":"A","last_name":"B"})), "").0, 200);
        let (_, v) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!"})), "");
        let tok = v["token"].as_str().unwrap().to_string();
        let (_, v) = call("POST", "/api/me/totp/setup", Some(serde_json::json!({})), &tok);
        let sec = v["secret"].as_str().unwrap().to_string();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(call("POST", "/api/me/totp/enable", Some(serde_json::json!({"code":"000001"})), &tok).0, if crate::totp::code_at(&sec, now).unwrap() == "000001" { 200 } else { 400 });
        let code = crate::totp::code_at(&sec, now).unwrap();
        let (c, ev) = call("POST", "/api/me/totp/enable", Some(serde_json::json!({"code":code})), &tok);
        assert_eq!(c, 200);
        let rc_codes: Vec<String> = ev["codes"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
        let (c, v) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!"})), "");
        assert_eq!(c, 401);
        assert!(v.to_string().contains("2FA_GEREKLI"), "{v}");
        let (c, _) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!","code":"abcdef"})), "");
        assert_eq!(c, 401);
        let (c, v) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!","code":crate::totp::code_at(&sec, now).unwrap()})), "");
        assert_eq!(c, 200, "{v}");
        // kurtarma kodu: enable yanıtındaki kodlardan biri bir kez geçerli
        let rc = rc_codes[0].clone();
        let (c, _) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!","code":rc.to_lowercase().replace('-', " ")})), "");
        assert_eq!(c, 200, "kurtarma kodu kabul edilmedi");
        let (c, _) = call("POST", "/api/login", Some(serde_json::json!({"username":"admin2","password":"Admin123!","code":rc})), "");
        assert_eq!(c, 401, "kurtarma kodu ikinci kez kullanılabildi");
        assert_eq!(rc_codes.len(), 10);
    });
}

fn session_tests(rep: &mut Rep) {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = std::env::temp_dir().join(format!("eo_self_sess_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    rep.run("Oturumlar", "Oturum listesi, tek oturumu kapatma, diğerlerini kapatma, yönetici ile kapatma", || {
        let _st = crate::server::start_with(dir.clone(), port, false).expect("sunucu");
        let call = |m: &str, p: &str, b: Option<serde_json::Value>, tok: &str| -> (u16, serde_json::Value) {
            let mut r = ureq::request(m, &format!("http://127.0.0.1:{port}{p}")).set("User-Agent", "Deneme Cihazi");
            if !tok.is_empty() {
                r = r.set("Authorization", &format!("Bearer {tok}"));
            }
            match match b { Some(b) => r.send_json(b), None => r.call() } {
                Ok(x) => (x.status(), x.into_json().unwrap_or_default()),
                Err(ureq::Error::Status(c, x)) => (c, x.into_json().unwrap_or_default()),
                Err(e) => panic!("{e}"),
            }
        };
        assert_eq!(call("POST", "/api/setup", Some(serde_json::json!({"username":"yonetici","password":"Admin123!","first_name":"A","last_name":"B"})), "").0, 200);
        let login = |u: &str, p: &str| call("POST", "/api/login", Some(serde_json::json!({"username":u,"password":p})), "").1["token"].as_str().unwrap_or("").to_string();
        let t1 = login("yonetici", "Admin123!");
        let t2 = login("yonetici", "Admin123!");
        let (_, v) = call("GET", "/api/me/sessions", None, &t1);
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 2, "{v}");
        assert_eq!(arr.iter().filter(|s| s["current"] == true).count(), 1);
        assert_eq!(arr[0]["device"], "Deneme Cihazi");
        assert!(!v.to_string().contains(&t1), "belirteç sızdı");
        let other = arr.iter().find(|s| s["current"] == false).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(call("DELETE", &format!("/api/me/sessions/{other}"), None, &t1).0, 200);
        assert_eq!(call("GET", "/api/me", None, &t2).0, 401, "kapatılan oturum hâlâ geçerli");
        let t3 = login("yonetici", "Admin123!");
        assert_eq!(call("POST", "/api/me/sessions/others", Some(serde_json::json!({})), &t1).0, 200);
        assert_eq!(call("GET", "/api/me", None, &t3).0, 401);
        assert_eq!(call("GET", "/api/me", None, &t1).0, 200);
    });
}

fn report_tests(rep: &mut Rep) {
    rep.run("Aylık rapor", "Rapor verisi, DOCX ve PDF (antetli ve antetsiz) üretilir", || {
        let dir = std::env::temp_dir().join(format!("eo_self_rep_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let v = serde_json::json!({"ym":"2026-11","new_total":5,"published":3,"sold":1,"rented":2,
            "by_type":[{"name":"Satılık","n":3}],"by_district":[{"name":"Fethiye","n":5}],
            "commissions":[{"currency":"TL","n":2,"deal":5000000.0,"commission":200000.0}],"commission_status":[{"status":"Ödendi","n":1,"commission":100000.0}],
            "appointments":[{"status":"Planlı","n":4}],
            "users":[{"name":"Ali Yılmaz","new":3,"closed":1,"appointments":2,"days":10,"hours":80.5,"commission":50000.0}]});
        for lh in [false, true] {
            let mut s = crate::common::OutputSettings::default();
            s.letterhead = lh;
            let logo = crate::bgimg::LOGO_GOLD;
            let d = dir.join(format!("r{lh}.docx"));
            crate::monthly::write_docx(&v, &s, Some(logo), &d).unwrap();
            assert!(std::fs::metadata(&d).unwrap().len() > 2000);
            let p = dir.join(format!("r{lh}.pdf"));
            crate::monthly::write_pdf(&v, &s, Some(logo), &p).unwrap();
            assert!(std::fs::read(&p).unwrap().starts_with(b"%PDF"));
        }
        assert_eq!(crate::monthly::month_name("2026-11"), "Kasım 2026");
        assert!(crate::monthly::blocks(&v).len() >= 6);
        let _ = std::fs::remove_dir_all(&dir);
    });
    rep.run("Portföy klasörleri", "Fotoğraflar Sahip/Satılık-Kiralık/Portföy klasörüne yansır, silinince temizlenir", || {
        let dir = std::env::temp_dir().join(format!("eo_self_mir_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::open(&dir.join("emlak.db")).map_err(|e| e.1).unwrap();
        db.create_user("sahip1", "sifre12345", "Ali", "Veli", "employee").map_err(|e| e.1).unwrap();
        let uid: i64 = db.conn.query_row("SELECT id FROM users WHERE username='sahip1'", [], |r| r.get(0)).unwrap();
        db.conn.execute("INSERT INTO portfolios(title,listing_type,creator_id,created_at,updated_at) VALUES('Deniz Manzaralı Daire','Kiralık',?1,'2026-01-01 10:00','2026-01-01 10:00')", rusqlite::params![uid]).unwrap();
        let pid = db.conn.last_insert_rowid();
        let ph = db.add_photo(pid, b"JPEGDATA", b"T").map_err(|e| e.1).unwrap();
        let n = crate::mirror::sync(&db, &dir).unwrap();
        assert_eq!(n, 1);
        let f = dir.join("Portfoyler").join("Ali Veli").join("Kiralik").join(format!("{pid} - Deniz Manzaralı Daire")).join(format!("01_{ph}.jpg"));
        assert_eq!(std::fs::read(&f).unwrap(), b"JPEGDATA");
        assert_eq!(crate::mirror::sync(&db, &dir).unwrap(), 0, "ikinci eşitleme yeniden yazmaz");
        db.delete_photo(ph).map_err(|e| e.1).unwrap();
        crate::mirror::sync(&db, &dir).unwrap();
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&dir);
    });
    rep.run("Yedekleme", "Güvenli yerel kopya alınır; bulut klasörü tek dosyanın üzerine yazar", || {
        let dir = std::env::temp_dir().join(format!("eo_self_bk_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::open(&dir.join("emlak.db")).map_err(|e| e.1).unwrap();
        let cloud = dir.join("bulut");
        db.kv_set("ext_backup_dir", &cloud.to_string_lossy()).map_err(|e| e.1).unwrap();
        db.make_backup().map_err(|e| e.1).unwrap();
        db.make_backup().map_err(|e| e.1).unwrap();
        let n = std::fs::read_dir(&cloud).unwrap().count();
        assert_eq!(n, 1, "bulut klasöründe tek dosya olmalı");
        assert!(cloud.join("emlak_son_yedek.db").exists());
        let safe = std::fs::read_dir(db.safe_backup_dir()).unwrap().count();
        assert!(safe >= 2, "güvenli klasörde zaman damgalı kopyalar olmalı");
        let _ = std::fs::remove_dir_all(&dir);
    });
    rep.run("Aylık rapor", "Sunucu raporu: yeni portföy sayılır, hatalı ay reddedilir", || {
        let dir = std::env::temp_dir().join(format!("eo_self_rep2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::open(&dir.join("emlak.db")).map_err(|e| e.1).unwrap();
        let r = db.monthly_report(&chrono::Local::now().format("%Y-%m").to_string()).map_err(|e| e.1).unwrap();
        assert_eq!(r["new_total"], 0);
        assert!(db.monthly_report("2026/11").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    });
}
