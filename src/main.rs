// Windows'ta siyah konsol penceresi açılmasın
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod api;
mod bgimg;
mod totp;
mod monthly;
mod mirror;
mod outbox;
mod offline;
mod ui_ctx;
mod ui_offmap;
mod ui_totp;
mod ui_brand;
mod app;
mod common;
mod db;
mod export;
mod i18n_doc;
mod i18n_ui;
pub mod map_tiles;
mod selftest;
mod sign;
mod tls;
mod ext_db;
mod server;
mod server_ext;
mod web;
mod ui_web;
mod ui_ext;
mod ui_ext2;
mod ui_map;
mod report;
mod theme;
mod restore;
#[cfg(test)]
mod tests_api;
mod updater;
mod ui_settings;
mod ui_admin;
mod ui_auth;
mod ui_form;
mod ui_portfolio;
mod ui_work;

use std::path::PathBuf;

fn arg_value(args: &[String], names: &[&str]) -> Option<String> {
    args.iter().position(|a| names.contains(&a.as_str())).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Güncelleme gözcüsü (kullanıcı görmez): --guncelleme-izle <yeni> <eski> <işaret>
    if args.get(1).map(|a| a == "--guncelleme-izle").unwrap_or(false) && args.len() >= 5 {
        updater::run_watcher(std::path::Path::new(&args[2]), std::path::Path::new(&args[3]), std::path::Path::new(&args[4]));
    }
    // Yeni sürüm doğrulama işareti: --guncelleme-dogrula <işaret>
    if let Some(m) = arg_value(&args, &["--guncelleme-dogrula"]) {
        let _ = app::VERIFY_MARKER.set(PathBuf::from(m));
    }
    // Yedekten geri yükleme: --geri-yukle <dosya> [--veri <klasör>] [--sonra-ac]
    if args.iter().any(|a| a == "--geri-yukle") {
        restore::run_cli(&args);
    }
    // Kurtarma: kullanıcının iki aşamalı doğrulamasını ana bilgisayarda kapatır:
    //   emlak-ofis --2fa-sifirla <kullanıcı adı> [--veri <klasör>]   (program kapalıyken çalıştırın)
    if let Some(user) = arg_value(&args, &["--2fa-sifirla"]) {
        let dir = arg_value(&args, &["--veri"]).map(PathBuf::from).unwrap_or_else(app::default_data_dir);
        let code = match db::Db::open(&dir.join("emlak.db")) {
            Ok(d) => match d.user_login_row(&user) {
                Ok(Some((u, _))) => {
                    let _ = d.kv_set(&format!("totp_on_{}", u.id), "0");
                    let _ = d.kv_set(&format!("totp_{}", u.id), "");
                    d.audit(u.id, &u.full_name(), "İki aşamalı doğrulama sıfırlandı", "ana bilgisayardan (komut satırı)");
                    println!("'{}' kullanıcısının iki aşamalı doğrulaması kapatıldı.", u.username);
                    0
                }
                _ => {
                    eprintln!("Kullanıcı bulunamadı: {user}");
                    2
                }
            },
            Err(e) => {
                eprintln!("Veritabanı açılamadı: {}", e.1);
                1
            }
        };
        std::process::exit(code);
    }
    // Arayüzsüz güncelleme: emlak-ofis --guncelle-paket <paket.zip>
    if let Some(z) = arg_value(&args, &["--guncelle-paket"]) {
        let pkg = match updater::read_package(std::path::Path::new(&z)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Paket hatası: {e}");
                std::process::exit(2);
            }
        };
        if !updater::is_newer(&pkg.version, updater::current_version()) {
            eprintln!("Paket v{} mevcut sürümden (v{}) yeni değil.", pkg.version, updater::current_version());
            std::process::exit(3);
        }
        match pkg.for_this_platform() {
            Some(b) => {
                if let Err(e) = updater::install_and_restart(b) {
                    eprintln!("Kurulamadı: {e}");
                    std::process::exit(4);
                }
            }
            None => {
                eprintln!("Pakette bu platform için dosya yok.");
                std::process::exit(5);
            }
        }
    }
    if let Some(pre) = arg_value(&args, &["--anahtar-uret"]) {
        match sign::cli_generate(&pre) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--kod-uret") {
        let g = |n: usize| args.get(i + n).cloned().unwrap_or_default();
        if args.len() < i + 5 {
            eprintln!("Kullanım: emlak-ofis --kod-uret <gizli-anahtar-dosyası> <H|K> <kurulum-günü> <lisans-günü> [not]\n  H = yönetici, K = kullanıcı; gün 0 = süresiz");
            std::process::exit(1);
        }
        match sign::cli_make_code(&g(1), &g(2), &g(3), &g(4), &g(5)) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--paket-imzala") && args.len() >= 4 {
        let i = args.iter().position(|a| a == "--paket-imzala").unwrap();
        match (args.get(i + 1), args.get(i + 2)) {
            (Some(z), Some(k)) => match sign::cli_sign_package(z, k) {
                Ok(_) => std::process::exit(0),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            },
            _ => {
                eprintln!("Kullanım: emlak-ofis --paket-imzala <paket.zip> <gizli-anahtar-dosyası>");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--surum") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Arayüzsüz sunucu: `emlak-ofis --sunucu [--port 7878] [--veri /yol/klasor]`
    if args.iter().any(|a| a == "--sunucu" || a == "--server") {
        let port: u16 = arg_value(&args, &["--port"]).and_then(|v| v.parse().ok()).unwrap_or(common::DEFAULT_PORT);
        let dir = arg_value(&args, &["--veri", "--data"]).map(PathBuf::from).unwrap_or_else(app::default_data_dir);
        let https = args.iter().any(|a| a == "--https");
        match server::start_with(dir.clone(), port, https) {
            Ok(_) => {
                println!("Emlak Ofis sunucusu çalışıyor.");
                println!("Veri klasörü : {}", dir.display());
                if https {
                    if let Ok(c) = tls::ensure_cert(&dir) {
                        println!("HTTPS açık. Sertifika parmak izi (SHA-256): {}", tls::fp_pretty(&c.fingerprint));
                    }
                }
                for ip in server::local_ips() {
                    println!("Bağlantı adresi: {ip}:{port}");
                }
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
            Err(e) => {
                eprintln!("Sunucu başlatılamadı: {e}");
                std::process::exit(1);
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(common::APP_NAME)
            .with_app_id("emlak-ofis")
            .with_icon(app_icon())
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([700.0, 480.0]),
        ..Default::default()
    };
    bgimg::install_linux_icon();
    eframe::run_native(common::APP_NAME, options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

/// Pencere / görev çubuğu simgesi (assets/icon256.png).
fn app_icon() -> eframe::egui::IconData {
    let bytes = include_bytes!("../assets/icon256.png");
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            eframe::egui::IconData { rgba: rgba.into_raw(), width, height }
        }
        Err(_) => eframe::egui::IconData::default(),
    }
}
