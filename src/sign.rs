//! Güncelleme paketi imzalama (ed25519). Gizli anahtar yalnızca yayıncıda (sizde) kalır;
//! programlara yalnızca açık anahtar girilir. İmza, "sürüm + platform + dosyanın SHA-256 özeti"
//! üzerindedir; paket ana bilgisayardan diğer bilgisayarlara dağıtılırken de doğrulanır.
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

pub fn message(version: &str, platform: &str, bin: &[u8]) -> Vec<u8> {
    format!("EOUPD1\n{}\n{}\n{:x}\n", version.trim(), platform, Sha256::digest(bin)).into_bytes()
}

pub fn sign(secret_b64: &str, version: &str, platform: &str, bin: &[u8]) -> Result<String, String> {
    let seed = B64.decode(secret_b64.trim()).map_err(|_| "Gizli anahtar okunamadı.".to_string())?;
    let seed: [u8; 32] = seed.try_into().map_err(|_| "Gizli anahtar 32 bayt olmalı.".to_string())?;
    let sig = SigningKey::from_bytes(&seed).sign(&message(version, platform, bin));
    Ok(B64.encode(sig.to_bytes()))
}

pub fn verify(public_b64: &str, version: &str, platform: &str, bin: &[u8], sig_b64: &str) -> Result<(), String> {
    let pk = B64.decode(public_b64.trim()).map_err(|_| "Güvenilir yayıncı anahtarı geçersiz.".to_string())?;
    let pk: [u8; 32] = pk.try_into().map_err(|_| "Güvenilir yayıncı anahtarı 32 bayt olmalı.".to_string())?;
    let vk = VerifyingKey::from_bytes(&pk).map_err(|_| "Güvenilir yayıncı anahtarı geçersiz.".to_string())?;
    let sg = B64.decode(sig_b64.trim()).map_err(|_| "İmza okunamadı.".to_string())?;
    let sg: [u8; 64] = sg.try_into().map_err(|_| "İmza biçimi geçersiz.".to_string())?;
    vk.verify(&message(version, platform, bin), &Signature::from_bytes(&sg)).map_err(|_| "İmza GEÇERSİZ: paket güvenilen yayıncıdan gelmiyor veya değiştirilmiş.".to_string())
}

/// (gizli, açık) anahtar çifti — ikisi de base64
pub fn generate() -> (String, String) {
    let sk = SigningKey::generate(&mut rand::rngs::OsRng);
    (B64.encode(sk.to_bytes()), B64.encode(sk.verifying_key().to_bytes()))
}

/// `--anahtar-uret <önek>`: <önek>.gizli yazar, açık anahtarı ekrana basar.
pub fn cli_generate(prefix: &str) -> Result<(), String> {
    let (sec, public) = generate();
    let path = format!("{prefix}.gizli");
    if Path::new(&path).exists() {
        return Err(format!("{path} zaten var; üzerine yazılmadı."));
    }
    std::fs::write(&path, &sec).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    println!("Gizli anahtar yazıldı: {path}  (KİMSEYLE PAYLAŞMAYIN, GitHub'a yüklemeyin, yedeğini güvenli yerde tutun)");
    println!("Açık anahtar (programda Ayarlar > Hakkında > 'Güvenilir yayıncı anahtarı' alanına yapıştırın):");
    println!("{public}");
    Ok(())
}

/// `--paket-imzala <paket.zip> <gizli-anahtar-dosyası>`: surum.json'a platform başına imza ekler.
pub fn cli_sign_package(zip_path: &str, key_file: &str) -> Result<(), String> {
    let sec = std::fs::read_to_string(key_file).map_err(|e| format!("Anahtar dosyası okunamadı: {e}"))?;
    let data = std::fs::read(zip_path).map_err(|e| format!("Paket okunamadı: {e}"))?;
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(&data)).map_err(|_| "Geçerli bir zip değil.".to_string())?;
    let mut mtxt = String::new();
    zin.by_name("surum.json").map_err(|_| "Pakette surum.json yok.".to_string())?.read_to_string(&mut mtxt).map_err(|e| e.to_string())?;
    let mut m: serde_json::Value = serde_json::from_str(&mtxt).map_err(|e| e.to_string())?;
    let ver = m["surum"].as_str().unwrap_or("").to_string();
    let plats: Vec<String> = m["paketler"].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
    for plat in &plats {
        let file = m["paketler"][plat]["dosya"].as_str().unwrap_or("").to_string();
        let mut bin = Vec::new();
        zin.by_name(&file).map_err(|_| format!("Pakette {file} yok."))?.read_to_end(&mut bin).map_err(|e| e.to_string())?;
        let s = sign(&sec, &ver, plat, &bin)?;
        m["paketler"][plat]["imza"] = serde_json::Value::String(s);
    }
    let out_path = format!("{zip_path}.tmp");
    {
        let mut zout = zip::ZipWriter::new(std::fs::File::create(&out_path).map_err(|e| e.to_string())?);
        let o = zip::write::SimpleFileOptions::default();
        for i in 0..zin.len() {
            let mut f = zin.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            zout.start_file(&name, o).map_err(|e| e.to_string())?;
            if name == "surum.json" {
                zout.write_all(serde_json::to_string_pretty(&m).unwrap().as_bytes()).map_err(|e| e.to_string())?;
            } else {
                zout.write_all(&buf).map_err(|e| e.to_string())?;
            }
        }
        zout.finish().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&out_path, zip_path).map_err(|e| e.to_string())?;
    println!("İmzalandı: {zip_path} (v{ver}; {})", plats.join(", "));
    Ok(())
}
