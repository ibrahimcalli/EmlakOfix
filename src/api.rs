//! İstemci tarafı: sunucuyla HTTP/JSON konuşan katman.
use crate::common::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct Api {
    base: String,
    token: Arc<Mutex<String>>,
    agent: ureq::Agent,
    /// HTTPS ise sunucu sertifikasının parmak izi (kaydedilmek üzere)
    seen_fp: Option<Arc<Mutex<Option<String>>>>,
}

/// Bilinen sunucu parmak izleri ("adres:port" -> sha256 hex); ayarlar dosyasından yüklenir.
static PINS: std::sync::OnceLock<Mutex<std::collections::BTreeMap<String, String>>> = std::sync::OnceLock::new();
fn pins() -> &'static Mutex<std::collections::BTreeMap<String, String>> {
    PINS.get_or_init(|| Mutex::new(Default::default()))
}
pub fn set_pins(m: std::collections::BTreeMap<String, String>) {
    *pins().lock().unwrap() = m;
}
pub fn get_pins() -> std::collections::BTreeMap<String, String> {
    pins().lock().unwrap().clone()
}
pub fn learn_pin(addr: &str, fp: &str) {
    pins().lock().unwrap().insert(addr.to_string(), fp.to_string());
}
pub fn forget_pin(addr: &str) {
    pins().lock().unwrap().remove(addr);
}

