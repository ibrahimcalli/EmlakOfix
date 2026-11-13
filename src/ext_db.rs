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

    /// Yeni alınan yedeği ayarlı harici klasöre kopyalar; şifre ayarlıysa şifreleyerek (.db.enc).
    pub fn copy_external(&self, backup: &Path) -> Result<Option<PathBuf>, String> {
        let dir = self.external_backup_dir();
        if dir.trim().is_empty() {
            return Ok(None);
        }
        let dir = PathBuf::from(dir.trim());
        std::fs::create_dir_all(&dir).map_err(|e| format!("Harici yedek klasörü açılamadı: {e}"))?;
        let name = backup.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "emlak.db".into());
        let pw = self.kv_get("ext_backup_pw").unwrap_or_default();
        let data = std::fs::read(backup).map_err(|e| e.to_string())?;
        let (out_name, bytes) = if pw.is_empty() { (name, data) } else { (format!("{name}.enc"), encrypt_bytes(&data, &pw)?) };
        let target = dir.join(&out_name);
        std::fs::write(&target, bytes).map_err(|e| format!("Harici yedek yazılamadı: {e}"))?;
        // harici klasörde de son 30 yedeği tut
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("emlak_") && (n.ends_with(".db") || n.ends_with(".db.enc"))).collect())
            .unwrap_or_default();
        files.sort();
        while files.len() > 30 {
            let old = files.remove(0);
            let _ = std::fs::remove_file(dir.join(old));
        }
        Ok(Some(target))
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
