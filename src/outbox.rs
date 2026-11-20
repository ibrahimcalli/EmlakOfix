//! Çevrimdışı kuyruk (giden kutusu): ana bilgisayara ulaşılamazken yapılan kayıt/düzenleme/silme
//! işlemleri bu bilgisayarda diske yazılır; bağlantı gelince sırayla ana bilgisayara gönderilir.
//! Kuyruk kullanıcı başınadır ve program kapansa da kaybolmaz.
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
    /// İkili gövde (fotoğraf) base64
    pub bytes_b64: Option<String>,
    /// Çevrimdışı oluşturulan kaydın geçici (negatif) kimliği
    pub temp: Option<i64>,
    pub at: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Disk {
    entries: Vec<Entry>,
    next_temp: i64,
}

#[derive(Default)]
struct State {
    file: Option<PathBuf>,
    disk: Disk,
    maps: Vec<(i64, i64)>,
    flushing: bool,
    notice: Option<String>,
}

static ST: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = ST.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(State::default))
}

fn save(s: &State) {
    if let Some(p) = &s.file {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if s.disk.entries.is_empty() {
            let _ = std::fs::remove_file(p);
        } else if let Ok(t) = serde_json::to_vec(&s.disk) {
            let _ = std::fs::write(p, t);
        }
    }
}

/// Kullanıcı girişinden sonra çağrılır; o kullanıcının bekleyen kuyruğunu yükler.
pub fn init(file: PathBuf) {
    with(|s| {
        s.disk = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if s.disk.next_temp >= 0 {
            s.disk.next_temp = -1;
        }
        s.file = Some(file);
        s.maps.clear();
        s.flushing = false;
    })
}

pub fn pending() -> usize {
    with(|s| s.disk.entries.len())
}

pub fn take_notice() -> Option<String> {
    with(|s| s.notice.take())
}

const PREFIXES: [&str; 11] = [
    "/api/portfolios", "/api/photos", "/api/crm", "/api/tasks", "/api/appointments", "/api/buyers", "/api/notes", "/api/leaves", "/api/goals", "/api/commissions", "/api/attendance",
];

/// Yalnızca kayıt içeriği işlemleri kuyruğa alınır; giriş, parola, kullanıcı yönetimi, ayarlar, yedek/geri yükleme alınmaz.
pub fn queueable(method: &str, path: &str) -> bool {
    matches!(method, "POST" | "PUT" | "DELETE") && PREFIXES.iter().any(|p| path == *p || path.starts_with(&format!("{p}/")) || path.starts_with(&format!("{p}?")))
}

pub fn is_conn_err(e: &str) -> bool {
    e.starts_with("Sunucuya bağlanılamadı")
}

/// İşlemi kuyruğa ekler; arayüze "başarılı" gibi görünen bir yanıt döndürür (POST için geçici kimlikle).
pub fn enqueue(method: &str, path: &str, body: Option<Value>, bytes: Option<&[u8]>) -> Value {
    with(|s| {
        let temp = if method == "POST" {
            if s.disk.next_temp >= 0 {
                s.disk.next_temp = -1;
            }
            let t = s.disk.next_temp;
            s.disk.next_temp -= 1;
            Some(t)
        } else {
            None
        };
        s.disk.entries.push(Entry {
            method: method.into(),
            path: path.into(),
            body,
            bytes_b64: bytes.map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
            temp,
            at: crate::common::now_str(),
        });
        save(s);
        s.notice = Some(format!("Ana bilgisayara ulaşılamıyor: kayıt bu bilgisayarda bekletiliyor, bağlantı gelince otomatik gönderilecek ({} bekleyen).", s.disk.entries.len()));
        match temp {
            Some(t) => json!({"id": t, "queued": true}),
            None => json!({"ok": true, "queued": true}),
        }
    })
}

fn remap_path(path: &str, maps: &[(i64, i64)]) -> String {
    let (base, q) = match path.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (path, None),
    };
    let segs: Vec<String> = base
        .split('/')
        .map(|sg| match sg.parse::<i64>() {
            Ok(n) if n < 0 => maps.iter().find(|(t, _)| *t == n).map(|(_, r)| r.to_string()).unwrap_or_else(|| sg.to_string()),
            _ => sg.to_string(),
        })
        .collect();
    let mut out = segs.join("/");
    if let Some(q) = q {
        out.push('?');
        out.push_str(q);
    }
    out
}

