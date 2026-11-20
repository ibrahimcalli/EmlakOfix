//! Sunucu ve istemcinin ortak kullandığı veri yapıları ve sabitler.
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 7878;
pub const DISCOVERY_PORT: u16 = 47862;
pub const APP_NAME: &str = "Emlak Ofis";

pub const LISTING_TYPES: [&str; 2] = ["Satılık", "Kiralık"];
pub const CATEGORIES: [&str; 15] = [
    "Daire", "Villa", "Müstakil Ev", "Dubleks / Tripleks", "Rezidans", "Arsa", "Tarla",
    "Zeytinlik / Bağ-Bahçe", "Dükkan", "Ofis", "Depo", "Otel / Pansiyon", "Bina", "Devren", "Diğer",
];
pub const STATUSES: [&str; 5] = ["Taslak", "Yayında", "Satıldı", "Kiralandı", "Pasif"];
pub const HEATINGS: [&str; 8] = [
    "Belirtilmedi", "Yok", "Soba", "Doğalgaz (Kombi)", "Merkezi Sistem", "Klima", "Yerden Isıtma", "Güneş Enerjisi",
];
pub const DEEDS: [&str; 8] = [
    "Belirtilmedi", "Kat Mülkiyeti", "Kat İrtifakı", "Hisseli Tapu", "Müstakil Tapulu", "Tarla Tapulu", "Arsa Tapulu", "Zilyetlik",
];
pub const ROOMS: [&str; 12] = [
    "-", "Stüdyo", "1+0", "1+1", "2+1", "3+1", "4+1", "5+1", "6+1", "7+ ", "Açık Ofis", "Diğer",
];
pub const CURRENCIES: [&str; 4] = ["TL", "USD", "EUR", "GBP"];
pub const TASK_KINDS: [&str; 3] = ["Görev", "Toplantı", "Not"];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email: String,
    pub role: String,
    pub active: bool,
    pub has_photo: bool,
    /// İki aşamalı doğrulama açık mı (yalnızca yönetici listesinde ve kendi profilinde doldurulur)
    pub totp: bool,
}

impl UserInfo {
    pub fn full_name(&self) -> String {
        let n = format!("{} {}", self.first_name.trim(), self.last_name.trim());
        let n = n.trim().to_string();
        if n.is_empty() { self.username.clone() } else { n }
    }
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
    /// Yönetici veya ekip lideri
    pub fn is_lead(&self) -> bool {
        self.role == "admin" || self.role == "manager"
    }
    #[allow(dead_code)]
    pub fn role_label(&self) -> &'static str {
        role_label(&self.role)
    }
}

