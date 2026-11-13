//! Uygulama içi güncelleme: paket (.zip) okuma, kurma, yeniden başlatma ve
//! açılamazsa otomatik eski sürüme dönme (gözcü süreç).
//!
//! Akış:
//!  1. `install_and_restart` yeni dosyayı yazar, çalışan programı `.eski` adıyla saklar.
//!  2. Eski dosya "gözcü" kipinde (`--guncelleme-izle`) başlatılır ve program kapanır.
//!  3. Gözcü yeni programı başlatır (`--guncelleme-dogrula <işaret>`) ve izler.
//!  4. Yeni program ~10 sn sorunsuz çalışırsa işaret dosyasını yazar → güncelleme kalıcı.
//!     Hata koduyla kapanırsa ya da süre dolarsa gözcü `.eski` dosyayı geri koyar ve açar.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const VERIFY_AFTER: Duration = Duration::from_secs(10);
const WATCH_TIMEOUT: Duration = Duration::from_secs(90);

pub fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn parse_ver(s: &str) -> Option<(u64, u64, u64)> {
    let mut it = s.trim().trim_start_matches('v').split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().ok()?;
    let c = it.next().unwrap_or("0").parse().ok()?;
    Some((a, b, c))
}

/// `a`, `b`'den KESİN olarak daha yeni mi?
pub fn is_newer(a: &str, b: &str) -> bool {
    match (parse_ver(a), parse_ver(b)) {
        (Some(x), Some(y)) => x > y,
        _ => false,
    }
}

#[derive(Deserialize)]
struct Entry {
    dosya: String,
    sha256: String,
    #[serde(default)]
    imza: String,
}
#[derive(Deserialize)]
struct Manifest {
    surum: String,
    #[serde(default)]
    notlar: String,
    paketler: std::collections::BTreeMap<String, Entry>,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub version: String,
    pub notes: String,
    /// (platform, program dosyası)
    pub bins: Vec<(String, Vec<u8>)>,
    /// platform -> imza (base64); yoksa boş
    #[allow(dead_code)]
    pub sigs: std::collections::BTreeMap<String, String>,
    /// Güvenilir yayıncı anahtarıyla tüm dosyaların imzası doğrulandı mı?
    pub signed: bool,
}

static TRUSTED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
pub fn set_trusted_key(k: &str) {
    *TRUSTED.lock().unwrap() = k.trim().to_string();
}
pub fn trusted_key() -> String {
    TRUSTED.lock().unwrap().clone()
}

impl Package {
    pub fn for_this_platform(&self) -> Option<&Vec<u8>> {
        self.bins.iter().find(|b| b.0 == platform()).map(|b| &b.1)
    }
    pub fn platforms(&self) -> Vec<String> {
        self.bins.iter().map(|b| b.0.clone()).collect()
    }
}

/// Güncelleme paketini (.zip) okur ve her dosyanın SHA-256 özetini doğrular.
pub fn read_package(path: &Path) -> Result<Package, String> {
    read_package_with(path, &trusted_key())
}

/// `trusted` doluysa her platform dosyasının geçerli imzası ZORUNLUDUR.
pub fn read_package_with(path: &Path, trusted: &str) -> Result<Package, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("Dosya açılamadı: {e}"))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|_| "Bu bir güncelleme paketi (.zip) değil.".to_string())?;
    let mut mtxt = String::new();
    zip.by_name("surum.json")
        .map_err(|_| "Pakette surum.json yok. Bu, kaynak kod zip'i olabilir; güncelleme paketi seçin (EmlakOfis-Guncelleme-….zip).".to_string())?
        .read_to_string(&mut mtxt)
        .map_err(|e| e.to_string())?;
    let m: Manifest = serde_json::from_str(&mtxt).map_err(|e| format!("surum.json okunamadı: {e}"))?;
    if parse_ver(&m.surum).is_none() {
        return Err(format!("Geçersiz sürüm numarası: {}", m.surum));
    }
    let mut bins = Vec::new();
    let mut sigs = std::collections::BTreeMap::new();
    for (plat, e) in &m.paketler {
        if plat != "linux" && plat != "windows" {
            continue;
        }
        let mut buf = Vec::new();
        zip.by_name(&e.dosya)
            .map_err(|_| format!("Pakette {} dosyası yok.", e.dosya))?
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        let h = format!("{:x}", Sha256::digest(&buf));
        if !h.eq_ignore_ascii_case(e.sha256.trim()) {
            return Err(format!("{} dosyası bozuk (özet uyuşmuyor). Paketi yeniden indirin.", e.dosya));
        }
        if buf.len() < 1_000_000 {
            return Err(format!("{} dosyası beklenenden çok küçük.", e.dosya));
        }
        if !trusted.trim().is_empty() {
            if e.imza.trim().is_empty() {
                return Err(format!("{} imzasız. Bu bilgisayarda güvenilir yayıncı anahtarı tanımlı; yalnızca imzalı paketler kurulur.", e.dosya));
            }
            crate::sign::verify(trusted, &m.surum, plat, &buf, &e.imza)?;
        }
        if !e.imza.is_empty() {
            sigs.insert(plat.clone(), e.imza.clone());
        }
        bins.push((plat.clone(), buf));
    }
    if bins.is_empty() {
        return Err("Pakette program dosyası yok.".into());
    }
    let signed = !trusted.trim().is_empty();
    Ok(Package { version: m.surum, notes: m.notlar, bins, sigs, signed })
}

