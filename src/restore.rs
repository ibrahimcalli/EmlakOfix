//! Yedekten geri yükleme. Çalışan program veritabanını değiştirmez: program kapanır,
//! bu modül (aynı ikili, `--geri-yukle` kipinde) dosyayı doğrulayıp yerine koyar ve programı yeniden açar.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn result_file() -> PathBuf {
    std::env::temp_dir().join("emlakofis_geri_yukleme_sonuc.txt")
}

/// Açılışta: son geri yükleme sonucunu verir (ve dosyayı siler). (başarılı mı, mesaj)
pub fn take_notice() -> Option<(bool, String)> {
    let p = result_file();
    let t = std::fs::read_to_string(&p).ok()?;
    let _ = std::fs::remove_file(&p);
    let (k, m) = t.split_once('|')?;
    Some((k == "tamam", m.trim().to_string()))
}

/// Yedek dosyasını doğrular ve `data_dir/emlak.db` yerine koyar. Mevcut veri `yedekler/` altına saklanır.
pub fn restore(data_dir: &Path, src: &Path, pw: &str) -> Result<String, String> {
    let raw = std::fs::read(src).map_err(|e| format!("Yedek okunamadı: {e}"))?;
    let plain = if raw.starts_with(b"EOBK1") {
        crate::ext_db::decrypt_bytes(&raw, pw).map_err(|e| format!("Şifre çözülemedi (parola yanlış olabilir): {e}"))?
    } else {
        raw
    };
    if !plain.starts_with(b"SQLite format 3\0") {
        return Err("Bu dosya bir Emlak Ofis yedeği (SQLite) değil.".into());
    }
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let tmp = data_dir.join("emlak_geri_yukleme.tmp");
    std::fs::write(&tmp, &plain).map_err(|e| e.to_string())?;
    let check = (|| -> Result<(), String> {
        let c = rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
        let ic: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        if ic != "ok" {
            return Err(format!("Yedek bozuk: {ic}"));
        }
        let ver: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        if ver > crate::ext_db::DB_VERSION {
            return Err(format!("Bu yedek daha yeni bir program sürümüne ait (şema {ver}); önce programı güncelleyin."));
        }
        c.query_row("SELECT COUNT(*) FROM portfolios", [], |r| r.get::<_, i64>(0)).map_err(|_| "Yedekte portföy tablosu yok.".to_string())?;
        Ok(())
    })();
    if let Err(e) = check {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let db = data_dir.join("emlak.db");
    let mut saved = String::new();
    if db.exists() {
        let dir = data_dir.join("yedekler");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join(format!("emlak_geri_yukleme_oncesi_{}.db", chrono::Local::now().format("%Y%m%d_%H%M%S")));
        // tutarlı kopya (WAL dahil): VACUUM INTO
        let ok = rusqlite::Connection::open(&db)
            .and_then(|c| c.execute("VACUUM INTO ?1", [target.to_string_lossy().to_string()]).map(|_| ()))
            .is_ok();
        if !ok {
            std::fs::copy(&db, &target).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                format!("Mevcut veri saklanamadı, geri yükleme iptal edildi: {e}")
            })?;
        }
        saved = target.display().to_string();
    }
    for ext in ["db-wal", "db-shm"] {
        let _ = std::fs::remove_file(data_dir.join(format!("emlak.{ext}")));
    }
    let _ = std::fs::remove_file(&db);
    std::fs::rename(&tmp, &db).map_err(|e| format!("Yedek yerine konamadı: {e}"))?;
    Ok(saved)
}

/// `emlak-ofis --geri-yukle <dosya> [--veri <klasör>] [--sonra-ac]`; parola EMLAK_YEDEK_PAROLA ortam değişkeninden.
pub fn run_cli(args: &[String]) -> ! {
    let get = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    let src = PathBuf::from(get("--geri-yukle").unwrap_or_default());
    let dir = get("--veri").map(PathBuf::from).unwrap_or_else(crate::app::default_data_dir);
    let reopen = args.iter().any(|a| a == "--sonra-ac");
    if reopen {
        std::thread::sleep(Duration::from_millis(2500)); // programın kapanmasını bekle
    }
    let pw = std::env::var("EMLAK_YEDEK_PAROLA").unwrap_or_default();
    let r = restore(&dir, &src, &pw);
    match &r {
        Ok(saved) => {
            println!("Geri yükleme tamamlandı. Önceki veri: {}", if saved.is_empty() { "(yoktu)" } else { saved });
            let _ = std::fs::write(result_file(), format!("tamam|Yedekten geri yüklendi. Önceki veri saklandı: {saved}"));
        }
        Err(e) => {
            eprintln!("Geri yükleme başarısız: {e}");
            let _ = std::fs::write(result_file(), format!("hata|{e}"));
        }
    }
    if reopen {
        if let Ok(exe) = std::env::current_exe() {
            let _ = crate::updater::spawn_detached(&mut Command::new(exe));
        }
    }
    std::process::exit(if r.is_ok() { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geri_yukleme_dogrular_ve_eskiyi_saklar() {
        let dir = std::env::temp_dir().join(format!("eo_restore_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // "yedek": gerçek şema + bir kayıt
        let bak = dir.join("yedek.db");
        {
            let db = crate::db::Db::open(&bak).unwrap();
            db.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            db.conn.execute("INSERT INTO portfolios(title,creator_id,created_at,updated_at) VALUES('YEDEKTEKI',0,'2026-01-01 00:00:00','2026-01-01 00:00:00')", []).unwrap();
            db.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
        }
        // mevcut veri
        let live = dir.join("veri");
        std::fs::create_dir_all(&live).unwrap();
        {
            let db = crate::db::Db::open(&live.join("emlak.db")).unwrap();
            db.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            db.conn.execute("INSERT INTO portfolios(title,creator_id,created_at,updated_at) VALUES('GUNCEL',0,'2026-02-02 00:00:00','2026-02-02 00:00:00')", []).unwrap();
        }
        let saved = restore(&live, &bak, "").unwrap();
        assert!(saved.contains("geri_yukleme_oncesi"));
        let c = rusqlite::Connection::open(live.join("emlak.db")).unwrap();
        let t: String = c.query_row("SELECT title FROM portfolios", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "YEDEKTEKI");
        let c2 = rusqlite::Connection::open(&saved).unwrap();
        let t2: String = c2.query_row("SELECT title FROM portfolios", [], |r| r.get(0)).unwrap();
        assert_eq!(t2, "GUNCEL");
        // bozuk dosya mevcut veriye dokunmaz
        let junk = dir.join("cop.db");
        std::fs::write(&junk, b"bu bir yedek degil").unwrap();
        assert!(restore(&live, &junk, "").is_err());
        let c3 = rusqlite::Connection::open(live.join("emlak.db")).unwrap();
        let t3: String = c3.query_row("SELECT title FROM portfolios", [], |r| r.get(0)).unwrap();
        assert_eq!(t3, "YEDEKTEKI");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