fn remap_value(v: &mut Value, maps: &[(i64, i64)]) {
    match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i < 0 {
                    if let Some((_, r)) = maps.iter().find(|(t, _)| *t == i) {
                        *v = json!(*r);
                    }
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| remap_value(x, maps)),
        Value::Object(o) => o.values_mut().for_each(|x| remap_value(x, maps)),
        _ => {}
    }
}

#[derive(Default, Debug)]
pub struct FlushOut {
    pub sent: usize,
    pub failed: Vec<String>,
    pub stopped: Option<String>,
    pub remaining: usize,
}

/// Kuyruğu sırayla gönderir. `send(yöntem, yol, gövde, bayt, kayıt_zamanı)` gerçek ağ çağrısıdır.
pub fn flush(send: &dyn Fn(&str, &str, Option<&Value>, Option<&[u8]>, &str) -> Result<Value, String>) -> FlushOut {
    if with(|s| std::mem::replace(&mut s.flushing, true)) {
        return FlushOut::default();
    }
    let mut out = FlushOut::default();
    loop {
        let Some((e, maps)) = with(|s| s.disk.entries.first().cloned().map(|e| (e, s.maps.clone()))) else { break };
        let path = remap_path(&e.path, &maps);
        let mut body = e.body.clone();
        if let Some(b) = body.as_mut() {
            remap_value(b, &maps);
        }
        let bytes = e.bytes_b64.as_ref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok());
        match send(&e.method, &path, body.as_ref(), bytes.as_deref(), &e.at) {
            Ok(resp) => {
                with(|s| {
                    if let (Some(t), Some(real)) = (e.temp, resp.get("id").and_then(|v| v.as_i64())) {
                        s.maps.push((t, real));
                    }
                    if !s.disk.entries.is_empty() {
                        s.disk.entries.remove(0);
                    }
                    save(s);
                });
                out.sent += 1;
            }
            Err(msg) => {
                if is_conn_err(&msg) || msg.contains("Oturum süresi doldu") {
                    out.stopped = Some(msg);
                    break;
                }
                // Kalıcı hata (ör. kayıt başkası tarafından silinmiş): atla, kullanıcıya bildir
                with(|s| {
                    if !s.disk.entries.is_empty() {
                        s.disk.entries.remove(0);
                    }
                    save(s);
                });
                out.failed.push(format!("{} {}: {}", e.method, e.path, msg));
            }
        }
    }
    out.remaining = with(|s| {
        s.flushing = false;
        s.disk.entries.len()
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temp_ids_are_remapped() {
        let maps = vec![(-1, 42)];
        assert_eq!(remap_path("/api/portfolios/-1/photos", &maps), "/api/portfolios/42/photos");
        assert_eq!(remap_path("/api/portfolios/-2/photos", &maps), "/api/portfolios/-2/photos");
        let mut v = json!({"a": -1, "b": [-1, 5], "c": {"d": -1}});
        remap_value(&mut v, &maps);
        assert_eq!(v, json!({"a": 42, "b": [42, 5], "c": {"d": 42}}));
    }

    #[test]
    fn queue_flush_roundtrip() {
        let f = std::env::temp_dir().join(format!("eo_outbox_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        init(f.clone());
        let r = enqueue("POST", "/api/portfolios", Some(json!({"title": "X"})), None);
        let tmp = r["id"].as_i64().unwrap();
        assert!(tmp < 0);
        enqueue("POST", &format!("/api/portfolios/{tmp}/photos"), None, Some(b"IMG"));
        enqueue("DELETE", "/api/tasks/7", None, None);
        assert_eq!(pending(), 3);
        assert!(f.exists());
        // bağlantı yok: hiçbiri gitmez
        let o = flush(&|_, _, _, _, _| Err("Sunucuya bağlanılamadı: x".into()));
        assert_eq!((o.sent, o.remaining), (0, 3));
        // diskten yeniden yükle (program kapanıp açılmış gibi)
        init(f.clone());
        assert_eq!(pending(), 3);
        let seen = Mutex::new(Vec::new());
        let o = flush(&|m, p, _, b, _| {
            seen.lock().unwrap().push(format!("{m} {p} {}", b.map(|x| x.len()).unwrap_or(0)));
            Ok(json!({"id": 99}))
        });
        assert_eq!((o.sent, o.remaining), (3, 0));
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen[1], "POST /api/portfolios/99/photos 3");
        assert!(!f.exists());
    }
}
