//! SQLite veritabanı katmanı (yalnızca sunucu tarafında kullanılır).
use crate::common::*;
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ApiErr(pub u16, pub String);

impl ApiErr {
    pub fn bad(m: impl Into<String>) -> Self {
        ApiErr(400, m.into())
    }
    pub fn forbidden() -> Self {
        ApiErr(403, "Bu işlem için yetkiniz yok.".into())
    }
    pub fn not_found() -> Self {
        ApiErr(404, "Kayıt bulunamadı.".into())
    }
    pub fn unauthorized() -> Self {
        ApiErr(401, "Oturum süresi doldu".into())
    }
}
impl From<rusqlite::Error> for ApiErr {
    fn from(e: rusqlite::Error) -> Self {
        ApiErr(500, format!("Veritabanı hatası: {e}"))
    }
}
pub type R<T> = Result<T, ApiErr>;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users(
  id INTEGER PRIMARY KEY, username TEXT NOT NULL UNIQUE COLLATE NOCASE, pass_hash TEXT NOT NULL,
  first_name TEXT NOT NULL DEFAULT '', last_name TEXT NOT NULL DEFAULT '',
  phone TEXT NOT NULL DEFAULT '', email TEXT NOT NULL DEFAULT '',
  role TEXT NOT NULL DEFAULT 'employee', active INTEGER NOT NULL DEFAULT 1,
  photo BLOB, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS districts(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE IF NOT EXISTS neighborhoods(
  id INTEGER PRIMARY KEY, district_id INTEGER NOT NULL REFERENCES districts(id) ON DELETE CASCADE,
  name TEXT NOT NULL, UNIQUE(district_id, name));
CREATE TABLE IF NOT EXISTS crm_contacts(
  id INTEGER PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '', phone TEXT NOT NULL DEFAULT '',
  email TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '',
  created_by INTEGER, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS portfolios(
  id INTEGER PRIMARY KEY, title TEXT NOT NULL, listing_type TEXT NOT NULL DEFAULT 'Satılık',
  category TEXT NOT NULL DEFAULT '', district TEXT NOT NULL DEFAULT '', neighborhood TEXT NOT NULL DEFAULT '',
  address TEXT NOT NULL DEFAULT '', lat REAL, lon REAL,
  price REAL NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'TL',
  gross_m2 REAL NOT NULL DEFAULT 0, net_m2 REAL NOT NULL DEFAULT 0,
  rooms TEXT NOT NULL DEFAULT '', building_age TEXT NOT NULL DEFAULT '',
  floor TEXT NOT NULL DEFAULT '', total_floors TEXT NOT NULL DEFAULT '',
  heating TEXT NOT NULL DEFAULT '', deed_status TEXT NOT NULL DEFAULT '',
  furnished INTEGER NOT NULL DEFAULT 0, swap_ok INTEGER NOT NULL DEFAULT 0, credit_ok INTEGER NOT NULL DEFAULT 0,
  description TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'Taslak',
  crm_id INTEGER REFERENCES crm_contacts(id) ON DELETE SET NULL,
  creator_id INTEGER NOT NULL REFERENCES users(id),
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  published_at TEXT NOT NULL DEFAULT '', closed_at TEXT NOT NULL DEFAULT '');
CREATE INDEX IF NOT EXISTS idx_p_created ON portfolios(created_at);
CREATE INDEX IF NOT EXISTS idx_p_loc ON portfolios(district, neighborhood);
CREATE TABLE IF NOT EXISTS photos(
  id INTEGER PRIMARY KEY, portfolio_id INTEGER NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
  position INTEGER NOT NULL DEFAULT 0, full BLOB NOT NULL, thumb BLOB NOT NULL);
CREATE INDEX IF NOT EXISTS idx_photo_p ON photos(portfolio_id, position);
CREATE TABLE IF NOT EXISTS tasks(
  id INTEGER PRIMARY KEY, title TEXT NOT NULL, details TEXT NOT NULL DEFAULT '', kind TEXT NOT NULL DEFAULT 'Görev',
  assigned_to INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  due_at TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'Açık',
  created_by INTEGER, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS notifications(
  id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  kind TEXT NOT NULL DEFAULT '', message TEXT NOT NULL, created_at TEXT NOT NULL, is_read INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS idx_n_user ON notifications(user_id, is_read);
CREATE TABLE IF NOT EXISTS attendance(
  id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  check_in TEXT NOT NULL, check_out TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '');
CREATE INDEX IF NOT EXISTS idx_att ON attendance(user_id, check_in);
CREATE TABLE IF NOT EXISTS goals(
  id INTEGER PRIMARY KEY, title TEXT NOT NULL, user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
  period_start TEXT NOT NULL, period_end TEXT NOT NULL,
  t_portfolio INTEGER NOT NULL DEFAULT 0, t_published INTEGER NOT NULL DEFAULT 0,
  t_rented INTEGER NOT NULL DEFAULT 0, t_sold INTEGER NOT NULL DEFAULT 0,
  note TEXT NOT NULL DEFAULT '', created_by INTEGER, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS blobs(key TEXT PRIMARY KEY, data BLOB NOT NULL);
"#;

const USER_COLS: &str = "id,username,first_name,last_name,phone,email,role,active,(photo IS NOT NULL)";
const CREATOR_NAME: &str = "(SELECT CASE WHEN trim(u.first_name||u.last_name)='' THEN u.username ELSE trim(u.first_name||' '||u.last_name) END FROM users u WHERE u.id=p.creator_id)";
const PCOLS: &str = "p.id,p.title,p.listing_type,p.category,p.district,p.neighborhood,p.address,p.lat,p.lon,p.price,p.currency,p.gross_m2,p.net_m2,p.rooms,p.building_age,p.floor,p.total_floors,p.heating,p.deed_status,p.furnished,p.swap_ok,p.credit_ok,p.description,p.status,p.crm_id,p.creator_id,p.created_at,p.updated_at,p.published_at,p.closed_at";

pub fn user_name_sql(col: &str) -> String {
    format!("(SELECT CASE WHEN trim(x.first_name||x.last_name)='' THEN x.username ELSE trim(x.first_name||' '||x.last_name) END FROM users x WHERE x.id={col})")
}

pub struct Db {
    pub conn: Connection,
    pub path: PathBuf,
}

pub fn hash_password(pw: &str) -> R<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiErr(500, format!("Şifre işlenemedi: {e}")))
}

pub fn verify_password(pw: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(h) => Argon2::default().verify_password(pw.as_bytes(), &h).is_ok(),
        Err(_) => false,
    }
}

fn row_user(r: &Row) -> rusqlite::Result<UserInfo> {
    Ok(UserInfo {
        id: r.get(0)?,
        username: r.get(1)?,
        first_name: r.get(2)?,
        last_name: r.get(3)?,
        phone: r.get(4)?,
        email: r.get(5)?,
        role: r.get(6)?,
        active: r.get::<_, i64>(7)? != 0,
        has_photo: r.get::<_, i64>(8)? != 0,
    })
}

fn row_portfolio(r: &Row) -> rusqlite::Result<Portfolio> {
    Ok(Portfolio {
        id: r.get(0)?,
        title: r.get(1)?,
        listing_type: r.get(2)?,
        category: r.get(3)?,
        district: r.get(4)?,
        neighborhood: r.get(5)?,
        address: r.get(6)?,
        lat: r.get(7)?,
        lon: r.get(8)?,
        price: r.get(9)?,
        currency: r.get(10)?,
        gross_m2: r.get(11)?,
        net_m2: r.get(12)?,
        rooms: r.get(13)?,
        building_age: r.get(14)?,
        floor: r.get(15)?,
        total_floors: r.get(16)?,
        heating: r.get(17)?,
        deed_status: r.get(18)?,
        furnished: r.get::<_, i64>(19)? != 0,
        swap_ok: r.get::<_, i64>(20)? != 0,
        credit_ok: r.get::<_, i64>(21)? != 0,
        description: r.get(22)?,
        status: r.get(23)?,
        crm_id: r.get(24)?,
        creator_id: r.get(25)?,
        created_at: r.get(26)?,
        updated_at: r.get(27)?,
        published_at: r.get(28)?,
        closed_at: r.get(29)?,
        ..Default::default()
    })
}

impl Db {
    pub fn open(path: &Path) -> R<Db> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch(SCHEMA)?;
        let db = Db { conn, path: path.to_path_buf() };
        db.migrate()?;
        db.seed_locations()?;
        Ok(db)
    }

    // ---------------- Konumlar ----------------
    fn seed_locations(&self) -> R<()> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM districts", [], |r| r.get(0))?;
        if n == 0 {
            self.import_locations(include_str!("../data/mugla_mahalle.csv"))?;
        }
        Ok(())
    }

    /// "Ilce;Mahalle" satırları
    pub fn import_locations(&self, csv: &str) -> R<usize> {
        let mut count = 0;
        let tx = self.conn.unchecked_transaction()?;
        for line in csv.lines() {
            let line = line.trim().trim_start_matches('\u{feff}');
            if line.is_empty() {
                continue;
            }
            let mut it = line.splitn(2, |c| c == ';' || c == '\t');
            let d = it.next().unwrap_or("").trim();
            let n = it.next().unwrap_or("").trim();
            if d.is_empty() {
                continue;
            }
            tx.execute("INSERT OR IGNORE INTO districts(name) VALUES(?1)", params![d])?;
            if !n.is_empty() {
                let did: i64 = tx.query_row("SELECT id FROM districts WHERE name=?1", params![d], |r| r.get(0))?;
                count += tx.execute("INSERT OR IGNORE INTO neighborhoods(district_id,name) VALUES(?1,?2)", params![did, n])?;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    pub fn list_locations(&self) -> R<Vec<District>> {
        let mut out: Vec<District> = Vec::new();
        let mut st = self.conn.prepare("SELECT id,name FROM districts ORDER BY name COLLATE NOCASE")?;
        let ds = st.query_map([], |r| Ok(District { id: r.get(0)?, name: r.get(1)?, neighborhoods: vec![] }))?;
        for d in ds {
            out.push(d?);
        }
        let mut st = self.conn.prepare("SELECT id,district_id,name FROM neighborhoods ORDER BY name COLLATE NOCASE")?;
        let ns = st.query_map([], |r| Ok((r.get::<_, i64>(1)?, Neighborhood { id: r.get(0)?, name: r.get(2)? })))?;
        for n in ns {
            let (did, nb) = n?;
            if let Some(d) = out.iter_mut().find(|d| d.id == did) {
                d.neighborhoods.push(nb);
            }
        }
        // Türkçe harf sırası için basit düzeltme
        for d in out.iter_mut() {
            d.neighborhoods.sort_by_key(|n| tr_key(&n.name));
        }
        out.sort_by_key(|d| tr_key(&d.name));
        Ok(out)
    }

    pub fn add_district(&self, name: &str) -> R<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiErr::bad("İlçe adı boş olamaz."));
        }
        self.conn.execute("INSERT OR IGNORE INTO districts(name) VALUES(?1)", params![name])?;
        Ok(())
    }
    pub fn delete_district(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM districts WHERE id=?1", params![id])?;
        Ok(())
    }
    pub fn add_neighborhood(&self, district_id: i64, name: &str) -> R<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiErr::bad("Mahalle adı boş olamaz."));
        }
        self.conn.execute("INSERT OR IGNORE INTO neighborhoods(district_id,name) VALUES(?1,?2)", params![district_id, name])?;
        Ok(())
    }
    pub fn delete_neighborhood(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM neighborhoods WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---------------- Kullanıcılar ----------------
    pub fn count_users(&self) -> R<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?)
    }

    pub fn user_by_id(&self, id: i64) -> R<Option<UserInfo>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {USER_COLS} FROM users WHERE id=?1"), params![id], row_user)
            .optional()?)
    }

    pub fn user_login_row(&self, username: &str) -> R<Option<(UserInfo, String)>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {USER_COLS},pass_hash FROM users WHERE username=?1"),
                params![username.trim()],
                |r| Ok((row_user(r)?, r.get::<_, String>(9)?)),
            )
            .optional()?)
    }

    pub fn list_users(&self) -> R<Vec<UserInfo>> {
        let mut st = self.conn.prepare(&format!("SELECT {USER_COLS} FROM users ORDER BY active DESC, first_name COLLATE NOCASE, username COLLATE NOCASE"))?;
        let rows = st.query_map([], row_user)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create_user(&self, username: &str, password: &str, first: &str, last: &str, role: &str) -> R<i64> {
        let username = username.trim();
        if username.len() < 3 {
            return Err(ApiErr::bad("Kullanıcı adı en az 3 karakter olmalı."));
        }
        if username.contains(char::is_whitespace) {
            return Err(ApiErr::bad("Kullanıcı adında boşluk olamaz."));
        }
        if password.len() < 6 {
            return Err(ApiErr::bad("Şifre en az 6 karakter olmalı."));
        }
        let exists: i64 = self.conn.query_row("SELECT COUNT(*) FROM users WHERE username=?1", params![username], |r| r.get(0))?;
        if exists > 0 {
            return Err(ApiErr::bad("Bu kullanıcı adı zaten kullanılıyor."));
        }
        let role = match role { "admin" => "admin", "manager" => "manager", _ => "employee" };
        let h = hash_password(password)?;
        self.conn.execute(
            "INSERT INTO users(username,pass_hash,first_name,last_name,role,active,created_at) VALUES(?1,?2,?3,?4,?5,1,?6)",
            params![username, h, first.trim(), last.trim(), role, now_str()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_profile(&self, id: i64, first: &str, last: &str, phone: &str, email: &str) -> R<()> {
        self.conn.execute(
            "UPDATE users SET first_name=?2,last_name=?3,phone=?4,email=?5 WHERE id=?1",
            params![id, first.trim(), last.trim(), phone.trim(), email.trim()],
        )?;
        Ok(())
    }

    pub fn set_password(&self, id: i64, password: &str) -> R<()> {
        if password.len() < 6 {
            return Err(ApiErr::bad("Şifre en az 6 karakter olmalı."));
        }
        let h = hash_password(password)?;
        self.conn.execute("UPDATE users SET pass_hash=?2 WHERE id=?1", params![id, h])?;
        Ok(())
    }

    pub fn check_password(&self, id: i64, password: &str) -> R<bool> {
        let h: String = self.conn.query_row("SELECT pass_hash FROM users WHERE id=?1", params![id], |r| r.get(0))?;
        Ok(verify_password(password, &h))
    }

    pub fn set_user_photo(&self, id: i64, data: Option<Vec<u8>>) -> R<()> {
        self.conn.execute("UPDATE users SET photo=?2 WHERE id=?1", params![id, data])?;
        Ok(())
    }
    pub fn user_photo(&self, id: i64) -> R<Option<Vec<u8>>> {
        Ok(self
            .conn
            .query_row("SELECT photo FROM users WHERE id=?1", params![id], |r| r.get::<_, Option<Vec<u8>>>(0))
            .optional()?
            .flatten())
    }

    pub fn admin_update_user(&self, id: i64, first: &str, last: &str, role: &str, active: bool) -> R<()> {
        let role = match role { "admin" => "admin", "manager" => "manager", _ => "employee" };
        let admins_other: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE role='admin' AND active=1 AND id<>?1",
            params![id],
            |r| r.get(0),
        )?;
        if admins_other == 0 && (role != "admin" || !active) {
            return Err(ApiErr::bad("Sistemde en az bir aktif yönetici kalmalı."));
        }
        self.conn.execute(
            "UPDATE users SET first_name=?2,last_name=?3,role=?4,active=?5 WHERE id=?1",
            params![id, first.trim(), last.trim(), role, active as i64],
        )?;
        Ok(())
    }

    fn active_user_ids(&self) -> R<Vec<i64>> {
        let mut st = self.conn.prepare("SELECT id FROM users WHERE active=1")?;
        let v = st.query_map([], |r| r.get(0))?.collect::<Result<Vec<i64>, _>>()?;
        Ok(v)
    }

    // ---------------- Bildirimler ----------------
    pub fn notify(&self, user_id: i64, kind: &str, message: &str) -> R<()> {
        self.conn.execute(
            "INSERT INTO notifications(user_id,kind,message,created_at,is_read) VALUES(?1,?2,?3,?4,0)",
            params![user_id, kind, message, now_str()],
        )?;
        Ok(())
    }
    pub fn notify_all_except(&self, except: i64, kind: &str, message: &str) -> R<()> {
        for id in self.active_user_ids()? {
            if id != except {
                self.notify(id, kind, message)?;
            }
        }
        Ok(())
    }
    pub fn list_notifications(&self, user_id: i64) -> R<Vec<NotifInfo>> {
        let mut st = self.conn.prepare(
            "SELECT id,kind,message,created_at,is_read FROM notifications WHERE user_id=?1 ORDER BY id DESC LIMIT 100",
        )?;
        let rows = st.query_map(params![user_id], |r| {
            Ok(NotifInfo { id: r.get(0)?, kind: r.get(1)?, message: r.get(2)?, created_at: r.get(3)?, read: r.get::<_, i64>(4)? != 0 })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn unread_count(&self, user_id: i64) -> R<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM notifications WHERE user_id=?1 AND is_read=0", params![user_id], |r| r.get(0))?)
    }
    pub fn mark_all_read(&self, user_id: i64) -> R<()> {
        self.conn.execute("UPDATE notifications SET is_read=1 WHERE user_id=?1", params![user_id])?;
        // eski kayıtları temizle
        self.conn.execute(
            "DELETE FROM notifications WHERE user_id=?1 AND is_read=1 AND id NOT IN (SELECT id FROM notifications WHERE user_id=?1 ORDER BY id DESC LIMIT 200)",
            params![user_id],
        )?;
        Ok(())
    }

    // ---------------- CRM ----------------
    fn upsert_contact(&self, user_id: i64, name: &str, phone: &str, email: &str, note: &str) -> R<Option<i64>> {
        let name = name.trim();
        let phone = phone.trim();
        if name.is_empty() && phone.is_empty() && email.trim().is_empty() {
            return Ok(None);
        }
        let digits = only_digits(phone);
        let mut found: Option<i64> = None;
        if digits.len() >= 7 {
            let mut st = self.conn.prepare("SELECT id,phone FROM crm_contacts WHERE phone<>''")?;
            let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
            let tail = &digits[digits.len() - 7..];
            for r in rows {
                let (id, ph) = r?;
                if only_digits(&ph).ends_with(tail) {
                    found = Some(id);
                    break;
                }
            }
        }
        if found.is_none() && digits.is_empty() && !name.is_empty() {
            found = self
                .conn
                .query_row("SELECT id FROM crm_contacts WHERE full_name=?1 COLLATE NOCASE AND phone=''", params![name], |r| r.get(0))
                .optional()?;
        }
        if let Some(id) = found {
            // Boş alanları doldur, mevcut bilgileri koru
            self.conn.execute(
                "UPDATE crm_contacts SET full_name=CASE WHEN full_name='' THEN ?2 ELSE full_name END, email=CASE WHEN email='' THEN ?3 ELSE email END, note=CASE WHEN ?4<>'' AND instr(note,?4)=0 THEN trim(note||char(10)||?4) ELSE note END WHERE id=?1",
                params![id, name, email.trim(), note.trim()],
            )?;
            return Ok(Some(id));
        }
        self.conn.execute(
            "INSERT INTO crm_contacts(full_name,phone,email,note,created_by,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![name, phone, email.trim(), note.trim(), user_id, now_str()],
        )?;
        Ok(Some(self.conn.last_insert_rowid()))
    }

    pub fn list_contacts(&self) -> R<Vec<CrmContact>> {
        let sql = format!(
            "SELECT id,full_name,phone,email,note,created_at,{},consent,consent_date,anonymized FROM crm_contacts c ORDER BY created_at DESC, id DESC",
            user_name_sql("c.created_by")
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map([], |r| {
            Ok(CrmContact {
                id: r.get(0)?,
                full_name: r.get(1)?,
                phone: r.get(2)?,
                email: r.get(3)?,
                note: r.get(4)?,
                created_at: r.get(5)?,
                created_by_name: r.get::<_, Option<String>>(6)?.unwrap_or_default(),
                portfolios: vec![],
                consent: r.get::<_, i64>(7)? != 0,
                consent_date: r.get(8)?,
                anonymized: r.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut out: Vec<CrmContact> = rows.collect::<Result<Vec<_>, _>>()?;
        let mut st = self.conn.prepare("SELECT crm_id,id,title,status,district,neighborhood FROM portfolios WHERE crm_id IS NOT NULL ORDER BY created_at DESC")?;
        let refs = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                CrmPortfolioRef { id: r.get(1)?, title: r.get(2)?, status: r.get(3)?, district: r.get(4)?, neighborhood: r.get(5)? },
            ))
        })?;
        for x in refs {
            let (cid, pr) = x?;
            if let Some(c) = out.iter_mut().find(|c| c.id == cid) {
                c.portfolios.push(pr);
            }
        }
        Ok(out)
    }

    pub fn update_contact(&self, id: i64, name: &str, phone: &str, email: &str, note: &str) -> R<()> {
        self.conn.execute(
            "UPDATE crm_contacts SET full_name=?2,phone=?3,email=?4,note=?5 WHERE id=?1",
            params![id, name.trim(), phone.trim(), email.trim(), note.trim()],
        )?;
        Ok(())
    }
    pub fn delete_contact(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM crm_contacts WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---------------- Portföyler ----------------
    pub fn list_portfolios(
        &self,
        q: &std::collections::HashMap<String, String>,
        me: &UserInfo,
    ) -> R<Vec<PortfolioSummary>> {
        let mut sql = format!(
            "SELECT p.id,p.title,p.listing_type,p.category,p.district,p.neighborhood,p.price,p.currency,p.gross_m2,p.rooms,p.status,p.creator_id,{CREATOR_NAME},p.created_at,\
             (SELECT id FROM photos WHERE portfolio_id=p.id ORDER BY position,id LIMIT 1),\
             (SELECT COUNT(*) FROM photos WHERE portfolio_id=p.id) FROM portfolios p WHERE 1=1"
        );
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        let mut add = |sql: &mut String, cond: &str, v: rusqlite::types::Value| {
            args.push(v);
            sql.push_str(&format!(" AND {}", cond.replace('?', &format!("?{}", args.len()))));
        };
        for (key, col) in [("district", "p.district"), ("neighborhood", "p.neighborhood"), ("type", "p.listing_type"), ("status", "p.status"), ("category", "p.category")] {
            if let Some(v) = q.get(key) {
                if !v.is_empty() {
                    add(&mut sql, &format!("{col}=?"), v.clone().into());
                }
            }
        }
        if q.get("mine").map(|v| v == "1").unwrap_or(false) {
            add(&mut sql, "p.creator_id=?", me.id.into());
        }
        if q.get("stale").map(|v| v == "1").unwrap_or(false) {
            let cutoff = (chrono::Local::now() - chrono::Duration::days(60)).format("%Y-%m-%d").to_string();
            args.push(cutoff.into());
            sql.push_str(&format!(" AND p.status='Yayında' AND p.published_at<>'' AND p.published_at<?{}", args.len()));
        }
        if let Some(t) = q.get("q") {
            let t = t.trim();
            if !t.is_empty() {
                let like = format!("%{}%", t);
                args.push(like.into());
                let n = args.len();
                sql.push_str(&format!(
                    " AND (p.title LIKE ?{n} OR p.address LIKE ?{n} OR p.neighborhood LIKE ?{n} OR p.district LIKE ?{n} OR p.description LIKE ?{n} OR ('P-'||substr('00000'||p.id,-5,5)) LIKE ?{n})"
                ));
            }
        }
        sql.push_str(" ORDER BY p.created_at DESC, p.id DESC");
        let lim: i64 = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(5000);
        sql.push_str(&format!(" LIMIT {}", lim.clamp(1, 20000)));
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(PortfolioSummary {
                id: r.get(0)?,
                title: r.get(1)?,
                listing_type: r.get(2)?,
                category: r.get(3)?,
                district: r.get(4)?,
                neighborhood: r.get(5)?,
                price: r.get(6)?,
                currency: r.get(7)?,
                gross_m2: r.get(8)?,
                rooms: r.get(9)?,
                status: r.get(10)?,
                creator_id: r.get(11)?,
                creator_name: r.get::<_, Option<String>>(12)?.unwrap_or_default(),
                created_at: r.get(13)?,
                cover_photo: r.get(14)?,
                photo_count: r.get(15)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_portfolio(&self, id: i64, me: &UserInfo) -> R<Portfolio> {
        let sql = format!("SELECT {PCOLS},{CREATOR_NAME} FROM portfolios p WHERE p.id=?1");
        let mut p = self
            .conn
            .query_row(&sql, params![id], |r| {
                let mut p = row_portfolio(r)?;
                p.creator_name = r.get::<_, Option<String>>(30)?.unwrap_or_default();
                Ok(p)
            })
            .optional()?
            .ok_or_else(ApiErr::not_found)?;
        let mut st = self.conn.prepare("SELECT id FROM photos WHERE portfolio_id=?1 ORDER BY position,id")?;
        p.photos = st.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<i64>, _>>()?;
        if me.is_admin() {
            if let Some(cid) = p.crm_id {
                if let Some((n, ph, em, no)) = self
                    .conn
                    .query_row("SELECT full_name,phone,email,note FROM crm_contacts WHERE id=?1", params![cid], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
                    })
                    .optional()?
                {
                    p.owner_name = n;
                    p.owner_phone = ph;
                    p.owner_email = em;
                    p.owner_note = no;
                }
            }
        } else {
            p.crm_id = None;
        }
        Ok(p)
    }

    pub fn portfolio_creator(&self, id: i64) -> R<i64> {
        self.conn
            .query_row("SELECT creator_id FROM portfolios WHERE id=?1", params![id], |r| r.get(0))
            .optional()?
            .ok_or_else(ApiErr::not_found)
    }

    fn status_times(old_status: &str, old_pub: &str, old_closed: &str, new: &str) -> (String, String) {
        let now = now_str();
        let mut published = old_pub.to_string();
        let mut closed = old_closed.to_string();
        if new == "Yayında" && published.is_empty() {
            published = now.clone();
        }
        if new == "Satıldı" || new == "Kiralandı" {
            if old_status != new || closed.is_empty() {
                closed = now;
            }
        } else {
            closed = String::new();
        }
        (published, closed)
    }

    pub fn create_portfolio(&self, me: &UserInfo, p: &Portfolio) -> R<i64> {
        if p.title.trim().is_empty() {
            return Err(ApiErr::bad("Portföy başlığı boş olamaz."));
        }
        let status = if STATUSES.contains(&p.status.as_str()) { p.status.clone() } else { "Taslak".to_string() };
        let (published, closed) = Self::status_times("", "", "", &status);
        let crm = self.upsert_contact(me.id, &p.owner_name, &p.owner_phone, &p.owner_email, &p.owner_note)?;
        let now = now_str();
        self.conn.execute(
            "INSERT INTO portfolios(title,listing_type,category,district,neighborhood,address,lat,lon,price,currency,gross_m2,net_m2,rooms,building_age,floor,total_floors,heating,deed_status,furnished,swap_ok,credit_ok,description,status,crm_id,creator_id,created_at,updated_at,published_at,closed_at) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?26,?27,?28)",
            params![
                p.title.trim(), p.listing_type, p.category, p.district, p.neighborhood, p.address.trim(), p.lat, p.lon,
                p.price, p.currency, p.gross_m2, p.net_m2, p.rooms, p.building_age.trim(), p.floor.trim(), p.total_floors.trim(),
                p.heating, p.deed_status, p.furnished as i64, p.swap_ok as i64, p.credit_ok as i64, p.description.trim(),
                status, crm, me.id, now, published, closed
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        if p.owner_consent {
            if let Some(cid) = crm {
                self.conn.execute("UPDATE crm_contacts SET consent=1, consent_date=?2 WHERE id=?1 AND consent=0", params![cid, now_str()[..10].to_string()])?;
            }
        }
        self.conn.execute(
            "INSERT INTO price_history(portfolio_id,old_price,new_price,currency,user_name,changed_at) VALUES(?1,0,?2,?3,?4,?5)",
            params![id, p.price, p.currency, me.full_name(), now],
        )?;
        let who = me.full_name();
        let loc = if p.neighborhood.is_empty() { p.district.clone() } else { format!("{} / {}", p.district, p.neighborhood) };
        self.notify_all_except(me.id, "Portföy", &format!("{} yeni portföy ekledi: {} ({})", who, p.title.trim(), loc))?;
        Ok(id)
    }

    pub fn update_portfolio(&self, me: &UserInfo, id: i64, p: &Portfolio) -> R<()> {
        if p.title.trim().is_empty() {
            return Err(ApiErr::bad("Portföy başlığı boş olamaz."));
        }
        let (old_status, old_pub, old_closed, old_crm, old_price, old_cur): (String, String, String, Option<i64>, f64, String) = self
            .conn
            .query_row("SELECT status,published_at,closed_at,crm_id,price,currency FROM portfolios WHERE id=?1", params![id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
            })
            .optional()?
            .ok_or_else(ApiErr::not_found)?;
        let status = if STATUSES.contains(&p.status.as_str()) { p.status.clone() } else { old_status.clone() };
        let (published, closed) = Self::status_times(&old_status, &old_pub, &old_closed, &status);
        let mut crm = old_crm;
        if me.is_admin() {
            if let Some(cid) = old_crm {
                self.update_contact(cid, &p.owner_name, &p.owner_phone, &p.owner_email, &p.owner_note)?;
            } else {
                crm = self.upsert_contact(me.id, &p.owner_name, &p.owner_phone, &p.owner_email, &p.owner_note)?;
            }
        }
        self.conn.execute(
            "UPDATE portfolios SET title=?2,listing_type=?3,category=?4,district=?5,neighborhood=?6,address=?7,lat=?8,lon=?9,price=?10,currency=?11,gross_m2=?12,net_m2=?13,rooms=?14,building_age=?15,floor=?16,total_floors=?17,heating=?18,deed_status=?19,furnished=?20,swap_ok=?21,credit_ok=?22,description=?23,status=?24,crm_id=?25,updated_at=?26,published_at=?27,closed_at=?28 WHERE id=?1",
            params![
                id, p.title.trim(), p.listing_type, p.category, p.district, p.neighborhood, p.address.trim(), p.lat, p.lon,
                p.price, p.currency, p.gross_m2, p.net_m2, p.rooms, p.building_age.trim(), p.floor.trim(), p.total_floors.trim(),
                p.heating, p.deed_status, p.furnished as i64, p.swap_ok as i64, p.credit_ok as i64, p.description.trim(),
                status, crm, now_str(), published, closed
            ],
        )?;
        if (old_price - p.price).abs() > 0.004 || old_cur != p.currency {
            self.conn.execute(
                "INSERT INTO price_history(portfolio_id,old_price,new_price,currency,user_name,changed_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![id, old_price, p.price, p.currency, me.full_name(), now_str()],
            )?;
        }
        Ok(())
    }

    pub fn delete_portfolio(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM portfolios WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---------------- Fotoğraflar ----------------
    pub fn add_photo(&self, portfolio_id: i64, full: &[u8], thumb: &[u8]) -> R<i64> {
        let pos: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position),-1)+1 FROM photos WHERE portfolio_id=?1",
            params![portfolio_id],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO photos(portfolio_id,position,full,thumb) VALUES(?1,?2,?3,?4)",
            params![portfolio_id, pos, full, thumb],
        )?;
        self.conn.execute("UPDATE portfolios SET updated_at=?2 WHERE id=?1", params![portfolio_id, now_str()])?;
        Ok(self.conn.last_insert_rowid())
    }
    pub fn photo_data(&self, id: i64, thumb: bool) -> R<Vec<u8>> {
        let col = if thumb { "thumb" } else { "full" };
        self.conn
            .query_row(&format!("SELECT {col} FROM photos WHERE id=?1"), params![id], |r| r.get(0))
            .optional()?
            .ok_or_else(ApiErr::not_found)
    }
    pub fn photo_portfolio(&self, id: i64) -> R<i64> {
        self.conn
            .query_row("SELECT portfolio_id FROM photos WHERE id=?1", params![id], |r| r.get(0))
            .optional()?
            .ok_or_else(ApiErr::not_found)
    }
    pub fn delete_photo(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM photos WHERE id=?1", params![id])?;
        Ok(())
    }
    pub fn set_cover(&self, id: i64) -> R<()> {
        let pid = self.photo_portfolio(id)?;
        let minpos: i64 = self.conn.query_row("SELECT COALESCE(MIN(position),0) FROM photos WHERE portfolio_id=?1", params![pid], |r| r.get(0))?;
        self.conn.execute("UPDATE photos SET position=?2 WHERE id=?1", params![id, minpos - 1])?;
        Ok(())
    }

    // ---------------- Görevler ----------------
    pub fn create_tasks(&self, me: &UserInfo, t: &NewTask) -> R<usize> {
        if t.title.trim().is_empty() {
            return Err(ApiErr::bad("Başlık boş olamaz."));
        }
        if t.assignees.is_empty() {
            return Err(ApiErr::bad("En az bir çalışan seçin."));
        }
        let kind = if TASK_KINDS.contains(&t.kind.as_str()) { t.kind.clone() } else { "Görev".into() };
        for uid in &t.assignees {
            self.conn.execute(
                "INSERT INTO tasks(title,details,kind,assigned_to,due_at,status,created_by,created_at) VALUES(?1,?2,?3,?4,?5,'Açık',?6,?7)",
                params![t.title.trim(), t.details.trim(), kind, uid, t.due_at.trim(), me.id, now_str()],
            )?;
            let when = if t.due_at.trim().is_empty() { String::new() } else { format!(" — {}", t.due_at.trim()) };
            self.notify(*uid, &kind, &format!("Yeni {}: {}{}", kind.to_lowercase(), t.title.trim(), when))?;
        }
        Ok(t.assignees.len())
    }

    pub fn list_tasks(&self, me: &UserInfo, all: bool) -> R<Vec<TaskInfo>> {
        let sql = format!(
            "SELECT t.id,t.title,t.details,t.kind,t.assigned_to,{},t.due_at,t.status,{},t.created_at FROM tasks t {} ORDER BY (t.status='Açık') DESC, CASE WHEN t.due_at='' THEN 1 ELSE 0 END, t.due_at, t.id DESC LIMIT 1000",
            user_name_sql("t.assigned_to"),
            user_name_sql("t.created_by"),
            if all && me.is_lead() { "" } else { "WHERE t.assigned_to=?1" }
        );
        let mut st = self.conn.prepare(&sql)?;
        let map = |r: &Row| -> rusqlite::Result<TaskInfo> {
            Ok(TaskInfo {
                id: r.get(0)?,
                title: r.get(1)?,
                details: r.get(2)?,
                kind: r.get(3)?,
                assigned_to: r.get(4)?,
                assignee_name: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
                due_at: r.get(6)?,
                status: r.get(7)?,
                created_by_name: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
                created_at: r.get(9)?,
            })
        };
        let v = if all && me.is_lead() {
            st.query_map([], map)?.collect::<Result<Vec<_>, _>>()?
        } else {
            st.query_map(params![me.id], map)?.collect::<Result<Vec<_>, _>>()?
        };
        Ok(v)
    }

    pub fn task_assignee(&self, id: i64) -> R<i64> {
        self.conn
            .query_row("SELECT assigned_to FROM tasks WHERE id=?1", params![id], |r| r.get(0))
            .optional()?
            .ok_or_else(ApiErr::not_found)
    }
    pub fn set_task_status(&self, id: i64, status: &str) -> R<()> {
        let s = if status == "Tamamlandı" { "Tamamlandı" } else { "Açık" };
        self.conn.execute("UPDATE tasks SET status=?2 WHERE id=?1", params![id, s])?;
        Ok(())
    }
    pub fn delete_task(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---------------- Mesai ----------------
    pub fn check_in(&self, uid: i64) -> R<()> {
        let open: i64 = self.conn.query_row("SELECT COUNT(*) FROM attendance WHERE user_id=?1 AND check_out=''", params![uid], |r| r.get(0))?;
        if open > 0 {
            return Err(ApiErr::bad("Zaten açık bir mesainiz var. Önce mesaiyi bitirin."));
        }
        self.conn.execute("INSERT INTO attendance(user_id,check_in) VALUES(?1,?2)", params![uid, now_str()])?;
        Ok(())
    }
    pub fn check_out(&self, uid: i64) -> R<()> {
        let n = self.conn.execute(
            "UPDATE attendance SET check_out=?2 WHERE user_id=?1 AND check_out=''",
            params![uid, now_str()],
        )?;
        if n == 0 {
            return Err(ApiErr::bad("Açık bir mesai kaydı yok."));
        }
        Ok(())
    }
    pub fn list_attendance(&self, user_id: Option<i64>, month: &str) -> R<Vec<AttInfo>> {
        let like = format!("{}%", month);
        let base = format!(
            "SELECT a.id,a.user_id,{},a.check_in,a.check_out,a.note FROM attendance a WHERE (a.check_in LIKE ?1 OR a.check_out='')",
            user_name_sql("a.user_id")
        );
        let map = |r: &Row| -> rusqlite::Result<AttInfo> {
            Ok(AttInfo {
                id: r.get(0)?,
                user_id: r.get(1)?,
                user_name: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                check_in: r.get(3)?,
                check_out: r.get(4)?,
                note: r.get(5)?,
            })
        };
        // açık kayıtlar başka aydan olsa da yalnızca o ayın filtresine uyanlar + açık olanlar gösterilir
        let v = if let Some(uid) = user_id {
            let sql = format!("{base} AND a.user_id=?2 ORDER BY a.check_in DESC");
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(params![like, uid], map)?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let sql = format!("{base} ORDER BY a.check_in DESC");
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(params![like], map)?.collect::<Result<Vec<_>, _>>()?;
            rows
        };
        Ok(v)
    }
    pub fn update_attendance(&self, id: i64, check_in: &str, check_out: &str, note: &str) -> R<()> {
        for t in [check_in, check_out] {
            if !t.is_empty() && chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S").is_err() {
                return Err(ApiErr::bad("Tarih biçimi: YYYY-AA-GG SS:DD:ss"));
            }
        }
        if check_in.is_empty() {
            return Err(ApiErr::bad("Giriş saati boş olamaz."));
        }
        self.conn.execute(
            "UPDATE attendance SET check_in=?2,check_out=?3,note=?4 WHERE id=?1",
            params![id, check_in, check_out, note],
        )?;
        Ok(())
    }
    pub fn delete_attendance(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM attendance WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---------------- Hedefler ----------------
    pub fn create_goal(&self, me: &UserInfo, g: &GoalInfo) -> R<()> {
        if g.title.trim().is_empty() {
            return Err(ApiErr::bad("Hedef başlığı boş olamaz."));
        }
        let uid: Option<i64> = if g.user_id > 0 { Some(g.user_id) } else { None };
        self.conn.execute(
            "INSERT INTO goals(title,user_id,period_start,period_end,t_portfolio,t_published,t_rented,t_sold,note,created_by,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![g.title.trim(), uid, g.period_start, g.period_end, g.t_portfolio, g.t_published, g.t_rented, g.t_sold, g.note.trim(), me.id, now_str()],
        )?;
        let msg = format!("Yeni hedef: {} ({} – {})", g.title.trim(), g.period_start, g.period_end);
        match uid {
            Some(u) => self.notify(u, "Hedef", &msg)?,
            None => {
                for id in self.active_user_ids()? {
                    self.notify(id, "Hedef", &msg)?;
                }
            }
        }
        Ok(())
    }
    pub fn delete_goal(&self, id: i64) -> R<()> {
        self.conn.execute("DELETE FROM goals WHERE id=?1", params![id])?;
        Ok(())
    }
    fn count(&self, sql: &str, s: &str, e: &str, uid: Option<i64>) -> R<i64> {
        let full = if uid.is_some() { format!("{sql} AND creator_id=?3") } else { sql.to_string() };
        Ok(match uid {
            Some(u) => self.conn.query_row(&full, params![s, e, u], |r| r.get(0))?,
            None => self.conn.query_row(&full, params![s, e], |r| r.get(0))?,
        })
    }
    pub fn list_goals(&self, me: &UserInfo) -> R<Vec<GoalInfo>> {
        let sql = format!(
            "SELECT id,title,COALESCE(user_id,0),{},period_start,period_end,t_portfolio,t_published,t_rented,t_sold,note FROM goals g {} ORDER BY period_end DESC, id DESC",
            user_name_sql("g.user_id"),
            if me.is_lead() { "" } else { "WHERE (g.user_id IS NULL OR g.user_id=?1)" }
        );
        let mut st = self.conn.prepare(&sql)?;
        let map = |r: &Row| -> rusqlite::Result<GoalInfo> {
            Ok(GoalInfo {
                id: r.get(0)?,
                title: r.get(1)?,
                user_id: r.get(2)?,
                user_name: r.get::<_, Option<String>>(3)?.unwrap_or_else(|| "Tüm ekip".into()),
                period_start: r.get(4)?,
                period_end: r.get(5)?,
                t_portfolio: r.get(6)?,
                t_published: r.get(7)?,
                t_rented: r.get(8)?,
                t_sold: r.get(9)?,
                note: r.get(10)?,
                ..Default::default()
            })
        };
        let mut goals = if me.is_lead() {
            st.query_map([], map)?.collect::<Result<Vec<_>, _>>()?
        } else {
            st.query_map(params![me.id], map)?.collect::<Result<Vec<_>, _>>()?
        };
        for g in goals.iter_mut() {
            let uid = if g.user_id > 0 { Some(g.user_id) } else { None };
            let (s, e) = (g.period_start.clone(), g.period_end.clone());
            g.p_portfolio = self.count("SELECT COUNT(*) FROM portfolios WHERE status<>'Taslak' AND substr(created_at,1,10) BETWEEN ?1 AND ?2", &s, &e, uid)?;
            g.p_published = self.count("SELECT COUNT(*) FROM portfolios WHERE published_at<>'' AND substr(published_at,1,10) BETWEEN ?1 AND ?2", &s, &e, uid)?;
            g.p_rented = self.count("SELECT COUNT(*) FROM portfolios WHERE status='Kiralandı' AND substr(closed_at,1,10) BETWEEN ?1 AND ?2", &s, &e, uid)?;
            g.p_sold = self.count("SELECT COUNT(*) FROM portfolios WHERE status='Satıldı' AND substr(closed_at,1,10) BETWEEN ?1 AND ?2", &s, &e, uid)?;
        }
        Ok(goals)
    }

    // ---------------- Ayarlar ----------------
    pub fn get_settings(&self) -> R<OutputSettings> {
        let v: Option<String> = self.conn.query_row("SELECT value FROM settings WHERE key='output'", [], |r| r.get(0)).optional()?;
        Ok(v.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }
    pub fn set_settings(&self, s: &OutputSettings) -> R<()> {
        let json = serde_json::to_string(s).map_err(|e| ApiErr::bad(e.to_string()))?;
        self.conn.execute("INSERT INTO settings(key,value) VALUES('output',?1) ON CONFLICT(key) DO UPDATE SET value=?1", params![json])?;
        Ok(())
    }
    pub fn set_blob(&self, key: &str, data: Option<&[u8]>) -> R<()> {
        match data {
            Some(d) => self.conn.execute("INSERT INTO blobs(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=?2", params![key, d])?,
            None => self.conn.execute("DELETE FROM blobs WHERE key=?1", params![key])?,
        };
        Ok(())
    }
    pub fn get_blob(&self, key: &str) -> R<Option<Vec<u8>>> {
        Ok(self.conn.query_row("SELECT data FROM blobs WHERE key=?1", params![key], |r| r.get(0)).optional()?)
    }

    // ---------------- Yedek ----------------
    pub fn backup_dir(&self) -> PathBuf {
        self.path.parent().unwrap_or(Path::new(".")).join("yedekler")
    }
    pub fn make_backup(&self) -> R<PathBuf> {
        let dir = self.backup_dir();
        std::fs::create_dir_all(&dir).map_err(|e| ApiErr(500, e.to_string()))?;
        let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let mut target = dir.join(format!("emlak_{stamp}.db"));
        let mut n = 2;
        while target.exists() {
            target = dir.join(format!("emlak_{stamp}_{n}.db"));
            n += 1;
        }
        self.conn.execute("VACUUM INTO ?1", params![target.to_string_lossy().to_string()])?;
        if let Err(e) = self.copy_external(&target) {
            eprintln!("Harici yedek hatası: {e}");
        }
        // en yeni 30 yedeği tut
        let mut files = self.list_backups();
        files.sort();
        while files.len() > 30 {
            let old = files.remove(0);
            let _ = std::fs::remove_file(dir.join(old));
        }
        Ok(target)
    }
    pub fn list_backups(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.backup_dir())
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".db")).collect())
            .unwrap_or_default();
        v.sort();
        v.reverse();
        v
    }
}

/// Türkçe harf sırasına yakın sıralama anahtarı
pub fn tr_key(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'ç' | 'Ç' => "c~".chars().collect::<Vec<_>>(),
            'ğ' | 'Ğ' => "g~".chars().collect(),
            'ı' | 'I' => "h~".chars().collect(),
            'İ' | 'i' => "i".chars().collect(),
            'ö' | 'Ö' => "o~".chars().collect(),
            'ş' | 'Ş' => "s~".chars().collect(),
            'ü' | 'Ü' => "u~".chars().collect(),
            c => c.to_lowercase().collect(),
        })
        .collect()
}
