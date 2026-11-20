//! Sürüm 1.2: veritabanı sürümleme (migration), genel sorgu yardımcıları, harici/şifreli yedek.
use crate::common::*;
use crate::db::*;
use rusqlite::types::ValueRef;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Bu program sürümünün beklediği veritabanı şema sürümü.
pub const DB_VERSION: i64 = 2;

const MIGRATION_V2: &str = r#"
CREATE TABLE IF NOT EXISTS audit_log(
  id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL DEFAULT 0, user_name TEXT NOT NULL DEFAULT '',
  action TEXT NOT NULL DEFAULT '', detail TEXT NOT NULL DEFAULT '', at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_audit_at ON audit_log(id);
CREATE TABLE IF NOT EXISTS price_history(
  id INTEGER PRIMARY KEY, portfolio_id INTEGER NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
  old_price REAL NOT NULL DEFAULT 0, new_price REAL NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'TL',
  user_name TEXT NOT NULL DEFAULT '', changed_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_ph_p ON price_history(portfolio_id);
CREATE TABLE IF NOT EXISTS buyers(
  id INTEGER PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '', phone TEXT NOT NULL DEFAULT '', email TEXT NOT NULL DEFAULT '',
  listing_type TEXT NOT NULL DEFAULT 'Satılık', category TEXT NOT NULL DEFAULT '',
  district TEXT NOT NULL DEFAULT '', neighborhood TEXT NOT NULL DEFAULT '',
  budget_min REAL NOT NULL DEFAULT 0, budget_max REAL NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'TL',
  rooms TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '',
  consent INTEGER NOT NULL DEFAULT 0, consent_date TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'Aktif', created_by INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS appointments(
  id INTEGER PRIMARY KEY, title TEXT NOT NULL DEFAULT '', kind TEXT NOT NULL DEFAULT 'Yer gösterme',
  start_at TEXT NOT NULL, duration_min INTEGER NOT NULL DEFAULT 60,
  user_id INTEGER NOT NULL, portfolio_id INTEGER NOT NULL DEFAULT 0, buyer_id INTEGER NOT NULL DEFAULT 0,
  note TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'Planlı', reminded INTEGER NOT NULL DEFAULT 0,
  created_by INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_ap_start ON appointments(start_at);
CREATE TABLE IF NOT EXISTS leaves(
  id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, kind TEXT NOT NULL DEFAULT 'Yıllık izin',
  start_date TEXT NOT NULL, end_date TEXT NOT NULL, note TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'Bekliyor', decided_by INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS announcements(
  id INTEGER PRIMARY KEY, title TEXT NOT NULL DEFAULT '', body TEXT NOT NULL DEFAULT '',
  pinned INTEGER NOT NULL DEFAULT 0, created_by INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS portfolio_notes(
  id INTEGER PRIMARY KEY, portfolio_id INTEGER NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
  user_id INTEGER NOT NULL DEFAULT 0, text TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_pn_p ON portfolio_notes(portfolio_id);
CREATE TABLE IF NOT EXISTS commissions(
  id INTEGER PRIMARY KEY, portfolio_id INTEGER NOT NULL DEFAULT 0, user_id INTEGER NOT NULL DEFAULT 0,
  deal_type TEXT NOT NULL DEFAULT 'Satış', deal_amount REAL NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'TL',
  rate REAL NOT NULL DEFAULT 0, commission REAL NOT NULL DEFAULT 0, emp_pct REAL NOT NULL DEFAULT 0, emp_amount REAL NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'Bekliyor', date TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL);
"#;

fn has_column(conn: &rusqlite::Connection, table: &str, col: &str) -> bool {
    let mut st = match conn.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let names: Vec<String> = st.query_map([], |r| r.get::<_, String>(1)).map(|it| it.filter_map(|x| x.ok()).collect()).unwrap_or_default();
    names.iter().any(|n| n == col)
}

impl Db {
    /// Veritabanını bu programın beklediği sürüme getirir. Eski veriyi korur; önce yedek alır.
    pub fn migrate(&self) -> R<()> {
        let ver: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if ver > DB_VERSION {
            return Err(ApiErr(
                500,
                format!("Bu veritabanı daha yeni bir program sürümüyle oluşturulmuş (şema {ver}). Önce programı güncelleyin."),
            ));
        }
        for col in ["description_en", "description_ru"] {
            if !has_column(&self.conn, "portfolios", col) {
                self.conn.execute_batch(&format!("ALTER TABLE portfolios ADD COLUMN {col} TEXT NOT NULL DEFAULT '';"))?;
            }
        }
        if !has_column(&self.conn, "crm_contacts", "photo") {
            self.conn.execute_batch("ALTER TABLE crm_contacts ADD COLUMN photo BLOB;")?;
        }
        crate::web::ensure_columns(self)?;
        if ver == DB_VERSION {
            return Ok(());
        }
        let users: i64 = self.conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        if users > 0 {
            let dir = self.backup_dir();
            let _ = std::fs::create_dir_all(&dir);
            let target = dir.join(format!("emlak_yukseltme_oncesi_s{}_{}.db", ver, chrono::Local::now().format("%Y%m%d_%H%M%S")));
            let _ = self.conn.execute("VACUUM INTO ?1", params![target.to_string_lossy().to_string()]);
        }
        if ver < 2 {
            self.conn.execute_batch(MIGRATION_V2)?;
            for (t, c, def) in [
                ("crm_contacts", "consent", "INTEGER NOT NULL DEFAULT 0"),
                ("crm_contacts", "consent_date", "TEXT NOT NULL DEFAULT ''"),
                ("crm_contacts", "anonymized", "INTEGER NOT NULL DEFAULT 0"),
            ] {
                if !has_column(&self.conn, t, c) {
                    self.conn.execute_batch(&format!("ALTER TABLE {t} ADD COLUMN {c} {def};"))?;
                }
            }
            // Eski portföylerin ilk fiyatını geçmişe yaz
            self.conn.execute(
                "INSERT INTO price_history(portfolio_id,old_price,new_price,currency,user_name,changed_at) \
                 SELECT p.id,0,p.price,p.currency,'',p.created_at FROM portfolios p WHERE NOT EXISTS (SELECT 1 FROM price_history h WHERE h.portfolio_id=p.id)",
                [],
            )?;
            if !has_column(&self.conn, "portfolios", "match_notified") {
                self.conn.execute_batch("ALTER TABLE portfolios ADD COLUMN match_notified INTEGER NOT NULL DEFAULT 0;")?;
                // mevcut yayındaki portföyler için eşleşme bildirimi tekrar gönderilmesin
                self.conn.execute("UPDATE portfolios SET match_notified=1", [])?;
            }
        }
        self.conn.execute_batch(&format!("PRAGMA user_version={DB_VERSION};"))?;
        Ok(())
    }

    /// Herhangi bir SELECT sonucunu sütun adlarıyla JSON nesneleri olarak döndürür.
    pub fn rows(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> R<Vec<Value>> {
        let mut st = self.conn.prepare(sql)?;
        let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let mut q = st.query(p)?;
        let mut out = Vec::new();
        while let Some(r) = q.next()? {
            let mut m = Map::new();
            for (i, n) in names.iter().enumerate() {
                let v = match r.get_ref(i)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(x) => json!(x),
                    ValueRef::Real(x) => json!(x),
                    ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
                    ValueRef::Blob(_) => Value::Null,
                };
                m.insert(n.clone(), v);
            }
            out.push(Value::Object(m));
        }
        Ok(out)
    }

    /// Aylık ofis raporu için veriler (ym = "YYYY-MM").
    pub fn monthly_report(&self, ym: &str) -> R<serde_json::Value> {
        use serde_json::json;
        if ym.len() != 7 || !ym.chars().enumerate().all(|(i, c)| if i == 4 { c == '-' } else { c.is_ascii_digit() }) {
            return Err(ApiErr::bad("Ay biçimi YYYY-AA olmalı (ör. 2026-11)."));
        }
        let like = format!("{ym}%");
        let c = &self.conn;
        let one = |sql: &str| -> i64 { c.query_row(sql, params![like], |r| r.get(0)).unwrap_or(0) };
        let new_total = one("SELECT COUNT(*) FROM portfolios WHERE created_at LIKE ?1");
        let published = one("SELECT COUNT(*) FROM portfolios WHERE published_at LIKE ?1");
        let sold = one("SELECT COUNT(*) FROM portfolios WHERE closed_at LIKE ?1 AND status='Satıldı'");
        let rented = one("SELECT COUNT(*) FROM portfolios WHERE closed_at LIKE ?1 AND status='Kiralandı'");
        let rows = |sql: &str| -> R<Vec<(String, i64, f64, f64)>> {
            let mut st = c.prepare(sql)?;
            let it = st.query_map(params![like], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?, r.get::<_, f64>(3)?)))?;
            Ok(it.filter_map(|x| x.ok()).collect())
        };
        let by_type: Vec<_> = rows("SELECT listing_type, COUNT(*), 0.0, 0.0 FROM portfolios WHERE created_at LIKE ?1 GROUP BY listing_type ORDER BY 2 DESC")?.into_iter().map(|r| json!({"name": r.0, "n": r.1})).collect();
        let by_district: Vec<_> = rows("SELECT CASE WHEN district='' THEN '(belirtilmemiş)' ELSE district END, COUNT(*), 0.0, 0.0 FROM portfolios WHERE created_at LIKE ?1 GROUP BY 1 ORDER BY 2 DESC LIMIT 8")?.into_iter().map(|r| json!({"name": r.0, "n": r.1})).collect();
        let comm: Vec<_> = rows("SELECT currency, COUNT(*), COALESCE(SUM(deal_amount),0), COALESCE(SUM(commission),0) FROM commissions WHERE date LIKE ?1 GROUP BY currency")?.into_iter().map(|r| json!({"currency": r.0, "n": r.1, "deal": r.2, "commission": r.3})).collect();
        let comm_status: Vec<_> = rows("SELECT status, COUNT(*), COALESCE(SUM(commission),0), 0.0 FROM commissions WHERE date LIKE ?1 GROUP BY status")?.into_iter().map(|r| json!({"status": r.0, "n": r.1, "commission": r.2})).collect();
        let appt: Vec<_> = rows("SELECT status, COUNT(*), 0.0, 0.0 FROM appointments WHERE start_at LIKE ?1 GROUP BY status")?.into_iter().map(|r| json!({"status": r.0, "n": r.1})).collect();
        // çalışan bazlı
        let mut users = Vec::new();
        {
            let mut st = c.prepare("SELECT id, CASE WHEN trim(first_name||last_name)='' THEN username ELSE trim(first_name||' '||last_name) END FROM users WHERE active=1 ORDER BY 2")?;
            let it = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
            for u in it.filter_map(|x| x.ok()) {
                let q = |sql: &str| -> i64 { c.query_row(sql, params![u.0, like], |r| r.get(0)).unwrap_or(0) };
                let newp = q("SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND created_at LIKE ?2");
                let closed = q("SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND closed_at LIKE ?2");
                let appts = q("SELECT COUNT(*) FROM appointments WHERE user_id=?1 AND start_at LIKE ?2");
                let cm: f64 = c.query_row("SELECT COALESCE(SUM(emp_amount),0) FROM commissions WHERE user_id=?1 AND date LIKE ?2", params![u.0, like], |r| r.get(0)).unwrap_or(0.0);
                // mesai
                let mut days = std::collections::HashSet::new();
                let mut secs = 0i64;
                let mut st2 = c.prepare("SELECT check_in, check_out FROM attendance WHERE user_id=?1 AND check_in LIKE ?2")?;
                for (a, b) in st2.query_map(params![u.0, like], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?.filter_map(|x| x.ok()) {
                    days.insert(a.chars().take(10).collect::<String>());
                    let f = "%Y-%m-%d %H:%M:%S";
                    if let (Ok(x), Ok(y)) = (chrono::NaiveDateTime::parse_from_str(&a, f), chrono::NaiveDateTime::parse_from_str(&b, f)) {
                        secs += (y - x).num_seconds().max(0);
                    }
                }
                if newp + closed + appts + days.len() as i64 > 0 || cm > 0.0 {
                    users.push(json!({"name": u.1, "new": newp, "closed": closed, "appointments": appts, "days": days.len(), "hours": (secs as f64 / 360.0).round() / 10.0, "commission": cm}));
                }
            }
        }
        Ok(json!({"ym": ym, "new_total": new_total, "published": published, "sold": sold, "rented": rented,
            "by_type": by_type, "by_district": by_district, "commissions": comm, "commission_status": comm_status,
            "appointments": appt, "users": users}))
    }

    pub fn kv_get(&self, key: &str) -> Option<String> {
        self.conn.query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| r.get(0)).optional().ok().flatten()
    }
    pub fn kv_set(&self, key: &str, val: &str) -> R<()> {
        self.conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", params![key, val])?;
        Ok(())
    }

    pub fn audit(&self, user_id: i64, user_name: &str, action: &str, detail: &str) {
        let _ = self.conn.execute(
            "INSERT INTO audit_log(user_id,user_name,action,detail,at) VALUES(?1,?2,?3,?4,?5)",
            params![user_id, user_name, action, detail, now_str()],
        );
        // son 20.000 kaydı tut
        let _ = self.conn.execute("DELETE FROM audit_log WHERE id < (SELECT COALESCE(MAX(id),0)-20000 FROM audit_log)", []);
    }

    // ---------------- Harici (ve isteğe bağlı şifreli) yedek ----------------
    pub fn external_backup_dir(&self) -> String {
        self.kv_get("ext_backup_dir").unwrap_or_default()
    }

    /// Güvenli yerel yedek klasörü: varsayılan Belgeler/Emlak Ofis Yedek (program kaldırılsa bile silinmez).
    pub fn safe_backup_dir(&self) -> PathBuf {
        let v = self.kv_get("safe_backup_dir").unwrap_or_default();
        if !v.trim().is_empty() {
            return PathBuf::from(v.trim());
        }
        if self.path.starts_with(std::env::temp_dir()) {
            return self.backup_dir().join("guvenli"); // testler gerçek Belgeler klasörünü kirletmesin
        }
        directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
            .or_else(|| directories::UserDirs::new().map(|u| u.home_dir().join("Documents")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Emlak Ofis Yedek")
    }

    /// Tutulacak yedek sayısı (varsayılan 14)
    pub fn backup_keep(&self) -> usize {
        self.kv_get("backup_keep").and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(14).clamp(2, 365)
    }

    /// rclone ile buluta (ör. Google Drive) yükler: şifreli tek dosya, arka planda. Durum dosyaya yazılır.
    pub fn rclone_sync(&self, backup: &Path) -> Result<(), String> {
        let remote = self.kv_get("rclone_remote").unwrap_or_default();
        let remote = remote.trim();
        if remote.is_empty() {
            return Ok(());
        }
        if self.kv_get("ext_backup_pw").unwrap_or_default().is_empty() {
            return Err("Buluta yedek için yedek parolası gerekli (veriler şifrelenmeden buluta gönderilmez).".into());
        }
        let local = self.copy_into(&self.backup_dir().join("bulut"), backup, true)?;
        let status = self.backup_dir().join("rclone_son_durum.txt");
        let dest = format!("{}/{}", remote.trim_end_matches('/'), local.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        std::thread::spawn(move || {
            let r = std::process::Command::new("rclone").arg("copyto").arg(&local).arg(&dest).output();
            let msg = match r {
                Ok(o) if o.status.success() => format!("{} OK -> {}", crate::common::now_str(), dest),
                Ok(o) => format!("{} HATA: {}", crate::common::now_str(), String::from_utf8_lossy(&o.stderr).trim()),
                Err(e) => format!("{} HATA: rclone çalıştırılamadı ({e}). rclone kurulu mu?", crate::common::now_str()),
            };
            let _ = std::fs::write(status, msg);
        });
        Ok(())
    }

    /// Ortak kopyalama: şifre ayarlıysa şifreler. overwrite=true ise tek dosyanın (emlak_son_yedek) üzerine yazar.
    fn copy_into(&self, dir: &Path, backup: &Path, overwrite: bool) -> Result<PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("Yedek klasörü açılamadı: {e}"))?;
        let name = if overwrite { "emlak_son_yedek.db".to_string() } else { backup.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "emlak.db".into()) };
        let pw = self.kv_get("ext_backup_pw").unwrap_or_default();
        let data = std::fs::read(backup).map_err(|e| e.to_string())?;
        let (out_name, bytes) = if pw.is_empty() { (name, data) } else { (format!("{name}.enc"), encrypt_bytes(&data, &pw)?) };
        let target = dir.join(&out_name);
        let tmp = dir.join(format!("{out_name}.tmp"));
        std::fs::write(&tmp, bytes).map_err(|e| format!("Yedek yazılamadı: {e}"))?;
        std::fs::rename(&tmp, &target).map_err(|e| format!("Yedek yazılamadı: {e}"))?;
        if overwrite {
            // parola eklendi/kaldırıldıysa diğer biçimdeki eski tek dosyayı sil
            let other = if out_name.ends_with(".enc") { "emlak_son_yedek.db" } else { "emlak_son_yedek.db.enc" };
            let _ = std::fs::remove_file(dir.join(other));
        } else {
            let mut files: Vec<String> = std::fs::read_dir(dir)
                .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("emlak_") && n != "emlak_son_yedek.db" && (n.ends_with(".db") || n.ends_with(".db.enc"))).collect())
                .unwrap_or_default();
            files.sort();
            while files.len() > self.backup_keep() {
                let old = files.remove(0);
                let _ = std::fs::remove_file(dir.join(old));
            }
        }
        Ok(target)
    }

    /// Yeni alınan yedeği ayarlı harici/bulut klasörüne kopyalar (varsayılan: tek dosyanın üzerine yazar).
    pub fn copy_external(&self, backup: &Path) -> Result<Option<PathBuf>, String> {
        let dir = self.external_backup_dir();
        if dir.trim().is_empty() {
            return Ok(None);
        }
        let overwrite = self.kv_get("ext_backup_overwrite").map(|v| v != "0").unwrap_or(true);
        self.copy_into(Path::new(dir.trim()), backup, overwrite).map(Some)
    }

    /// Güvenli yerel yedek (Belgeler\Emlak Ofis Yedek): zaman damgalı son 30 kopya.
    pub fn copy_safe(&self, backup: &Path) -> Result<PathBuf, String> {
        self.copy_into(&self.safe_backup_dir(), backup, false)
    }
}

const MAGIC: &[u8; 5] = b"EOBK1";

fn derive_key(pw: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    argon2::Argon2::default().hash_password_into(pw.as_bytes(), salt, &mut key).map_err(|e| e.to_string())?;
    Ok(key)
}

pub fn encrypt_bytes(data: &[u8], pw: &str) -> Result<Vec<u8>, String> {
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
    use rand::RngCore;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let key = derive_key(pw, &salt)?;
    let c = ChaCha20Poly1305::new(Key::from_slice(&key));
    let ct = c.encrypt(Nonce::from_slice(&nonce), data).map_err(|_| "Şifreleme hatası".to_string())?;
    let mut out = Vec::with_capacity(5 + 16 + 12 + ct.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt_bytes(data: &[u8], pw: &str) -> Result<Vec<u8>, String> {
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
    if data.len() < 33 || &data[..5] != MAGIC {
        return Err("Bu dosya şifreli bir Emlak Ofis yedeği değil.".into());
    }
    let salt = &data[5..21];
    let nonce = &data[21..33];
    let key = derive_key(pw, salt)?;
    let c = ChaCha20Poly1305::new(Key::from_slice(&key));
    c.decrypt(Nonce::from_slice(nonce), &data[33..]).map_err(|_| "Şifre hatalı ya da dosya bozuk.".to_string())
}
