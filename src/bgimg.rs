//! Giriş ekranı arka planı: hazır resimler, kişisel resim (en az çözünürlük denetimi + otomatik küçültme).
use std::path::{Path, PathBuf};

pub const MIN_W: u32 = 1280;
pub const MIN_H: u32 = 720;
/// Kaydedilen resmin en uzun kenarı (daha büyükler küçültülür).
pub const MAX_SIDE: u32 = 2560;

const SUNSET: &[u8] = include_bytes!("../assets/bg_gunbatimi.jpg");
const LAKE: &[u8] = include_bytes!("../assets/bg_gol_evi.jpg");
pub const LOGO_GOLD: &[u8] = include_bytes!("../assets/logo_altin.png");

pub fn custom_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis").map(|d| d.config_dir().join("arkaplan.jpg"))
}

/// Seçime göre resim baytları (None = düz zemin).
pub fn bytes_for(bg: &str) -> Option<Vec<u8>> {
    match bg {
        "gunbatimi" => Some(SUNSET.to_vec()),
        "gol" => Some(LAKE.to_vec()),
        "ozel" => custom_path().and_then(|p| std::fs::read(p).ok()).or_else(|| Some(SUNSET.to_vec())),
        _ => None,
    }
}

/// Resmi denetler ve gerekirse küçültür. Dönüş: (jpeg baytları, özgün boyut, kaydedilen boyut).
pub fn process(bytes: &[u8]) -> Result<(Vec<u8>, (u32, u32), (u32, u32)), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("Resim açılamadı: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w < MIN_W || h < MIN_H {
        return Err(format!("Resim çok küçük ({w}×{h}). Tam ekranda bulanık görünmemesi için en az {MIN_W}×{MIN_H} olmalı."));
    }
    let img = if w.max(h) > MAX_SIDE { img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Lanczos3) } else { img };
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85).encode_image(&rgb).map_err(|e| e.to_string())?;
    Ok((out, (w, h), (rgb.width(), rgb.height())))
}

/// Seçilen dosyayı denetleyip ayar klasörüne kaydeder.
pub fn import(src: &Path) -> Result<String, String> {
    let bytes = std::fs::read(src).map_err(|e| format!("Dosya okunamadı: {e}"))?;
    let (out, (w, h), (nw, nh)) = process(&bytes)?;
    let dst = custom_path().ok_or("Ayar klasörü bulunamadı")?;
    if let Some(d) = dst.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(&dst, out).map_err(|e| format!("Kaydedilemedi: {e}"))?;
    Ok(if (w, h) == (nw, nh) { format!("Resim kaydedildi ({w}×{h}).") } else { format!("Resim kaydedildi: {w}×{h} → {nw}×{nh} olarak küçültüldü.") })
}

/// Linux: dosya seçme pencerelerinde ve görev çubuğunda program simgesi görünsün diye simge + masaüstü girdisi kurar.
pub fn install_linux_icon() {
    #[cfg(target_os = "linux")]
    {
        let Some(ud) = directories::BaseDirs::new() else { return };
        let data = ud.data_dir().to_path_buf();
        let icon = data.join("icons/hicolor/256x256/apps/emlak-ofis.png");
        if !icon.exists() {
            if let Some(d) = icon.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(&icon, include_bytes!("../assets/icon256.png"));
        }
        let desk = data.join("applications/emlak-ofis.desktop");
        if let Ok(exe) = std::env::current_exe() {
            let want = format!(
                "[Desktop Entry]\nType=Application\nName=Emlak Ofis\nExec={}\nIcon=emlak-ofis\nStartupWMClass=emlak-ofis\nTerminal=false\nCategories=Office;\n",
                exe.display()
            );
            if std::fs::read_to_string(&desk).map(|c| c != want).unwrap_or(true) {
                if let Some(d) = desk.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(&desk, want);
            }
        }
    }
}
