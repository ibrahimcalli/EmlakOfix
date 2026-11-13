//! Şifreli (HTTPS) bağlantı: ana bilgisayar kendi kendine imzalı sertifika üretir; istemciler ilk
//! bağlantıda sertifikanın SHA-256 parmak izini kaydeder (ilk kullanımda güven – TOFU) ve sonraki
//! bağlantılarda yalnızca aynı parmak iziyle konuşur. Parmak izi değişirse bağlantı reddedilir.
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring as provider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct ServerCert {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub fingerprint: String,
}

pub fn fp_of_der(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02x}")).collect()
}

/// "ab12…" → "AB:12:…" (ekranda göstermek için)
pub fn fp_pretty(fp: &str) -> String {
    fp.as_bytes().chunks(2).map(|c| String::from_utf8_lossy(c).to_uppercase()).collect::<Vec<_>>().join(":")
}

fn der_from_pem(pem: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine;
    let s = String::from_utf8_lossy(pem);
    let body: String = s.lines().filter(|l| !l.starts_with("-----")).map(|l| l.trim()).collect();
    base64::engine::general_purpose::STANDARD.decode(body).ok()
}

/// Veri klasöründe sertifika yoksa üretir; varsa okur.
pub fn ensure_cert(dir: &Path) -> Result<ServerCert, String> {
    let (cp, kp) = (dir.join("tls_cert.pem"), dir.join("tls_key.pem"));
    if let (Ok(c), Ok(k)) = (std::fs::read(&cp), std::fs::read(&kp)) {
        if let Some(der) = der_from_pem(&c) {
            return Ok(ServerCert { fingerprint: fp_of_der(&der), cert_pem: c, key_pem: k });
        }
    }
    let mut names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    for ip in crate::server::local_ips() {
        if !names.contains(&ip) {
            names.push(ip);
        }
    }
    let ck = rcgen::generate_simple_self_signed(names).map_err(|e| format!("Sertifika üretilemedi: {e}"))?;
    let (c, k) = (ck.cert.pem().into_bytes(), ck.key_pair.serialize_pem().into_bytes());
    let _ = std::fs::create_dir_all(dir);
    std::fs::write(&cp, &c).map_err(|e| format!("Sertifika yazılamadı: {e}"))?;
    std::fs::write(&kp, &k).map_err(|e| format!("Anahtar yazılamadı: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&kp, std::fs::Permissions::from_mode(0o600));
    }
    Ok(ServerCert { fingerprint: fp_of_der(ck.cert.der()), cert_pem: c, key_pem: k })
}

/// İstemci doğrulayıcı: `pin` varsa yalnızca o parmak izini kabul eder; yoksa ilk görüleni
/// `seen` içine yazar ve kabul eder (TOFU – çağıran taraf kaydetmekten sorumludur).
#[derive(Debug)]
pub struct PinVerifier {
    pin: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, _i: &[CertificateDer<'_>], _n: &ServerName<'_>, _o: &[u8], _now: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let fp = fp_of_der(end_entity.as_ref());
        *self.seen.lock().unwrap() = Some(fp.clone());
        match &self.pin {
            Some(p) if *p != fp => Err(rustls::Error::General("sertifika parmak izi eşleşmiyor".into())),
            _ => Ok(ServerCertVerified::assertion()),
        }
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &provider::default_provider().signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &provider::default_provider().signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider::default_provider().signature_verification_algorithms.supported_schemes()
    }
}

/// (ureq TLS yapılandırması, görülen parmak izi kutusu)
pub fn pinned_config(pin: Option<String>) -> (Arc<rustls::ClientConfig>, Arc<Mutex<Option<String>>>) {
    let seen = Arc::new(Mutex::new(None));
    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(provider::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("TLS sürümleri")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinVerifier { pin, seen: seen.clone() }))
        .with_no_client_auth();
    (Arc::new(cfg), seen)
}
