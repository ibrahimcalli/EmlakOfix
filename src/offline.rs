//! Çevrimdışı giriş ve yerel şifreli veri kopyası.
//! Yönetici açtıysa (Ayarlar > Yedekleme), her başarılı GET yanıtı bu bilgisayarda, kullanıcının
//! parolasından türetilen anahtarla ŞİFRELİ saklanır. Ana bilgisayara ulaşılamazsa aynı parola ile
//! çevrimdışı giriş yapılabilir; listeler son bilinen veriden gösterilir, yeni işlemler kuyruğa alınır.
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
struct St {
    enabled: bool,
    file: Option<PathBuf>,
    pw: String,
    map: BTreeMap<String, Value>,
    me: Value,
    dirty: bool,
}

static ST: Mutex<Option<St>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut St) -> R) -> R {
    let mut g = ST.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(St::default))
}

const SKIP: [&str; 12] = ["/api/rev", "/api/ping", "photo", "thumb", "report", "sessions", "totp", "backup", "update", "audit", "download", "/api/login"];

pub fn cacheable(path: &str) -> bool {
    path.starts_with("/api/") && !SKIP.iter().any(|s| path.contains(s))
}

pub fn enabled() -> bool {
    with(|s| s.enabled)
}

pub fn put(path: &str, v: &Value) {
    with(|s| {
        if s.enabled {
            if s.map.get(path) != Some(v) {
                s.map.insert(path.to_string(), v.clone());
                s.dirty = true;
            }
        }
    })
}

pub fn get(path: &str) -> Option<Value> {
    with(|s| if s.enabled { s.map.get(path).cloned() } else { None })
}

/// Çevrimiçi girişten sonra: önbellek etkinse mevcut dosyayı yükler (yoksa boş başlar); kapalıysa dosyayı siler.
pub fn start(file: PathBuf, pw: &str, enabled: bool, me: &Value) {
    with(|s| {
        s.enabled = enabled;
        s.file = Some(file.clone());
        s.pw = pw.to_string();
        s.me = me.clone();
        s.map.clear();
        s.dirty = enabled;
        if !enabled {
            let _ = std::fs::remove_file(&file);
        } else if let Ok(raw) = std::fs::read(&file) {
            if let Ok(plain) = crate::ext_db::decrypt_bytes(&raw, pw) {
                if let Ok(v) = serde_json::from_slice::<Value>(&plain) {
                    if let Some(o) = v.get("paths").and_then(|p| p.as_object()) {
                        s.map = o.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    }
                }
            }
        }
    })
}

/// Çevrimdışı giriş: parola doğruysa kullanıcı bilgisini döndürür ve önbelleği yükler.
pub fn login(file: PathBuf, pw: &str) -> Result<Value, String> {
    let raw = std::fs::read(&file).map_err(|_| "Ana bilgisayara ulaşılamıyor ve bu bilgisayarda çevrimdışı kopya yok (çevrimdışı giriş için önce bir kez çevrimiçi giriş yapılmış olmalı).".to_string())?;
    let plain = crate::ext_db::decrypt_bytes(&raw, pw).map_err(|_| "Ana bilgisayara ulaşılamıyor; çevrimdışı giriş için şifre doğrulanamadı.".to_string())?;
    let v: Value = serde_json::from_slice(&plain).map_err(|_| "Çevrimdışı kopya okunamadı.".to_string())?;
    let me = v.get("me").cloned().ok_or("Çevrimdışı kopya eksik.")?;
    with(|s| {
        s.enabled = true;
        s.file = Some(file);
        s.pw = pw.to_string();
        s.me = me.clone();
        s.map = v.get("paths").and_then(|p| p.as_object()).map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        s.dirty = false;
    });
    Ok(me)
}

/// Değişiklik varsa şifreli dosyayı yazar.
pub fn save_if_dirty() {
    with(|s| {
        if !s.enabled || !s.dirty {
            return;
        }
        let Some(f) = s.file.clone() else { return };
        let body = json!({"me": s.me, "paths": s.map});
        if let Ok(bytes) = serde_json::to_vec(&body) {
            if let Ok(enc) = crate::ext_db::encrypt_bytes(&bytes, &s.pw) {
                if let Some(d) = f.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let tmp = f.with_extension("tmp");
                if std::fs::write(&tmp, enc).is_ok() && std::fs::rename(&tmp, &f).is_ok() {
                    s.dirty = false;
                }
            }
        }
    })
}

pub fn clear_memory() {
    save_if_dirty();
    with(|s| *s = St::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_roundtrip_and_wrong_password() {
        let f = std::env::temp_dir().join(format!("eo_off_{}.eoc", std::process::id()));
        let _ = std::fs::remove_file(&f);
        start(f.clone(), "parola-1", true, &json!({"username":"ali","role":"employee"}));
        put("/api/portfolios", &json!([{"id":1,"title":"Daire"}]));
        put("/api/rev", &json!({"rev":1}));
        assert!(!cacheable("/api/rev"));
        save_if_dirty();
        assert!(f.exists());
        let raw = std::fs::read(&f).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("Daire"), "dosya şifreli olmalı");
        clear_memory();
        assert!(login(f.clone(), "yanlis").is_err());
        let me = login(f.clone(), "parola-1").unwrap();
        assert_eq!(me["username"], "ali");
        assert_eq!(get("/api/portfolios").unwrap()[0]["title"], "Daire");
        // yönetici kapattı: dosya silinir
        start(f.clone(), "parola-1", false, &json!({}));
        assert!(!f.exists());
        clear_memory();
    }
}