impl Api {
    fn build(base: String, tls: Option<(Arc<rustls::ClientConfig>, Arc<Mutex<Option<String>>>)>) -> Api {
        let mut b = ureq::AgentBuilder::new().user_agent(&format!("Emlak Ofis {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::OS)).timeout_connect(Duration::from_secs(4)).timeout(Duration::from_secs(90));
        let mut seen_fp = None;
        if let Some((cfg, seen)) = tls {
            b = b.tls_config(cfg);
            seen_fp = Some(seen);
        }
        Api { base, token: Arc::new(Mutex::new(String::new())), agent: b.build(), seen_fp }
    }

    /// Adres "ip:port" ise önce HTTPS dener (kayıtlı parmak iziyle), olmazsa ve parmak izi
    /// kayıtlı değilse HTTP'ye düşer. Kayıtlı parmak izi varken HTTP'ye ASLA düşmez (indirgeme saldırısı).
    pub fn new(addr: &str) -> Api {
        let a = addr.trim().trim_end_matches('/');
        if a.starts_with("http://") {
            return Self::build(a.to_string(), None);
        }
        let host = a.trim_start_matches("https://");
        let pin = pins().lock().unwrap().get(host).cloned();
        let tls = crate::tls::pinned_config(pin.clone());
        let https = Self::build(format!("https://{host}"), Some((tls.0.clone(), tls.1.clone())));
        if pin.is_some() || a.starts_with("https://") {
            return https;
        }
        // Parmak izi henüz yok: HTTPS dene, olmazsa düz HTTP.
        let probe = https.agent.get(&format!("{}/api/ping", https.base)).timeout(Duration::from_millis(2500)).call();
        if probe.is_ok() {
            https
        } else {
            Self::build(format!("http://{host}"), None)
        }
    }

    /// HTTPS ise ve parmak izi yeniyse (ilk bağlantı) döndürür.
    pub fn seen_fingerprint(&self) -> Option<String> {
        self.seen_fp.as_ref().and_then(|s| s.lock().unwrap().clone())
    }
    pub fn is_https(&self) -> bool {
        self.base.starts_with("https://")
    }
    pub fn host(&self) -> String {
        self.base.trim_start_matches("https://").trim_start_matches("http://").to_string()
    }

    pub fn set_token(&self, t: &str) {
        *self.token.lock().unwrap() = t.to_string();
    }

    fn req(&self, method: &str, path: &str) -> ureq::Request {
        let mut r = self.agent.request(method, &format!("{}{}", self.base, path));
        let t = self.token.lock().unwrap().clone();
        if !t.is_empty() {
            r = r.set("Authorization", &format!("Bearer {t}"));
        }
        r
    }

    fn fin(r: Result<ureq::Response, ureq::Error>) -> Result<ureq::Response, String> {
        match r {
            Ok(x) => Ok(x),
            Err(ureq::Error::Status(code, resp)) => {
                let txt = resp.into_string().unwrap_or_default();
                let msg = serde_json::from_str::<Value>(&txt)
                    .ok()
                    .and_then(|v| v["error"].as_str().map(String::from))
                    .unwrap_or(format!("Sunucu hatası ({code})"));
                Err(msg)
            }
            Err(e) => {
                let t = e.to_string();
                if t.contains("parmak izi") {
                    Err("Sunucunun şifreleme sertifikası değişti. Ana bilgisayar yeniden kurulduysa Ayarlar > Bağlantı bölümünden \"Sertifika güvenini sıfırla\" deyin; beklenmedik bir durumsa ağınız dinleniyor olabilir, bağlanmayın.".to_string())
                } else {
                    Err(format!("Sunucuya bağlanılamadı: {e}"))
                }
            }
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        match Self::fin(self.req("GET", path).call()) {
            Ok(r) => {
                if crate::offline::enabled() && crate::offline::cacheable(path) {
                    let v: Value = r.into_json().map_err(|e| format!("Yanıt okunamadı: {e}"))?;
                    crate::offline::put(path, &v);
                    serde_json::from_value(v).map_err(|e| format!("Yanıt okunamadı: {e}"))
                } else {
                    r.into_json::<T>().map_err(|e| format!("Yanıt okunamadı: {e}"))
                }
            }
            Err(e) if crate::outbox::is_conn_err(&e) && crate::offline::cacheable(path) => match crate::offline::get(path).and_then(|v| serde_json::from_value(v).ok()) {
                Some(t) => Ok(t),
                None => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    pub fn send<B: Serialize, T: DeserializeOwned>(&self, method: &str, path: &str, body: &B) -> Result<T, String> {
        let v = serde_json::to_value(body).map_err(|e| e.to_string())?;
        let r = Self::fin(self.req(method, path).send_json(v))?;
        r.into_json::<T>().map_err(|e| format!("Yanıt okunamadı: {e}"))
    }

    /// JSON gövdeli yazma işlemi; bağlantı yoksa ve işlem kuyruğa uygunsa çevrimdışı kuyruğa alınır.
    fn mutate<B: Serialize>(&self, method: &str, path: &str, body: &B) -> Result<Value, String> {
        match self.send::<B, Value>(method, path, body) {
            Err(e) if crate::outbox::is_conn_err(&e) && crate::outbox::queueable(method, path) => {
                Ok(crate::outbox::enqueue(method, path, serde_json::to_value(body).ok(), None))
            }
            r => r,
        }
    }
    pub fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<Value, String> {
        self.mutate("POST", path, body)
    }
    pub fn put<B: Serialize>(&self, path: &str, body: &B) -> Result<Value, String> {
        self.mutate("PUT", path, body)
    }
    pub fn delete(&self, path: &str) -> Result<Value, String> {
        let r = Self::fin(self.req("DELETE", path).call());
        match r {
            Err(e) if crate::outbox::is_conn_err(&e) && crate::outbox::queueable("DELETE", path) => Ok(crate::outbox::enqueue("DELETE", path, None, None)),
            r => r?.into_json::<Value>().map_err(|e| e.to_string()),
        }
    }

    /// Kuyruktaki bir işlemi (kuyruğa geri almadan) gönderir; kayıt zamanı sunucuya denetim kaydı için iletilir.
    pub fn send_queued(&self, method: &str, path: &str, body: Option<&Value>, bytes: Option<&[u8]>, at: &str) -> Result<Value, String> {
        let r = self.req(method, path).set("X-Offline-At", &at.replace(['\n', '\r'], " "));
        let resp = match (body, bytes) {
            (_, Some(b)) => Self::fin(r.set("Content-Type", "application/octet-stream").send_bytes(b))?,
            (Some(v), None) => Self::fin(r.send_json(v.clone()))?,
            (None, None) => Self::fin(r.call())?,
        };
        resp.into_json::<Value>().map_err(|e| e.to_string())
    }

    pub fn get_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let r = Self::fin(self.req("GET", path).call())?;
        let mut buf = Vec::new();
        r.into_reader().take(64 * 1024 * 1024).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }

    pub fn send_bytes(&self, method: &str, path: &str, bytes: &[u8]) -> Result<Value, String> {
        let r = Self::fin(self.req(method, path).set("Content-Type", "application/octet-stream").send_bytes(bytes));
        match r {
            Err(e) if crate::outbox::is_conn_err(&e) && crate::outbox::queueable(method, path) => Ok(crate::outbox::enqueue(method, path, None, Some(bytes))),
            r => r?.into_json::<Value>().map_err(|e| e.to_string()),
        }
    }

    pub fn ping(&self) -> Result<Ping, String> {
        let r = Self::fin(self.agent.get(&format!("{}/api/ping", self.base)).timeout(Duration::from_secs(4)).call())?;
        r.into_json::<Ping>().map_err(|e| e.to_string())
    }

    pub fn login(&self, username: &str, password: &str, code: &str) -> Result<LoginReply, String> {
        let rep: LoginReply = self.send("POST", "/api/login", &serde_json::json!({"username": username, "password": password, "code": code}))?;
        self.set_token(&rep.token);
        Ok(rep)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Ping {
    pub app: String,
    pub version: String,
    pub needs_setup: bool,
}

/// Yerel ağda çalışan sunucuları UDP yayınıyla bulur.
/// Pin kullanmadan (yalnızca "bu adreste Emlak Ofis var mı" diye) HTTPS ve HTTP dener.
pub fn ping_any(addr: &str) -> Option<Ping> {
    let (cfg, _) = crate::tls::pinned_config(None);
    let https = Api::build(format!("https://{addr}"), Some((cfg, Arc::new(Mutex::new(None)))));
    https.ping().ok().or_else(|| Api::build(format!("http://{addr}"), None).ping().ok())
}

pub fn discover() -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(1200)));
        let _ = sock.send_to(b"EMLAK-OFIS?", ("255.255.255.255", DISCOVERY_PORT));
        let mut buf = [0u8; 64];
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(1500) {
            if let Ok((n, from)) = sock.recv_from(&mut buf) {
                let msg = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(port) = msg.strip_prefix("EMLAK-OFIS!") {
                    let addr = format!("{}:{}", from.ip(), port.trim());
                    if !found.contains(&addr) {
                        found.push(addr);
                    }
                }
            } else {
                break;
            }
        }
    }
    found
}

/// UDP yayını + yerel ağın (/24) TCP taraması. Wi-Fi yönlendiricileri yayın paketlerini
/// engelleyebildiği için ikinci yöntem kablosuz bağlantılarda da ana bilgisayarı bulur.
pub fn discover_all(port: u16) -> Vec<String> {
    use std::net::{SocketAddr, TcpStream};
    let mut found = discover();
    let mut prefixes: Vec<String> = Vec::new();
    for ip in crate::server::local_ips() {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            let p = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
            if !prefixes.contains(&p) {
                prefixes.push(p);
            }
        }
    }
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();
    for pre in prefixes {
        for host in 1..255u32 {
            let hits = hits.clone();
            let addr = format!("{pre}.{host}");
            handles.push(std::thread::spawn(move || {
                if let Ok(sa) = format!("{addr}:{port}").parse::<SocketAddr>() {
                    if TcpStream::connect_timeout(&sa, Duration::from_millis(700)).is_ok() {
                        let full = format!("{addr}:{port}");
                        if ping_any(&full).map(|p| p.app == "emlak-ofis").unwrap_or(false) {
                            if let Ok(mut h) = hits.lock() {
                                h.push(full);
                            }
                        }
                    }
                }
            }));
        }
    }
    for h in handles {
        let _ = h.join();
    }
    if let Ok(h) = hits.lock() {
        for a in h.iter() {
            if !found.contains(a) {
                found.push(a.clone());
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    /// Çalışan bir sunucu (7999) gerektirir: elle çalıştırın: cargo test -- --ignored
    #[test]
    #[ignore]
    fn ag_taramasi_sunucuyu_bulur() {
        let v = super::discover_all(7999);
        println!("bulunan: {v:?}");
        assert!(!v.is_empty());
    }
}