#[allow(dead_code)]
pub fn role_label(role: &str) -> &'static str {
    match role {
        "admin" => "Yönetici",
        "manager" => "Ekip lideri",
        _ => "Çalışan",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct LoginReply {
    pub token: String,
    pub user: UserInfo,
    /// Yönetici çevrimdışı giriş/yerel kopya özelliğini açtı mı
    pub offline_ok: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PortfolioSummary {
    pub id: i64,
    pub title: String,
    pub listing_type: String,
    pub category: String,
    pub district: String,
    pub neighborhood: String,
    pub price: f64,
    pub currency: String,
    pub gross_m2: f64,
    pub rooms: String,
    pub status: String,
    pub creator_id: i64,
    pub creator_name: String,
    pub created_at: String,
    pub cover_photo: Option<i64>,
    pub photo_count: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Portfolio {
    pub id: i64,
    pub title: String,
    pub listing_type: String,
    pub category: String,
    pub district: String,
    pub neighborhood: String,
    pub address: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub price: f64,
    pub currency: String,
    pub gross_m2: f64,
    pub net_m2: f64,
    pub rooms: String,
    pub building_age: String,
    pub floor: String,
    pub total_floors: String,
    pub heating: String,
    pub deed_status: String,
    pub furnished: bool,
    pub swap_ok: bool,
    pub credit_ok: bool,
    pub description: String,
    /// İlan metninin İngilizce ve Rusça sürümleri (Türkçe metin `description`)
    pub description_en: String,
    pub description_ru: String,
    pub status: String,
    // Mülk sahibi (yalnızca yönetici okuyabilir; kayıt sırasında herkes girebilir)
    pub owner_name: String,
    pub owner_phone: String,
    pub owner_email: String,
    pub owner_note: String,
    pub owner_consent: bool,
    pub crm_id: Option<i64>,
    // Salt okunur
    pub creator_id: i64,
    pub creator_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: String,
    pub closed_at: String,
    pub photos: Vec<i64>,
    /// Etiketler (Müstakil, Fırsat…)
    pub tags: Vec<String>,
    /// Web sitesi yayın durumu: '' | 'Bekliyor' | 'Yayında'
    pub web_status: String,
    pub web_at: String,
}

impl Portfolio {
    pub fn new_blank() -> Self {
        Portfolio {
            listing_type: "Satılık".into(),
            category: "Daire".into(),
            currency: "TL".into(),
            status: "Yayında".into(),
            heating: "Belirtilmedi".into(),
            deed_status: "Belirtilmedi".into(),
            rooms: "-".into(),
            ..Default::default()
        }
    }
    pub fn listing_no(&self) -> String {
        format!("P-{:05}", self.id)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Neighborhood {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct District {
    pub id: i64,
    pub name: String,
    pub neighborhoods: Vec<Neighborhood>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CrmPortfolioRef {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub district: String,
    pub neighborhood: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CrmContact {
    pub id: i64,
    pub full_name: String,
    pub phone: String,
    pub email: String,
    pub note: String,
    pub created_at: String,
    pub created_by_name: String,
    pub portfolios: Vec<CrmPortfolioRef>,
    pub consent: bool,
    pub consent_date: String,
    pub anonymized: bool,
    pub has_photo: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct TaskInfo {
    pub id: i64,
    pub title: String,
    pub details: String,
    pub kind: String,
    pub assigned_to: i64,
    pub assignee_name: String,
    pub due_at: String,
    pub status: String,
    pub created_by_name: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct NewTask {
    pub title: String,
    pub details: String,
    pub kind: String,
    pub assignees: Vec<i64>,
    pub due_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct NotifInfo {
    pub id: i64,
    pub kind: String,
    pub message: String,
    pub created_at: String,
    pub read: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct AttInfo {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub check_in: String,
    pub check_out: String,
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct GoalInfo {
    pub id: i64,
    pub title: String,
    /// 0 = ekip hedefi (tüm çalışanların toplamı)
    pub user_id: i64,
    pub user_name: String,
    pub period_start: String,
    pub period_end: String,
    pub t_portfolio: i64,
    pub t_published: i64,
    pub t_rented: i64,
    pub t_sold: i64,
    pub p_portfolio: i64,
    pub p_published: i64,
    pub p_rented: i64,
    pub p_sold: i64,
    pub t_web: i64,
    pub p_web: i64,
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct OutputSettings {
    pub office_name: String,
    pub tagline: String,
    pub office_phone: String,
    pub office_address: String,
    pub footer_note: String,
    pub accent: String,
    pub show_logo: bool,
    pub show_office_contact: bool,
    pub show_listing_no: bool,
    pub show_date: bool,
    pub show_price: bool,
    pub show_address: bool,
    pub show_map_link: bool,
    pub show_details: bool,
    pub show_description: bool,
    pub show_photos: bool,
    pub max_photos: i64,
    pub show_agent_photo: bool,
    pub show_agent_name: bool,
    pub show_agent_phone: bool,
    pub show_agent_email: bool,
    pub watermark: bool,
    /// Antetli (önceden basılı) kağıda yazdırma: logo/ofis adı üst bilgisi basılmaz, üstte boşluk bırakılır
    pub letterhead: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        OutputSettings {
            office_name: "Emlak Ofisi".into(),
            tagline: String::new(),
            office_phone: String::new(),
            office_address: String::new(),
            footer_note: "Bu belge bilgilendirme amaçlıdır; fiyat ve bilgiler önceden haber verilmeksizin değişebilir.".into(),
            accent: "#1F4E79".into(),
            show_logo: true,
            show_office_contact: true,
            show_listing_no: true,
            show_date: true,
            show_price: true,
            show_address: true,
            show_map_link: true,
            show_details: true,
            show_description: true,
            show_photos: true,
            max_photos: 8,
            show_agent_photo: true,
            show_agent_name: true,
            show_agent_phone: true,
            show_agent_email: true,
            watermark: false,
            letterhead: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ServerInfo {
    pub data_dir: String,
    pub db_size_mb: f64,
    pub backups: Vec<String>,
    pub addresses: Vec<String>,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct RevReply {
    pub rev: u64,
    pub unread: i64,
}

// ---- Yardımcı fonksiyonlar ----

/// 1250000 -> "1.250.000"
pub fn fmt_thousands(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let mut out: String = out.chars().rev().collect();
    if n < 0 {
        out.insert(0, '-');
    }
    out
}

pub fn fmt_price(price: f64, currency: &str) -> String {
    if price <= 0.0 {
        return "Fiyat sorunuz".into();
    }
    let cur = if currency.is_empty() { "TL" } else { currency };
    format!("{} {}", fmt_thousands(price), cur)
}

pub fn fmt_m2(v: f64) -> String {
    if v <= 0.0 {
        String::new()
    } else if (v - v.round()).abs() < 0.001 {
        format!("{} m²", v.round() as i64)
    } else {
        format!("{:.1} m²", v)
    }
}

pub fn map_url(lat: f64, lon: f64) -> String {
    format!("https://www.google.com/maps?q={:.6},{:.6}", lat, lon)
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn only_digits(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Telefonu WhatsApp için uluslararası biçime çevirir (Türkiye varsayılır).
pub fn wa_number(phone: &str) -> String {
    let d = only_digits(phone);
    if d.is_empty() {
        return d;
    }
    if d.starts_with("00") {
        d[2..].to_string()
    } else if d.starts_with('0') {
        format!("90{}", &d[1..])
    } else if d.len() == 10 {
        format!("90{}", d)
    } else {
        d
    }
}

pub fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Google Maps vb. bağlantıdan veya "lat, lon" metninden koordinat ayıklar.
pub fn parse_coords(text: &str) -> Option<(f64, f64)> {
    let t = text.trim();
    // !3d36.62!4d29.11
    if let (Some(a), Some(b)) = (t.rfind("!3d"), t.rfind("!4d")) {
        let lat: String = t[a + 3..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
        let lon: String = t[b + 3..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
        if let (Ok(la), Ok(lo)) = (lat.parse::<f64>(), lon.parse::<f64>()) {
            return Some((la, lo));
        }
    }
    // @36.62,29.11 veya q=36.62,29.11 veya düz "36.62, 29.11"
    let cleaned = if let Some(i) = t.find('@') {
        t[i + 1..].to_string()
    } else if let Some(i) = t.find("q=") {
        t[i + 2..].to_string()
    } else if let Some(i) = t.find("ll=") {
        t[i + 3..].to_string()
    } else {
        t.to_string()
    };
    let cleaned = cleaned.replace("%2C", ",").replace("%2c", ",");
    let mut nums = Vec::new();
    let mut cur = String::new();
    for c in cleaned.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                nums.push(std::mem::take(&mut cur));
            }
            if nums.len() >= 2 {
                break;
            }
        }
    }
    if !cur.is_empty() {
        nums.push(cur);
    }
    if nums.len() >= 2 {
        if let (Ok(la), Ok(lo)) = (nums[0].parse::<f64>(), nums[1].parse::<f64>()) {
            if la.abs() <= 90.0 && lo.abs() <= 180.0 {
                return Some((la, lo));
            }
        }
    }
    None
}

/// Varsayılan hatırlatma metni. {ad}, {baslik}, {saat}, {ofis} yer tutucuları.
pub const REMIND_DEFAULT: &str = "Merhaba {ad}, {tarih} saat {saat} için {baslik} randevunuzu hatırlatırız. — {ofis}";

fn parse_ip(s: &str) -> Option<std::net::IpAddr> {
    s.trim().parse().ok()
}

/// "192.168.1.0/24", "10.0.0.5", "::1" gibi kayıtlardan biri `ip` ile eşleşiyor mu?
pub fn ip_matches(rule: &str, ip: std::net::IpAddr) -> bool {
    let rule = rule.trim();
    if let Some((net, bits)) = rule.split_once('/') {
        let (Some(n), Ok(b)) = (parse_ip(net), bits.trim().parse::<u32>()) else { return false };
        match (n, ip) {
            (std::net::IpAddr::V4(n), std::net::IpAddr::V4(i)) if b <= 32 => {
                let mask = if b == 0 { 0 } else { u32::MAX << (32 - b) };
                u32::from(n) & mask == u32::from(i) & mask
            }
            (std::net::IpAddr::V6(n), std::net::IpAddr::V6(i)) if b <= 128 => {
                let mask = if b == 0 { 0 } else { u128::MAX << (128 - b) };
                u128::from(n) & mask == u128::from(i) & mask
            }
            _ => false,
        }
    } else {
        parse_ip(rule).map(|r| r == ip).unwrap_or(false)
    }
}

fn ip_rules(list: &str) -> Vec<String> {
    list.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

/// Geçersiz kayıtların listesi (boşsa hepsi geçerli).
pub fn ip_allow_invalid(list: &str) -> Vec<String> {
    ip_rules(list)
        .into_iter()
        .filter(|r| {
            let ok = match r.split_once('/') {
                Some((n, b)) => parse_ip(n).is_some() && b.trim().parse::<u32>().is_ok(),
                None => parse_ip(r).is_some(),
            };
            !ok
        })
        .collect()
}

/// Liste boşsa herkese açık; yerel (loopback) bağlantı her zaman serbesttir.
pub fn ip_allowed(list: &str, ip: &str) -> bool {
    let rules = ip_rules(list);
    if rules.is_empty() {
        return true;
    }
    let Some(addr) = parse_ip(ip) else { return false };
    let addr = match addr {
        std::net::IpAddr::V6(v6) => v6.to_ipv4_mapped().map(std::net::IpAddr::V4).unwrap_or(addr),
        a => a,
    };
    addr.is_loopback() || rules.iter().any(|r| ip_matches(r, addr))
}

/// Lisans süresi dolunca true; sunucu istekleri reddedilir.
pub static LICENSE_LOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