/// Gözcünün sonucu yazdığı dosya (açılışta "geri alındı" bildirimi için).
pub fn status_file() -> PathBuf {
    std::env::temp_dir().join("emlakofis_guncelleme_durum.txt")
}
pub fn marker_file() -> PathBuf {
    std::env::temp_dir().join("emlakofis_guncelleme_tamam.txt")
}

/// Açılışta: önceki güncelleme geri alındıysa nedenini verir (ve dosyayı siler).
pub fn take_rollback_notice() -> Option<String> {
    let p = status_file();
    let t = std::fs::read_to_string(&p).ok()?;
    let _ = std::fs::remove_file(&p);
    t.strip_prefix("geri|").map(|s| s.trim().to_string())
}

fn restore(exe: &Path, eski: &Path, why: &str) {
    // Windows'ta yeni sürüm tam kapanana kadar kısa bekleyip tekrar dene
    let tmp = exe.with_extension("geri");
    for _ in 0..20 {
        if std::fs::copy(eski, &tmp).is_ok() && std::fs::rename(&tmp, exe).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(exe, std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::write(status_file(), format!("geri|{why}"));
    let _ = spawn_detached(&mut Command::new(exe));
}

pub fn spawn_detached(cmd: &mut Command) -> std::io::Result<std::process::Child> {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0800_0000);
    }
    cmd.spawn()
}

/// Gözcü kipi: `emlak-ofis --guncelleme-izle <yeni exe> <eski kopya> <işaret dosyası>`
pub fn run_watcher(exe: &Path, eski: &Path, marker: &Path) -> ! {
    let _ = std::fs::remove_file(marker);
    let _ = std::fs::remove_file(status_file());
    let mut child = match Command::new(exe).arg("--guncelleme-dogrula").arg(marker).stdin(Stdio::null()).spawn() {
        Ok(c) => c,
        Err(e) => {
            restore(exe, eski, &format!("yeni sürüm başlatılamadı: {e}"));
            std::process::exit(0);
        }
    };
    let t0 = Instant::now();
    loop {
        if marker.exists() {
            let _ = std::fs::remove_file(marker);
            std::process::exit(0); // güncelleme kalıcı; .eski dosya "önceki sürüm" olarak kalır
        }
        match child.try_wait() {
            Ok(Some(st)) => {
                if st.success() {
                    std::process::exit(0); // kullanıcı programı normal kapattı
                }
                restore(exe, eski, &format!("yeni sürüm hata koduyla kapandı ({st})"));
                std::process::exit(0);
            }
            Ok(None) => {}
            Err(_) => {}
        }
        if t0.elapsed() > WATCH_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            restore(exe, eski, "yeni sürüm zamanında doğrulanamadı");
            std::process::exit(0);
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Yeni programı kurar, gözcüyü başlatır ve bu programı kapatır. Başarıda geri DÖNMEZ.
pub fn install_and_restart(new_bytes: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let yeni = exe.with_extension("yeni");
    let eski = exe.with_extension("eski");
    std::fs::write(&yeni, new_bytes).map_err(|e| format!("Program klasörüne yazılamadı ({}): {e}. Programı kullanıcı klasörüne taşıyıp yeniden deneyin.", yeni.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&yeni, std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::remove_file(&eski);
    std::fs::rename(&exe, &eski).map_err(|e| format!("Mevcut program yedeklenemedi: {e}"))?;
    if let Err(e) = std::fs::rename(&yeni, &exe) {
        let _ = std::fs::rename(&eski, &exe);
        return Err(format!("Yeni program yerleştirilemedi: {e}"));
    }
    let marker = marker_file();
    let _ = std::fs::remove_file(&marker);
    let w = spawn_detached(Command::new(&eski).arg("--guncelleme-izle").arg(&exe).arg(&eski).arg(&marker));
    if let Err(e) = w {
        let _ = std::fs::remove_file(&exe);
        let _ = std::fs::rename(&eski, &exe);
        return Err(format!("Güncelleme gözcüsü başlatılamadı: {e}"));
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn surum_karsilastirma() {
        assert!(is_newer("1.2.1", "1.2.0"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
        assert!(!is_newer("abc", "1.2.0"));
    }

    fn make_zip(path: &Path, data: &[u8], good_hash: bool) {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        let h = if good_hash { format!("{:x}", Sha256::digest(data)) } else { "00".into() };
        z.start_file("surum.json", o).unwrap();
        z.write_all(format!("{{\"surum\":\"9.9.9\",\"notlar\":\"x\",\"paketler\":{{\"linux\":{{\"dosya\":\"linux/emlak-ofis\",\"sha256\":\"{h}\"}}}}}}").as_bytes()).unwrap();
        z.start_file("linux/emlak-ofis", o).unwrap();
        z.write_all(data).unwrap();
        z.finish().unwrap();
    }

    #[test]
    fn paket_okuma_ve_ozet_dogrulama() {
        let dir = std::env::temp_dir().join(format!("eo_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = vec![7u8; 1_200_000];
        let ok = dir.join("ok.zip");
        make_zip(&ok, &data, true);
        let p = read_package(&ok).unwrap();
        assert_eq!(p.version, "9.9.9");
        assert_eq!(p.bins[0].1.len(), data.len());
        let bad = dir.join("bad.zip");
        make_zip(&bad, &data, false);
        assert!(read_package(&bad).unwrap_err().contains("bozuk"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
