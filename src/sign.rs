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

const HOST_TAG: &str = "EOHOST1";
const CODE_TAG: &str = "EOCODE2";
pub const NO_LIMIT: &str = "9999-12-31";

/// Doğrulanmış kodun içeriği.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeInfo {
    pub id: String,
    /// Kodun kurulumda KULLANILABİLECEĞİ son gün
    pub install_by: String,
    pub note: String,
    /// 'H' = yönetici (ana bilgisayar), 'K' = kullanıcı (istemci)
    pub kind: char,
    /// Programın ÇALIŞABİLECEĞİ son gün (lisans bitiş tarihi); 9999-12-31 = süresiz
    pub license_until: String,
}

fn code_msg(c: &CodeInfo) -> Vec<u8> {
    format!("{CODE_TAG}\n{}\n{}\n{}\n{}\n{}\n", c.id, c.install_by, c.kind, c.license_until, c.note).into_bytes()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn day_after(days: u32) -> String {
    if days == 0 { NO_LIMIT.to_string() } else { (chrono::Local::now() + chrono::Duration::days(days.min(3650) as i64)).format("%Y-%m-%d").to_string() }
}

/// Kurulum/lisans kodu üretir (gizli anahtarı olan yayıncı): EO-<yük>.<imza>
/// `install_days`: kodun kaç gün içinde kullanılabileceği (0 = süresiz); `license_days`: program kaç gün çalışabilir (0 = süresiz)
pub fn make_code(secret_b64: &str, kind: char, install_days: u32, license_days: u32, note: &str) -> Result<String, String> {
    let seed = B64.decode(secret_b64.trim()).map_err(|_| "Gizli anahtar okunamadı.".to_string())?;
    let seed: [u8; 32] = seed.try_into().map_err(|_| "Gizli anahtar 32 bayt olmalı.".to_string())?;
    if kind != 'H' && kind != 'K' {
        return Err("Kod türü H (yönetici) veya K (kullanıcı) olmalı.".into());
    }
    let note: String = note.chars().filter(|c| !c.is_control() && *c != '|').take(40).collect();
    let info = CodeInfo {
        id: (0..6).map(|_| format!("{:02x}", rand::random::<u8>())).collect(),
        install_by: day_after(install_days),
        note,
        kind,
        license_until: day_after(license_days),
    };
    let payload = format!("{}|{}|{}|{}|{}", info.id, info.install_by, info.note, info.kind, info.license_until);
    let sig = SigningKey::from_bytes(&seed).sign(&code_msg(&info));
    let u = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    Ok(format!("EO-{}.{}", u.encode(payload.as_bytes()), u.encode(sig.to_bytes())))
}

/// Eski (v1) uyumluluğu: yönetici kodu, süresiz lisans.
pub fn make_host_code(secret_b64: &str, days: u32, note: &str) -> Result<String, String> {
    make_code(secret_b64, 'H', days, 0, note)
}

/// Kodu doğrular (imza + kurulum son günü); lisans tarihini burada KONTROL ETMEZ.
pub fn verify_code(public_b64: &str, code: &str) -> Result<CodeInfo, String> {
    let u = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let bad = || "Kod geçersiz.".to_string();
    let code = code.trim().strip_prefix("EO-").ok_or_else(bad)?;
    let (p, s) = code.split_once('.').ok_or_else(bad)?;
    let payload = String::from_utf8(u.decode(p).map_err(|_| bad())?).map_err(|_| bad())?;
    let parts: Vec<&str> = payload.split('|').collect();
    let info = match parts.len() {
        // v1: id|exp|note  → yönetici, süresiz lisans
        3 => CodeInfo { id: parts[0].into(), install_by: parts[1].into(), note: parts[2].into(), kind: 'H', license_until: NO_LIMIT.into() },
        5 => CodeInfo { id: parts[0].into(), install_by: parts[1].into(), note: parts[2].into(), kind: parts[3].chars().next().unwrap_or('?'), license_until: parts[4].into() },
        _ => return Err(bad()),
    };
    let msg = if parts.len() == 3 { host_msg(&info.id, &info.install_by, &info.note) } else { code_msg(&info) };
    let pk = B64.decode(public_b64.trim()).map_err(|_| "Yayıncı anahtarı geçersiz.".to_string())?;
    let pk: [u8; 32] = pk.try_into().map_err(|_| "Yayıncı anahtarı 32 bayt olmalı.".to_string())?;
    let vk = VerifyingKey::from_bytes(&pk).map_err(|_| "Yayıncı anahtarı geçersiz.".to_string())?;
    let sg: [u8; 64] = u.decode(s).map_err(|_| bad())?.try_into().map_err(|_| bad())?;
    vk.verify(&msg, &Signature::from_bytes(&sg)).map_err(|_| "Kod geçersiz: yayıncı imzası doğrulanamadı.".to_string())?;
    if info.kind != 'H' && info.kind != 'K' {
        return Err(bad());
    }
    if info.install_by.as_str() < today().as_str() {
        return Err(format!("Kodun kullanım süresi dolmuş ({}).", info.install_by));
    }
    if info.license_until.as_str() < today().as_str() {
        return Err(format!("Kodun lisans tarihi geçmiş ({}).", info.license_until));
    }
    Ok(info)
}

/// Eski ad (testler/uyumluluk): (kimlik, son tarih, not)
pub fn verify_host_code(public_b64: &str, code: &str) -> Result<(String, String, String), String> {
    let i = verify_code(public_b64, code)?;
    Ok((i.id, i.install_by, i.note))
}

/// `--kod-uret <gizli-dosya> <H|K> <kurulum-gün> <lisans-gün> [not]` — programı açmadan, herhangi bir bilgisayarda kod üretir.
pub fn cli_make_code(key_file: &str, kind: &str, install_days: &str, license_days: &str, note: &str) -> Result<(), String> {
    let sec = std::fs::read_to_string(key_file).map_err(|e| format!("Anahtar dosyası okunamadı: {e}"))?;
    let k = kind.trim().to_uppercase().chars().next().unwrap_or('?');
    let c = make_code(&sec, k, install_days.parse().map_err(|_| "Kurulum günü sayı olmalı.".to_string())?, license_days.parse().map_err(|_| "Lisans günü sayı olmalı.".to_string())?, note)?;
    println!("{c}");
    Ok(())
}

/// Lisans kilidi: tarih geçtiyse true. `last_seen` saat geri alınarak kandırmayı zorlaştırır.
pub fn license_expired(license_until: &str, last_seen: &str) -> bool {
    if license_until.trim().is_empty() || license_until == NO_LIMIT {
        return false;
    }
    let t = today();
    let now = if last_seen > t.as_str() { last_seen } else { t.as_str() };
    license_until < now
}

/// Lisansın bitmesine kalan gün (süresiz/bilinmiyorsa None)
pub fn license_days_left(license_until: &str) -> Option<i64> {
    if license_until.trim().is_empty() || license_until == NO_LIMIT {
        return None;
    }
    let d = chrono::NaiveDate::parse_from_str(license_until, "%Y-%m-%d").ok()?;
    Some((d - chrono::Local::now().date_naive()).num_days())
}

fn host_msg(id: &str, exp: &str, note: &str) -> Vec<u8> {
    format!("{HOST_TAG}\n{id}\n{exp}\n{note}\n").into_bytes()
}

/// Kod koruması için kullanılan yayıncı açık anahtarı: ayarlardaki güvenilir anahtar, yoksa derlemeye gömülü anahtar.
pub fn host_key(cfg_key: &str) -> String {
    if !cfg_key.trim().is_empty() {
        return cfg_key.trim().to_string();
    }
    include_str!("../assets/yayinci_anahtar.pub").lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or("").to_string()
}

#[cfg(test)]
mod host_tests {
    use super::*;
    #[test]
    fn host_code_roundtrip() {
        let (sec, public) = generate();
        let c = make_host_code(&sec, 7, "Ofis A").unwrap();
        let (_, _, n) = verify_host_code(&public, &c).unwrap();
        assert_eq!(n, "Ofis A");
        let (_, other) = generate();
        assert!(verify_host_code(&other, &c).is_err());
        assert!(verify_host_code(&public, "EO-abc.def").is_err());
        let mut t = c.clone();
        t.insert(5, 'x');
        assert!(verify_host_code(&public, &t).is_err());
    }
    #[test]
    fn code_v2_kinds_and_license() {
        let (sec, public) = generate();
        let c = make_code(&sec, 'K', 3, 30, "Ali").unwrap();
        let i = verify_code(&public, &c).unwrap();
        assert_eq!(i.kind, 'K');
        assert_ne!(i.license_until, NO_LIMIT);
        assert!(!license_expired(&i.license_until, ""));
        assert!(license_expired("2000-01-01", ""));
        assert!(!license_expired(NO_LIMIT, ""));
        assert!(make_code(&sec, 'X', 1, 1, "").is_err());
        // kind sahte değiştirilemez
        let (p, s) = c.trim_start_matches("EO-").split_once('.').unwrap();
        let u = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let pl = String::from_utf8(u.decode(p).unwrap()).unwrap().replace("|K|", "|H|");
        assert!(verify_code(&public, &format!("EO-{}.{}", u.encode(pl), s)).is_err());
    }
}

/// Anahtar çiftini verilen klasöre yazar (gizli dosya 0600); (dosya yolu, açık anahtar) döndürür.
pub fn generate_into(dir: &Path) -> Result<(std::path::PathBuf, String), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (sec, public) = generate();
    let mut path = dir.join("emlak_ofis_anahtar.gizli");
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("emlak_ofis_anahtar_{n}.gizli"));
        n += 1;
    }
    std::fs::write(&path, &sec).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    let _ = std::fs::write(dir.join(path.file_stem().map(|s| format!("{}.acik.txt", s.to_string_lossy())).unwrap_or_else(|| "acik.txt".into())), &public);
    Ok((path, public))
}
