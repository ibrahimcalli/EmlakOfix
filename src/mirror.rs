//! Portföy fotoğraflarını ana bilgisayarda düzenli klasör yapısına yansıtır:
//! <veri>/Portfoyler/<Sahip>/<Satılık|Kiralık>/<No - Başlık>/NN_<id>.jpg
//! Asıl kayıt veritabanında kalır; klasör, kolay erişim ve ek yedek içindir.
use crate::db::Db;
use rusqlite::params;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn safe(s: &str, fallback: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
    let t = t.trim().trim_matches('.').trim();
    let t: String = t.chars().take(60).collect();
    if t.is_empty() { fallback.to_string() } else { t }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
}

/// Eksik dosyaları yazar, artık olmayanları siler. Yazılan dosya sayısını döndürür.
pub fn sync(db: &Db, data_dir: &Path) -> Result<usize, String> {
    let root = data_dir.join("Portfoyler");
    let mut stmt = db
        .conn
        .prepare(
            "SELECT ph.id, ph.position, p.id, p.title, p.listing_type, p.category, p.price, p.currency, p.district, p.neighborhood, \
             COALESCE(NULLIF((SELECT full_name FROM crm_contacts c WHERE c.id=p.crm_id),''), (SELECT COALESCE(NULLIF(first_name||' '||last_name,' '),username) FROM users u WHERE u.id=p.creator_id), 'Sahipsiz') \
             FROM photos ph JOIN portfolios p ON p.id=ph.portfolio_id ORDER BY p.id, ph.position",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, f64>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?, r.get::<_, String>(9)?, r.get::<_, String>(10)?))
        })
        .map_err(|e| e.to_string())?
        .flatten()
        .collect::<Vec<_>>();
    let mut keep: HashSet<PathBuf> = HashSet::new();
    let mut written = 0;
    let mut infos: HashSet<i64> = HashSet::new();
    for (i, (pid_photo, pos, pid, title, lt, cat, price, cur, dist, nb, owner)) in rows.iter().enumerate() {
        let lt = if lt.eq_ignore_ascii_case("Kiralık") { "Kiralik" } else { "Satilik" };
        let dir = root.join(safe(owner, "Sahipsiz")).join(lt).join(format!("{} - {}", pid, safe(title, "Portfoy")));
        let f = dir.join(format!("{:02}_{}.jpg", pos + 1, pid_photo));
        keep.insert(f.clone());
        if !f.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let data: Vec<u8> = db.conn.query_row("SELECT full FROM photos WHERE id=?1", params![pid_photo], |r| r.get(0)).map_err(|e| e.to_string())?;
            std::fs::write(&f, data).map_err(|e| e.to_string())?;
            written += 1;
        }
        if infos.insert(*pid) {
            let info = dir.join("bilgi.txt");
            let txt = format!("Portföy No: {pid}\nBaşlık: {title}\nTür: {lt}\nKategori: {cat}\nKonum: {nb} / {dist}\nFiyat: {price} {cur}\nSahip: {owner}\n");
            keep.insert(info.clone());
            let _ = std::fs::write(&info, txt);
        }
        let _ = i;
    }
    // Artıkları temizle (silinen/taşınan portföyler)
    let mut all = Vec::new();
    walk(&root, &mut all);
    for f in all {
        if !keep.contains(&f) {
            let _ = std::fs::remove_file(&f);
        }
    }
    prune_empty(&root);
    Ok(written)
}

fn prune_empty(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                prune_empty(&p);
                let _ = std::fs::remove_dir(&p); // yalnızca boşsa silinir
            }
        }
    }
}
