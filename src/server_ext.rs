//! Sürüm 1.2 sunucu uçları: alıcılar, eşleştirme, takvim, izin, komisyon, panel, duyuru, işlem kaydı,
//! KVKK, fotoğraf sıralama/döndürme, uygulama içi güncelleme, harici yedek ayarları.
use crate::common::*;
use crate::db::*;
use crate::server::{Resp, State};
use chrono::{Datelike, Duration as CDur, NaiveDate};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::HashMap;

macro_rules! P {
    ($($x:expr),* $(,)?) => { &[$(&$x as &dyn rusqlite::ToSql),*] };
}

fn vs(v: &Value, k: &str) -> String {
    match &v[k] {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
fn vf(v: &Value, k: &str) -> f64 {
    match &v[k] {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().replace(',', ".").parse().unwrap_or(0.0),
        _ => 0.0,
    }
}
fn vi(v: &Value, k: &str) -> i64 {
    match &v[k] {
        Value::Number(n) => n.as_i64().unwrap_or(0),
        Value::String(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}
fn vb(v: &Value, k: &str) -> bool {
    match &v[k] {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}
fn body_json(b: &[u8]) -> R<Value> {
    if b.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(b).map_err(|e| ApiErr::bad(format!("Geçersiz istek: {e}")))
}
fn id_of(s: &str) -> R<i64> {
    s.parse().map_err(|_| ApiErr::bad("Geçersiz kimlik"))
}
fn ok() -> R<Option<Resp>> {
    Ok(Some(Resp::Json(json!({"ok": true}))))
}
fn out(v: Value) -> R<Option<Resp>> {
    Ok(Some(Resp::Json(v)))
}
fn need_admin(me: &UserInfo) -> R<()> {
    if me.is_admin() { Ok(()) } else { Err(ApiErr::forbidden()) }
}
fn need_lead(me: &UserInfo) -> R<()> {
    if me.is_lead() { Ok(()) } else { Err(ApiErr::forbidden()) }
}
fn cnt(db: &Db, sql: &str, p: &[&dyn rusqlite::ToSql]) -> i64 {
    db.conn.query_row(sql, p, |r| r.get::<_, i64>(0)).unwrap_or(0)
}
pub fn aud(db: &Db, me: &UserInfo, action: &str, detail: &str) {
    db.audit(me.id, &me.full_name(), action, detail);
}

// ------------------------------------------------------------ portföy kanca noktaları

pub struct Snap {
    status: String,
    price: f64,
}

pub fn snapshot(db: &Db, id: i64) -> Option<Snap> {
    db.conn.query_row("SELECT status,price FROM portfolios WHERE id=?1", params![id], |r| Ok(Snap { status: r.get(0)?, price: r.get(1)? })).ok()
}

/// Portföy kaydedildikten sonra: işlem kaydı + alıcı eşleştirme bildirimi.
pub fn after_portfolio_save(db: &Db, me: &UserInfo, id: i64, before: Option<Snap>) {
    let Some(now) = snapshot(db, id) else { return };
    let title: String = db.conn.query_row("SELECT title FROM portfolios WHERE id=?1", params![id], |r| r.get(0)).unwrap_or_default();
    match &before {
        None => aud(db, me, "Portföy eklendi", &format!("P-{:05} {}", id, title)),
        Some(b) => {
            let mut d = format!("P-{:05} {}", id, title);
            if b.status != now.status {
                d.push_str(&format!(" | durum: {} → {}", b.status, now.status));
            }
            if (b.price - now.price).abs() > 0.004 {
                d.push_str(&format!(" | fiyat: {} → {}", b.price, now.price));
            }
            aud(db, me, "Portföy güncellendi", &d);
        }
    }
    let reset = match &before {
        None => true,
        Some(b) => b.status != now.status || (b.price - now.price).abs() > 0.004,
    };
    if reset {
        let _ = db.conn.execute("UPDATE portfolios SET match_notified=0 WHERE id=?1", params![id]);
    }
    if db.kv_get("match_enabled").as_deref() == Some("0") {
        return;
    }
    let notified: i64 = cnt(db, "SELECT match_notified FROM portfolios WHERE id=?1", P![id]);
    if now.status == "Yayında" && notified == 0 {
        if let Ok(buyers) = buyers_for_portfolio(db, id) {
            for b in buyers {
                let owner = vi(&b, "created_by");
                if owner > 0 {
                    let _ = db.notify(
                        owner,
                        "Eşleşme",
                        &format!("P-{:05} \"{}\" portföyü alıcınız {} ile eşleşiyor.", id, title, vs(&b, "full_name")),
                    );
                }
            }
        }
        let _ = db.conn.execute("UPDATE portfolios SET match_notified=1 WHERE id=?1", params![id]);
    }
}

fn buyers_for_portfolio(db: &Db, pid: i64) -> R<Vec<Value>> {
    db.rows(
        "SELECT b.* FROM buyers b, portfolios p WHERE p.id=?1 AND b.status='Aktif' AND b.listing_type=p.listing_type \
         AND (b.category='' OR b.category=p.category) AND (b.district='' OR b.district=p.district) \
         AND (b.neighborhood='' OR b.neighborhood=p.neighborhood) AND b.currency=p.currency \
         AND (b.budget_min<=0 OR p.price>=b.budget_min) AND (b.budget_max<=0 OR p.price<=b.budget_max) \
         AND (b.rooms='' OR b.rooms='-' OR b.rooms=p.rooms) ORDER BY b.updated_at DESC",
        P![pid],
    )
}

fn portfolios_for_buyer(db: &Db, bid: i64) -> R<Vec<Value>> {
    db.rows(
        "SELECT p.id,p.title,p.listing_type,p.category,p.district,p.neighborhood,p.price,p.currency,p.rooms,p.gross_m2 \
         FROM portfolios p, buyers b WHERE b.id=?1 AND p.status='Yayında' AND b.listing_type=p.listing_type \
         AND (b.category='' OR b.category=p.category) AND (b.district='' OR b.district=p.district) \
         AND (b.neighborhood='' OR b.neighborhood=p.neighborhood) AND b.currency=p.currency \
         AND (b.budget_min<=0 OR p.price>=b.budget_min) AND (b.budget_max<=0 OR p.price<=b.budget_max) \
         AND (b.rooms='' OR b.rooms='-' OR b.rooms=p.rooms) ORDER BY p.created_at DESC LIMIT 100",
        P![bid],
    )
}

fn buyer_visible(db: &Db, me: &UserInfo, id: i64) -> R<()> {
    if me.is_lead() {
        return Ok(());
    }
    let owner = cnt(db, "SELECT created_by FROM buyers WHERE id=?1", P![id]);
    if owner == me.id { Ok(()) } else { Err(ApiErr::forbidden()) }
}

fn uname(col: &str) -> String {
    user_name_sql(col)
}

// ------------------------------------------------------------ ana yönlendirici

pub fn route(st: &State, db: &Db, me: &UserInfo, method: &str, segs: &[&str], q: &HashMap<String, String>, body: &[u8]) -> Option<R<Resp>> {
    match inner(st, db, me, method, segs, q, body) {
        Ok(Some(r)) => Some(Ok(r)),
        Ok(None) => None,
        Err(e) => Some(Err(e)),
    }
}

fn inner(st: &State, db: &Db, me: &UserInfo, method: &str, segs: &[&str], q: &HashMap<String, String>, body: &[u8]) -> R<Option<Resp>> {
    let now = now_str();
    if let Some(r) = crate::web::route(db, me, method, segs, body)? {
        return Ok(Some(r));
    }
    match (method, segs) {
        // ================================================= PANEL
        ("GET", ["api", "dashboard"]) => {
            let month = q.get("month").cloned().unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
            let ml = format!("{month}%");
            let cutoff = (chrono::Local::now() - CDur::days(60)).format("%Y-%m-%d").to_string();
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let week = (chrono::Local::now() + CDur::days(7)).format("%Y-%m-%d").to_string();
            let lead = me.is_lead();
            let uid = if lead { 0 } else { me.id };
            // lead için uid=0 tüm kayıtları kapsar (user_id=?1 OR ?1=0)
            let mut by_user = Vec::new();
            let users = db.list_users()?;
            for u in users.iter().filter(|u| u.active && (lead || u.id == me.id)) {
                by_user.push(json!({
                    "user": u.full_name(),
                    "created": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND created_at LIKE ?2", P![u.id, ml]),
                    "published": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND published_at LIKE ?2", P![u.id, ml]),
                    "rented": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND status='Kiralandı' AND closed_at LIKE ?2", P![u.id, ml]),
                    "sold": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE creator_id=?1 AND status='Satıldı' AND closed_at LIKE ?2", P![u.id, ml]),
                }));
            }
            let by_district = db.rows("SELECT district AS name, COUNT(*) AS n FROM portfolios WHERE status='Yayında' AND district<>'' GROUP BY district ORDER BY n DESC LIMIT 12", P![])?;
            let mut trend = Vec::new();
            let first = NaiveDate::from_ymd_opt(chrono::Local::now().year(), chrono::Local::now().month(), 1).unwrap();
            for k in (0..6).rev() {
                let mut y = first.year();
                let mut m = first.month() as i32 - k;
                while m <= 0 {
                    m += 12;
                    y -= 1;
                }
                let key = format!("{y:04}-{m:02}");
                let like = format!("{key}%");
                trend.push(json!({
                    "month": key,
                    "created": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE created_at LIKE ?1", P![like]),
                    "closed": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE status IN ('Satıldı','Kiralandı') AND closed_at LIKE ?1", P![like]),
                }));
            }
            out(json!({
                "month": month,
                "total": cnt(db, "SELECT COUNT(*) FROM portfolios", P![]),
                "active": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE status='Yayında'", P![]),
                "new_month": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE created_at LIKE ?1", P![ml]),
                "published_month": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE published_at LIKE ?1", P![ml]),
                "rented_month": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE status='Kiralandı' AND closed_at LIKE ?1", P![ml]),
                "sold_month": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE status='Satıldı' AND closed_at LIKE ?1", P![ml]),
                "stale": cnt(db, "SELECT COUNT(*) FROM portfolios WHERE status='Yayında' AND published_at<>'' AND published_at<?1", P![cutoff]),
                "buyers": cnt(db, "SELECT COUNT(*) FROM buyers WHERE status='Aktif' AND (created_by=?1 OR ?1=0)", P![uid]),
                "open_tasks": cnt(db, "SELECT COUNT(*) FROM tasks WHERE status<>'Tamamlandı' AND (assigned_to=?1 OR ?1=0)", P![uid]),
                "appts_today": cnt(db, "SELECT COUNT(*) FROM appointments WHERE status='Planlı' AND substr(start_at,1,10)=?1 AND (user_id=?2 OR ?2=0)", P![today, uid]),
                "appts_week": cnt(db, "SELECT COUNT(*) FROM appointments WHERE status='Planlı' AND substr(start_at,1,10)>=?1 AND substr(start_at,1,10)<=?2 AND (user_id=?3 OR ?3=0)", P![today, week, uid]),
                "pending_leaves": if lead { cnt(db, "SELECT COUNT(*) FROM leaves WHERE status='Bekliyor'", P![]) } else { 0 },
                "by_user": by_user,
                "by_district": by_district,
                "trend": trend,
            }))
        }

        // ================================================= ALICILAR
        ("GET", ["api", "buyers"]) => {
            let uid = if me.is_lead() { 0 } else { me.id };
            let sql = format!(
                "SELECT b.*, {} AS owner_name FROM buyers b WHERE (b.created_by=?1 OR ?1=0) ORDER BY b.updated_at DESC",
                uname("b.created_by")
            );
            let mut rows = db.rows(&sql, P![uid])?;
            for r in rows.iter_mut() {
                let id = vi(r, "id");
                let n = portfolios_for_buyer(db, id).map(|v| v.len()).unwrap_or(0);
                r["match_count"] = json!(n);
            }
            out(Value::Array(rows))
        }
        ("POST", ["api", "buyers"]) => {
            let v = body_json(body)?;
            if vs(&v, "full_name").is_empty() {
                return Err(ApiErr::bad("Alıcı adı boş olamaz."));
            }
            let consent = vb(&v, "consent");
            db.conn.execute(
                "INSERT INTO buyers(full_name,phone,email,listing_type,category,district,neighborhood,budget_min,budget_max,currency,rooms,note,consent,consent_date,status,created_by,created_at,updated_at) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?17)",
                params![
                    vs(&v, "full_name"), vs(&v, "phone"), vs(&v, "email"),
                    if vs(&v, "listing_type").is_empty() { "Satılık".to_string() } else { vs(&v, "listing_type") },
                    vs(&v, "category"), vs(&v, "district"), vs(&v, "neighborhood"),
                    vf(&v, "budget_min"), vf(&v, "budget_max"),
                    if vs(&v, "currency").is_empty() { "TL".to_string() } else { vs(&v, "currency") },
                    vs(&v, "rooms"), vs(&v, "note"), consent as i64, if consent { now[..10].to_string() } else { String::new() },
                    if vs(&v, "status").is_empty() { "Aktif".to_string() } else { vs(&v, "status") }, me.id, now
                ],
            )?;
            let id = db.conn.last_insert_rowid();
            aud(db, me, "Alıcı eklendi", &format!("{} (#{})", vs(&v, "full_name"), id));
            let n = portfolios_for_buyer(db, id)?.len();
            if n > 0 {
                let _ = db.notify(me.id, "Eşleşme", &format!("{} için şu an {} yayındaki portföy eşleşiyor.", vs(&v, "full_name"), n));
            }
            out(json!({"id": id, "matches": n}))
        }
        ("PUT", ["api", "buyers", id]) => {
            let id = id_of(id)?;
            buyer_visible(db, me, id)?;
            let v = body_json(body)?;
            let consent = vb(&v, "consent");
            db.conn.execute(
                "UPDATE buyers SET full_name=?2,phone=?3,email=?4,listing_type=?5,category=?6,district=?7,neighborhood=?8,budget_min=?9,budget_max=?10,currency=?11,rooms=?12,note=?13,status=?14,\
                 consent=?15, consent_date=CASE WHEN ?15=1 AND consent_date='' THEN ?16 WHEN ?15=0 THEN '' ELSE consent_date END, updated_at=?17 WHERE id=?1",
                params![
                    id, vs(&v, "full_name"), vs(&v, "phone"), vs(&v, "email"), vs(&v, "listing_type"), vs(&v, "category"), vs(&v, "district"),
                    vs(&v, "neighborhood"), vf(&v, "budget_min"), vf(&v, "budget_max"), vs(&v, "currency"), vs(&v, "rooms"), vs(&v, "note"),
                    vs(&v, "status"), consent as i64, now[..10].to_string(), now
                ],
            )?;
            ok()
        }
        ("DELETE", ["api", "buyers", id]) => {
            let id = id_of(id)?;
            buyer_visible(db, me, id)?;
            let name: String = db.conn.query_row("SELECT full_name FROM buyers WHERE id=?1", params![id], |r| r.get(0)).unwrap_or_default();
            db.conn.execute("DELETE FROM buyers WHERE id=?1", params![id])?;
            aud(db, me, "Alıcı silindi", &format!("{name} (#{id})"));
            ok()
        }
        ("GET", ["api", "buyers", id, "matches"]) => {
            let id = id_of(id)?;
            buyer_visible(db, me, id)?;
            out(Value::Array(portfolios_for_buyer(db, id)?))
        }
        ("GET", ["api", "buyers", id, "export"]) => {
            let id = id_of(id)?;
            buyer_visible(db, me, id)?;
            let mut rows = db.rows("SELECT * FROM buyers WHERE id=?1", P![id])?;
            aud(db, me, "KVKK: alıcı verisi dışa aktarıldı", &format!("#{id}"));
            out(rows.pop().unwrap_or(json!({})))
        }
        ("POST", ["api", "buyers", id, "anonymize"]) => {
            let id = id_of(id)?;
            need_admin(me)?;
            db.conn.execute(
                "UPDATE buyers SET full_name='Anonim alıcı #'||id, phone='', email='', note='', consent=0, consent_date='', status='Pasif', updated_at=?2 WHERE id=?1",
                params![id, now],
            )?;
            aud(db, me, "KVKK: alıcı anonimleştirildi", &format!("#{id}"));
            ok()
        }
        ("GET", ["api", "portfolios", id, "matches"]) => {
            let id = id_of(id)?;
            let mut rows = buyers_for_portfolio(db, id)?;
            if !me.is_lead() {
                rows.retain(|r| vi(r, "created_by") == me.id);
            }
            out(Value::Array(rows))
        }

        // ================================================= PORTFÖY EKLERİ
        ("GET", ["api", "portfolios", id, "history"]) => {
            let id = id_of(id)?;
            out(Value::Array(db.rows("SELECT old_price,new_price,currency,user_name,changed_at FROM price_history WHERE portfolio_id=?1 ORDER BY id DESC", P![id])?))
        }
        ("GET", ["api", "portfolios", id, "notes"]) => {
            let id = id_of(id)?;
            let sql = format!("SELECT n.id,n.user_id,n.text,n.created_at,{} AS user_name FROM portfolio_notes n WHERE n.portfolio_id=?1 ORDER BY n.id DESC", uname("n.user_id"));
            out(Value::Array(db.rows(&sql, P![id])?))
        }
        ("POST", ["api", "portfolios", id, "notes"]) => {
            let id = id_of(id)?;
            let v = body_json(body)?;
            let t = vs(&v, "text");
            if t.is_empty() {
                return Err(ApiErr::bad("Not boş olamaz."));
            }
            db.conn.execute("INSERT INTO portfolio_notes(portfolio_id,user_id,text,created_at) VALUES(?1,?2,?3,?4)", params![id, me.id, t, now])?;
            let creator = db.portfolio_creator(id).unwrap_or(0);
            if creator > 0 && creator != me.id {
                let _ = db.notify(creator, "Portföy notu", &format!("{} P-{:05} portföyünüze not ekledi.", me.full_name(), id));
            }
            ok()
        }
        ("DELETE", ["api", "notes", id]) => {
            let id = id_of(id)?;
            let owner = cnt(db, "SELECT user_id FROM portfolio_notes WHERE id=?1", P![id]);
            if owner != me.id && !me.is_admin() {
                return Err(ApiErr::forbidden());
            }
            db.conn.execute("DELETE FROM portfolio_notes WHERE id=?1", params![id])?;
            ok()
        }
        ("PUT", ["api", "portfolios", id, "photos", "order"]) => {
            let id = id_of(id)?;
            if !me.is_admin() && db.portfolio_creator(id)? != me.id {
                return Err(ApiErr::forbidden());
            }
            let v = body_json(body)?;
            if let Some(ids) = v["ids"].as_array() {
                for (pos, pid) in ids.iter().filter_map(|x| x.as_i64()).enumerate() {
                    db.conn.execute("UPDATE photos SET position=?2 WHERE id=?1 AND portfolio_id=?3", params![pid, pos as i64, id])?;
                }
            }
            crate::web::push_bg(db, id);
            ok()
        }
        ("POST", ["api", "photos", id, "rotate"]) => {
            let id = id_of(id)?;
            let pid = db.photo_portfolio(id)?;
            if !me.is_admin() && db.portfolio_creator(pid)? != me.id {
                return Err(ApiErr::forbidden());
            }
            let deg = q.get("deg").and_then(|s| s.parse::<i32>().ok()).unwrap_or(90);
            let full = db.photo_data(id, false)?;
            let mut img = image::load_from_memory(&full).map_err(|_| ApiErr::bad("Resim okunamadı."))?;
            img = match deg.rem_euclid(360) {
                90 => img.rotate90(),
                180 => img.rotate180(),
                270 => img.rotate270(),
                _ => img,
            };
            let enc = |im: &image::DynamicImage, qual: u8| -> R<Vec<u8>> {
                let mut o = Vec::new();
                let e = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut o, qual);
                image::DynamicImage::ImageRgb8(im.to_rgb8()).write_with_encoder(e).map_err(|e| ApiErr(500, e.to_string()))?;
                Ok(o)
            };
            let thumb = img.resize(320, 320, image::imageops::FilterType::Triangle);
            db.conn.execute("UPDATE photos SET full=?2, thumb=?3 WHERE id=?1", params![id, enc(&img, 85)?, enc(&thumb, 78)?])?;
            crate::web::push_bg(db, pid);
            ok()
        }
        ("GET", ["api", "map"]) => out(Value::Array(db.rows(
            "SELECT id,title,listing_type,category,district,neighborhood,price,currency,status,lat,lon,creator_id FROM portfolios WHERE lat IS NOT NULL AND lon IS NOT NULL AND status IN ('Yayında','Taslak') ORDER BY id DESC LIMIT 2000",
            P![],
        )?)),

        // ================================================= DUYURULAR
        ("GET", ["api", "announcements"]) => {
            let sql = format!("SELECT a.*,{} AS author FROM announcements a ORDER BY a.pinned DESC, a.id DESC LIMIT 60", uname("a.created_by"));
            out(Value::Array(db.rows(&sql, P![])?))
        }
        ("POST", ["api", "announcements"]) => {
            need_lead(me)?;
            let v = body_json(body)?;
            if vs(&v, "title").is_empty() {
                return Err(ApiErr::bad("Duyuru başlığı boş olamaz."));
            }
            db.conn.execute(
                "INSERT INTO announcements(title,body,pinned,created_by,created_at) VALUES(?1,?2,?3,?4,?5)",
                params![vs(&v, "title"), vs(&v, "body"), vb(&v, "pinned") as i64, me.id, now],
            )?;
            let _ = db.notify_all_except(me.id, "Duyuru", &vs(&v, "title"));
            aud(db, me, "Duyuru yayınlandı", &vs(&v, "title"));
            ok()
        }
        ("PUT", ["api", "announcements", id, "pin"]) => {
            need_lead(me)?;
            let v = body_json(body)?;
            db.conn.execute("UPDATE announcements SET pinned=?2 WHERE id=?1", params![id_of(id)?, vb(&v, "pinned") as i64])?;
            ok()
        }
        ("DELETE", ["api", "announcements", id]) => {
            need_lead(me)?;
            db.conn.execute("DELETE FROM announcements WHERE id=?1", params![id_of(id)?])?;
            ok()
        }

        // ================================================= TAKVİM
        ("GET", ["api", "appointments"]) => {
            let from = q.get("from").cloned().unwrap_or_else(|| now[..10].to_string());
            let to = q.get("to").cloned().unwrap_or_else(|| (chrono::Local::now() + CDur::days(31)).format("%Y-%m-%d").to_string());
            let sql = format!(
                "SELECT a.*, {} AS user_name, COALESCE((SELECT title FROM portfolios p WHERE p.id=a.portfolio_id),'') AS portfolio_title, \
                 COALESCE((SELECT full_name FROM buyers b WHERE b.id=a.buyer_id),'') AS buyer_name, \
                 COALESCE((SELECT phone FROM buyers b WHERE b.id=a.buyer_id),'') AS buyer_phone, \
                 COALESCE((SELECT email FROM buyers b WHERE b.id=a.buyer_id),'') AS buyer_email \
                 FROM appointments a WHERE substr(a.start_at,1,10)>=?1 AND substr(a.start_at,1,10)<=?2 ORDER BY a.start_at",
                uname("a.user_id")
            );
            out(Value::Array(db.rows(&sql, P![from, to])?))
        }
        ("POST", ["api", "appointments"]) => {
            let v = body_json(body)?;
            let mut uid = vi(&v, "user_id");
            if uid <= 0 || (!me.is_lead() && uid != me.id) {
                uid = me.id;
            }
            let start = vs(&v, "start_at");
            if start.len() < 16 || vs(&v, "title").is_empty() {
                return Err(ApiErr::bad("Başlık ve tarih/saat gerekli."));
            }
            let dur = vi(&v, "duration_min").clamp(5, 600);
            if !vb(&v, "force") {
                if let Some(c) = conflict(db, uid, &start, dur, 0) {
                    return Err(ApiErr(409, format!("Çakışma: {c} Yine de kaydetmek için onaylayın.")));
                }
            }
            db.conn.execute(
                "INSERT INTO appointments(title,kind,start_at,duration_min,user_id,portfolio_id,buyer_id,note,status,created_by,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'Planlı',?9,?10)",
                params![vs(&v, "title"), vs(&v, "kind"), start, dur, uid, vi(&v, "portfolio_id"), vi(&v, "buyer_id"), vs(&v, "note"), me.id, now],
            )?;
            if uid != me.id {
                let _ = db.notify(uid, "Randevu", &format!("{} sizin için randevu ekledi: {} ({})", me.full_name(), vs(&v, "title"), start));
            }
            ok()
        }
        ("PUT", ["api", "appointments", id]) => {
            let id = id_of(id)?;
            let owner = cnt(db, "SELECT user_id FROM appointments WHERE id=?1", P![id]);
            let creator = cnt(db, "SELECT created_by FROM appointments WHERE id=?1", P![id]);
            if !me.is_lead() && owner != me.id && creator != me.id {
                return Err(ApiErr::forbidden());
            }
            let v = body_json(body)?;
            if !vb(&v, "force") && vs(&v, "status") != "İptal" && vs(&v, "status") != "Tamamlandı" {
                if let Some(c) = conflict(db, vi(&v, "user_id").max(owner), &vs(&v, "start_at"), vi(&v, "duration_min").clamp(5, 600), id) {
                    return Err(ApiErr(409, format!("Çakışma: {c} Yine de kaydetmek için onaylayın.")));
                }
            }
            db.conn.execute(
                "UPDATE appointments SET title=?2,kind=?3,start_at=?4,duration_min=?5,portfolio_id=?6,buyer_id=?7,note=?8,status=?9,reminded=0 WHERE id=?1",
                params![id, vs(&v, "title"), vs(&v, "kind"), vs(&v, "start_at"), vi(&v, "duration_min").clamp(5, 600), vi(&v, "portfolio_id"), vi(&v, "buyer_id"), vs(&v, "note"), vs(&v, "status")],
            )?;
            ok()
        }
        ("DELETE", ["api", "appointments", id]) => {
            let id = id_of(id)?;
            let owner = cnt(db, "SELECT user_id FROM appointments WHERE id=?1", P![id]);
            if !me.is_lead() && owner != me.id {
                return Err(ApiErr::forbidden());
            }
            db.conn.execute("DELETE FROM appointments WHERE id=?1", params![id])?;
            ok()
        }

        // ================================================= İZİN / PUANTAJ
        ("GET", ["api", "leaves"]) => {
            let year = q.get("year").cloned().unwrap_or_else(|| now[..4].to_string());
            let like = format!("{year}%");
            let uid = if me.is_lead() { 0 } else { me.id };
            let sql = format!("SELECT l.*,{} AS user_name FROM leaves l WHERE (l.user_id=?1 OR ?1=0) AND (l.start_date LIKE ?2 OR l.end_date LIKE ?2) ORDER BY l.start_date DESC", uname("l.user_id"));
            out(Value::Array(db.rows(&sql, P![uid, like])?))
        }
        ("POST", ["api", "leaves"]) => {
            let v = body_json(body)?;
            let mut uid = vi(&v, "user_id");
            if uid <= 0 || (!me.is_lead() && uid != me.id) {
                uid = me.id;
            }
            let (s, e) = (vs(&v, "start_date"), vs(&v, "end_date"));
            let (sd, ed) = (NaiveDate::parse_from_str(&s, "%Y-%m-%d"), NaiveDate::parse_from_str(&e, "%Y-%m-%d"));
            if sd.is_err() || ed.is_err() || ed.unwrap() < sd.unwrap() {
                return Err(ApiErr::bad("Geçerli bir tarih aralığı girin."));
            }
            let status = "Bekliyor"; // her talep önce yönetici onayına gider
            db.conn.execute(
                "INSERT INTO leaves(user_id,kind,start_date,end_date,note,status,decided_by,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![uid, vs(&v, "kind"), s, e, vs(&v, "note"), status, 0, now],
            )?;
            {
                for u in db.list_users()?.iter().filter(|u| u.active && u.is_lead() && u.id != me.id) {
                    let _ = db.notify(u.id, "İzin talebi", &format!("{} izin talep etti: {} – {} ({})", me.full_name(), s, e, vs(&v, "kind")));
                }
            }
            ok()
        }
        ("PUT", ["api", "leaves", id, "status"]) => {
            need_lead(me)?;
            let id = id_of(id)?;
            let v = body_json(body)?;
            let status = vs(&v, "status");
            if !["Onaylandı", "Reddedildi", "Bekliyor"].contains(&status.as_str()) {
                return Err(ApiErr::bad("Geçersiz durum."));
            }
            db.conn.execute("UPDATE leaves SET status=?2, decided_by=?3 WHERE id=?1", params![id, status, me.id])?;
            let uid = cnt(db, "SELECT user_id FROM leaves WHERE id=?1", P![id]);
            let _ = db.notify(uid, "İzin", &format!("İzin talebiniz: {status}"));
            aud(db, me, "İzin kararı", &format!("#{id} {status}"));
            ok()
        }
        ("DELETE", ["api", "leaves", id]) => {
            let id = id_of(id)?;
            let (owner, status): (i64, String) = db
                .conn
                .query_row("SELECT user_id,status FROM leaves WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|_| ApiErr::not_found())?;
            if !me.is_lead() && !(owner == me.id && status == "Bekliyor") {
                return Err(ApiErr::forbidden());
            }
            db.conn.execute("DELETE FROM leaves WHERE id=?1", params![id])?;
            ok()
        }
        ("GET", ["api", "timesheet"]) => {
            let month = q.get("month").cloned().unwrap_or_else(|| now[..7].to_string());
            let first = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").map_err(|_| ApiErr::bad("Geçersiz ay."))?;
            let next = if first.month() == 12 { NaiveDate::from_ymd_opt(first.year() + 1, 1, 1).unwrap() } else { NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1).unwrap() };
            let last = next - CDur::days(1);
            let today = chrono::Local::now().date_naive();
            let until = if today < last { today } else { last };
            let work_start = db.kv_get("work_start").unwrap_or_else(|| "09:30".into());
            let mut expected = 0;
            let mut d = first;
            while d <= until {
                if d.weekday().number_from_monday() <= 5 {
                    expected += 1;
                }
                d += CDur::days(1);
            }
            let mut rows = Vec::new();
            for u in db.list_users()?.iter().filter(|u| u.active && (me.is_lead() || u.id == me.id)) {
                let like = format!("{month}%");
                let att = db.rows("SELECT check_in,check_out FROM attendance WHERE user_id=?1 AND check_in LIKE ?2", P![u.id, like])?;
                let mut days = std::collections::BTreeSet::new();
                let mut minutes = 0i64;
                let mut late = 0;
                for a in &att {
                    let ci = vs(a, "check_in");
                    let co = vs(a, "check_out");
                    if ci.len() >= 16 {
                        if days.insert(ci[..10].to_string()) && ci[11..16] > *work_start.as_str() {
                            late += 1;
                        }
                        if let (Ok(x), Ok(y)) = (
                            chrono::NaiveDateTime::parse_from_str(&ci, "%Y-%m-%d %H:%M:%S"),
                            chrono::NaiveDateTime::parse_from_str(&co, "%Y-%m-%d %H:%M:%S"),
                        ) {
                            minutes += (y - x).num_minutes().max(0);
                        }
                    }
                }
                let lv = db.rows("SELECT kind,start_date,end_date FROM leaves WHERE user_id=?1 AND status='Onaylandı' AND start_date<=?3 AND end_date>=?2", P![u.id, first.format("%Y-%m-%d").to_string(), last.format("%Y-%m-%d").to_string()])?;
                let (mut leave_days, mut sick_days) = (0, 0);
                for l in &lv {
                    let (Ok(s), Ok(e)) = (NaiveDate::parse_from_str(&vs(l, "start_date"), "%Y-%m-%d"), NaiveDate::parse_from_str(&vs(l, "end_date"), "%Y-%m-%d")) else { continue };
                    let mut d = s.max(first);
                    let end = e.min(last);
                    while d <= end {
                        if d.weekday().number_from_monday() <= 5 && d <= until {
                            leave_days += 1;
                            if vs(l, "kind").contains("Hastal") {
                                sick_days += 1;
                            }
                        }
                        d += CDur::days(1);
                    }
                }
                let absent = (expected - days.len() as i64 - leave_days).max(0);
                rows.push(json!({
                    "user_id": u.id, "user": u.full_name(), "days": days.len(), "hours": (minutes as f64 / 60.0 * 10.0).round() / 10.0,
                    "late": late, "leave_days": leave_days, "sick_days": sick_days, "expected": expected, "absent": absent,
                }));
            }
            out(json!({"month": month, "work_start": work_start, "rows": rows}))
        }

        // ================================================= KOMİSYON (yalnızca yönetici)
        ("GET", ["api", "commissions"]) => {
            need_admin(me)?;
            let month = q.get("month").cloned().unwrap_or_default();
            let like = format!("{month}%");
            let sql = format!(
                "SELECT c.*, {} AS user_name, COALESCE((SELECT title FROM portfolios p WHERE p.id=c.portfolio_id),'') AS portfolio_title FROM commissions c WHERE c.date LIKE ?1 ORDER BY c.date DESC, c.id DESC",
                uname("c.user_id")
            );
            out(Value::Array(db.rows(&sql, P![like])?))
        }
        ("POST", ["api", "commissions"]) | ("PUT", ["api", "commissions", _]) => {
            need_admin(me)?;
            let v = body_json(body)?;
            let deal = vf(&v, "deal_amount");
            let rate = vf(&v, "rate");
            let mut com = vf(&v, "commission");
            if com <= 0.0 && rate > 0.0 {
                com = deal * rate / 100.0;
            }
            let pct = vf(&v, "emp_pct");
            let emp = com * pct / 100.0;
            let date = if vs(&v, "date").is_empty() { now[..10].to_string() } else { vs(&v, "date") };
            let cur = if vs(&v, "currency").is_empty() { "TL".to_string() } else { vs(&v, "currency") };
            let status = if vs(&v, "status").is_empty() { "Bekliyor".to_string() } else { vs(&v, "status") };
            if method == "POST" {
                db.conn.execute(
                    "INSERT INTO commissions(portfolio_id,user_id,deal_type,deal_amount,currency,rate,commission,emp_pct,emp_amount,status,date,note,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                    params![vi(&v, "portfolio_id"), vi(&v, "user_id"), vs(&v, "deal_type"), deal, cur, rate, com, pct, emp, status, date, vs(&v, "note"), now],
                )?;
                aud(db, me, "Komisyon eklendi", &format!("{:.0} {}", com, cur));
            } else {
                let id = id_of(segs[2])?;
                db.conn.execute(
                    "UPDATE commissions SET portfolio_id=?2,user_id=?3,deal_type=?4,deal_amount=?5,currency=?6,rate=?7,commission=?8,emp_pct=?9,emp_amount=?10,status=?11,date=?12,note=?13 WHERE id=?1",
                    params![id, vi(&v, "portfolio_id"), vi(&v, "user_id"), vs(&v, "deal_type"), deal, cur, rate, com, pct, emp, status, date, vs(&v, "note")],
                )?;
                aud(db, me, "Komisyon güncellendi", &format!("#{id} {status}"));
            }
            ok()
        }
        ("DELETE", ["api", "commissions", id]) => {
            need_admin(me)?;
            db.conn.execute("DELETE FROM commissions WHERE id=?1", params![id_of(id)?])?;
            aud(db, me, "Komisyon silindi", id);
            ok()
        }
        ("GET", ["api", "commissions", "summary"]) => {
            need_admin(me)?;
            let like = format!("{}%", q.get("month").cloned().unwrap_or_default());
            let sql = format!(
                "SELECT {} AS user_name, c.currency AS currency, SUM(c.commission) AS total, SUM(c.emp_amount) AS emp_total, \
                 SUM(CASE WHEN c.status='Ödendi' THEN c.emp_amount ELSE 0 END) AS paid, SUM(CASE WHEN c.status<>'Ödendi' THEN c.emp_amount ELSE 0 END) AS pending, COUNT(*) AS n \
                 FROM commissions c WHERE c.date LIKE ?1 GROUP BY c.user_id, c.currency ORDER BY emp_total DESC",
                uname("c.user_id")
            );
            out(Value::Array(db.rows(&sql, P![like])?))
        }

        // ================================================= İŞLEM KAYDI
        ("GET", ["api", "audit"]) => {
            need_admin(me)?;
            let like = format!("%{}%", q.get("q").cloned().unwrap_or_default());
            let limit = q.get("limit").and_then(|s| s.parse::<i64>().ok()).unwrap_or(300).clamp(10, 2000);
            out(Value::Array(db.rows("SELECT id,user_name,action,detail,at FROM audit_log WHERE user_name||' '||action||' '||detail LIKE ?1 ORDER BY id DESC LIMIT ?2", P![like, limit])?))
        }

        // ================================================= KVKK (CRM)
        ("PUT", ["api", "crm", id, "consent"]) => {
            need_admin(me)?;
            let id = id_of(id)?;
            let v = body_json(body)?;
            let c = vb(&v, "consent");
            db.conn.execute(
                "UPDATE crm_contacts SET consent=?2, consent_date=CASE WHEN ?2=1 THEN ?3 ELSE '' END WHERE id=?1",
                params![id, c as i64, now[..10].to_string()],
            )?;
            aud(db, me, "KVKK: açık rıza güncellendi", &format!("CRM #{id} → {}", if c { "var" } else { "yok" }));
            ok()
        }
        ("GET", ["api", "crm", id, "export"]) => {
            need_admin(me)?;
            let id = id_of(id)?;
            let mut c = db.rows("SELECT id,full_name,phone,email,note,created_at,consent,consent_date,anonymized FROM crm_contacts WHERE id=?1", P![id])?;
            let ps = db.rows("SELECT id,title,listing_type,category,district,neighborhood,price,currency,status,created_at FROM portfolios WHERE crm_id=?1", P![id])?;
            aud(db, me, "KVKK: mülk sahibi verisi dışa aktarıldı", &format!("CRM #{id}"));
            let mut o = c.pop().unwrap_or(json!({}));
            o["portfolios"] = Value::Array(ps);
            out(o)
        }
        ("POST", ["api", "crm", id, "anonymize"]) => {
            need_admin(me)?;
            let id = id_of(id)?;
            db.conn.execute(
                "UPDATE crm_contacts SET full_name='Anonim kişi #'||id, phone='', email='', note='', consent=0, consent_date='', anonymized=1 WHERE id=?1",
                params![id],
            )?;
            aud(db, me, "KVKK: mülk sahibi anonimleştirildi", &format!("CRM #{id}"));
            ok()
        }

        // ================================================= AYARLAR (ek)
        ("GET", ["api", "settings", "ext"]) => {
            need_admin(me)?;
            out(json!({
                "ext_backup_dir": db.external_backup_dir(),
                "backup_keep": db.backup_keep(),
                "rclone_remote": db.kv_get("rclone_remote").unwrap_or_default(),
                "rclone_status": std::fs::read_to_string(db.backup_dir().join("rclone_son_durum.txt")).unwrap_or_default(),
                "safe_backup_dir": db.safe_backup_dir().to_string_lossy(),
                "ext_backup_overwrite": db.kv_get("ext_backup_overwrite").map(|s| s != "0").unwrap_or(true),
                "ext_backup_pw_set": !db.kv_get("ext_backup_pw").unwrap_or_default().is_empty(),
                "work_start": db.kv_get("work_start").unwrap_or_else(|| "09:30".into()),
                "match_enabled": db.kv_get("match_enabled").map(|s| s != "0").unwrap_or(true),
                "ip_allow": db.kv_get("ip_allow").unwrap_or_default(),
                "offline_enabled": db.kv_get("offline_enabled").map(|v| v == "1").unwrap_or(false),
                "remind_wa": db.kv_get("remind_wa").unwrap_or_else(|| crate::common::REMIND_DEFAULT.into()),
            }))
        }
        ("PUT", ["api", "settings", "ext"]) => {
            need_admin(me)?;
            let v = body_json(body)?;
            if v.get("ext_backup_dir").is_some() {
                db.kv_set("ext_backup_dir", &vs(&v, "ext_backup_dir"))?;
            }
            if v.get("backup_keep").is_some() {
                db.kv_set("backup_keep", &v["backup_keep"].as_i64().unwrap_or(14).to_string())?;
            }
            if v.get("rclone_remote").is_some() {
                db.kv_set("rclone_remote", &vs(&v, "rclone_remote"))?;
            }
            if v.get("safe_backup_dir").is_some() {
                db.kv_set("safe_backup_dir", &vs(&v, "safe_backup_dir"))?;
            }
            if v.get("ext_backup_overwrite").is_some() {
                db.kv_set("ext_backup_overwrite", if vb(&v, "ext_backup_overwrite") { "1" } else { "0" })?;
            }
            if vb(&v, "clear_pw") {
                db.kv_set("ext_backup_pw", "")?;
            } else if !vs(&v, "ext_backup_pw").is_empty() {
                db.kv_set("ext_backup_pw", &vs(&v, "ext_backup_pw"))?;
            }
            if !vs(&v, "work_start").is_empty() {
                db.kv_set("work_start", &vs(&v, "work_start"))?;
            }
            if v.get("match_enabled").is_some() {
                db.kv_set("match_enabled", if vb(&v, "match_enabled") { "1" } else { "0" })?;
            }
            if v.get("offline_enabled").is_some() {
                db.kv_set("offline_enabled", if vb(&v, "offline_enabled") { "1" } else { "0" })?;
            }
            if v.get("remind_wa").is_some() {
                db.kv_set("remind_wa", &vs(&v, "remind_wa"))?;
            }
            if v.get("ip_allow").is_some() {
                let raw = vs(&v, "ip_allow");
                let bad = crate::common::ip_allow_invalid(&raw);
                if !bad.is_empty() {
                    return Err(ApiErr::bad(&format!("Geçersiz IP/ağ: {}", bad.join(", "))));
                }
                db.kv_set("ip_allow", &raw)?;
            }
            aud(db, me, "Ayarlar değişti", "Yedekleme / mesai saati / IP kısıtlama");
            ok()
        }
        ("POST", ["api", "backup", "external"]) => {
            need_admin(me)?;
            let p = db.make_backup()?;
            let ext = db.copy_external(&p).map_err(|e| ApiErr(500, e))?;
            out(json!({"path": p.to_string_lossy(), "external": ext.map(|x| x.to_string_lossy().to_string())}))
        }

        // ================================================= GÜNCELLEME
        ("GET", ["api", "update", "info"]) => {
            let mut pk = Vec::new();
            for plat in ["linux", "windows"] {
                let ver = db.kv_get(&format!("update_{plat}_version")).unwrap_or_default();
                let f = st.data_dir.join("updates").join(plat);
                if !ver.is_empty() && f.exists() {
                    let sig = std::fs::read_to_string(st.data_dir.join("updates").join(format!("{plat}.imza"))).unwrap_or_default();
                    pk.push(json!({"platform": plat, "version": ver, "size": std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0), "sig": sig}));
                }
            }
            out(json!({"server_version": env!("CARGO_PKG_VERSION"), "packages": pk}))
        }
        ("POST", ["api", "update", "upload"]) => {
            need_admin(me)?;
            let plat = q.get("platform").cloned().unwrap_or_default();
            let ver = q.get("version").cloned().unwrap_or_default();
            if !["linux", "windows"].contains(&plat.as_str()) || ver.is_empty() || body.len() < 1_000_000 {
                return Err(ApiErr::bad("Geçersiz güncelleme paketi."));
            }
            let dir = st.data_dir.join("updates");
            std::fs::create_dir_all(&dir).map_err(|e| ApiErr(500, e.to_string()))?;
            // Güvenilir yayıncı anahtarı tanımlıysa sunucu da imzayı doğrular (ek güvence)
            let sig = q.get("sig").cloned().unwrap_or_default();
            let tk = crate::updater::trusted_key();
            if !tk.is_empty() {
                crate::sign::verify(&tk, &ver, &plat, body, &sig).map_err(ApiErr::bad)?;
            }
            std::fs::write(dir.join(&plat), body).map_err(|e| ApiErr(500, e.to_string()))?;
            let _ = std::fs::write(dir.join(format!("{plat}.imza")), &sig);
            db.kv_set(&format!("update_{plat}_version"), &ver)?;
            aud(db, me, "Güncelleme paketi yüklendi", &format!("{plat} v{ver}"));
            ok()
        }
        ("GET", ["api", "update", "download"]) => {
            let plat = q.get("platform").cloned().unwrap_or_default();
            if !["linux", "windows"].contains(&plat.as_str()) {
                return Err(ApiErr::bad("Geçersiz platform."));
            }
            let data = std::fs::read(st.data_dir.join("updates").join(&plat)).map_err(|_| ApiErr::not_found())?;
            Ok(Some(Resp::Bytes(data, "application/octet-stream")))
        }
        _ => Ok(None),
    }
}

/// Aynı kullanıcının aynı saatlere denk gelen başka randevusunu bulur.
fn conflict(db: &Db, uid: i64, start: &str, dur: i64, ignore: i64) -> Option<String> {
    let s = chrono::NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M").or_else(|_| chrono::NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S")).ok()?;
    let e = s + CDur::minutes(dur);
    let day = &start[..10];
    let rows = db.rows("SELECT id,title,start_at,duration_min FROM appointments WHERE user_id=?1 AND substr(start_at,1,10)=?2 AND status='Planlı' AND id<>?3", P![uid, day, ignore]).ok()?;
    for r in rows {
        let rs = chrono::NaiveDateTime::parse_from_str(&vs(&r, "start_at"), "%Y-%m-%d %H:%M").or_else(|_| chrono::NaiveDateTime::parse_from_str(&vs(&r, "start_at"), "%Y-%m-%d %H:%M:%S")).ok()?;
        let re = rs + CDur::minutes(vi(&r, "duration_min"));
        if rs < e && s < re {
            return Some(format!("\"{}\" ({}) ile aynı saatte.", vs(&r, "title"), &vs(&r, "start_at")[11..16]));
        }
    }
    None
}

/// Arka plan: randevu hatırlatması (30 dk kala) — sunucu açılışında bir kez başlatılır.
pub fn start_background(st: std::sync::Arc<State>) {
    let mut last_backup = std::time::Instant::now();
    let mut last_mirror = std::time::Instant::now();
    let mut first = true;
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let Ok(db) = st.db.lock() else { continue };
        // Günlük otomatik yedek (sunucu açık kaldığı sürece; en yeni 30 yedek tutulur, harici klasör ayarlıysa oraya da kopyalanır)
        if last_backup.elapsed() >= std::time::Duration::from_secs(24 * 3600) {
            last_backup = std::time::Instant::now();
            let _ = db.make_backup();
        }
        if last_mirror.elapsed() >= std::time::Duration::from_secs(120) || first {
            last_mirror = std::time::Instant::now();
            first = false;
            let _ = crate::mirror::sync(&db, &st.data_dir);
        }
        let now = chrono::Local::now().naive_local();
        let soon = (now + CDur::minutes(30)).format("%Y-%m-%d %H:%M").to_string();
        let nowf = now.format("%Y-%m-%d %H:%M").to_string();
        if let Ok(rows) = db.rows(
            "SELECT id,user_id,title,start_at FROM appointments WHERE status='Planlı' AND reminded=0 AND start_at<=?1 AND start_at>=?2",
            P![soon, nowf],
        ) {
            for r in rows {
                let _ = db.notify(vi(&r, "user_id"), "Randevu hatırlatma", &format!("{} — {}", vs(&r, "title"), &vs(&r, "start_at")[11..16.min(vs(&r, "start_at").len())]));
                let _ = db.conn.execute("UPDATE appointments SET reminded=1 WHERE id=?1", params![vi(&r, "id")]);
                st.rev.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });
}
