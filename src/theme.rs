//! Görünüm: tema (açık/koyu), vurgu ve buton renkleri, font, yazı boyutu.
//! Ayarlar her bilgisayarda yerel olarak saklanır (ayarlar.json).
use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct Look {
    pub dark: bool,
    /// Ana renk (#RRGGBB): başlıklar, seçili menü, bağlantılar
    pub accent: String,
    /// Buton rengi (#RRGGBB); boşsa ana renkle aynı
    pub button: String,
    /// "varsayilan" | "dejavu" | "liberation" | "sistem:<ad>"
    pub font: String,
    /// Yüzde: 85 – 150
    pub scale: u32,
    pub lang: String,
    /// "klasik" | "altin" (Altın & Lacivert) | "gece" (derin koyu)
    pub skin: String,
    /// Giriş ekranı arka planı: "gunbatimi" | "gol" | "ozel" | "yok"
    pub bg: String,
    /// Programın zemin rengi (#RRGGBB); boşsa temanın kendi zemini
    pub surface: String,
}

impl Default for Look {
    fn default() -> Self {
        Look { dark: false, accent: "#1F4E79".into(), button: String::new(), font: "varsayilan".into(), scale: 100, lang: "tr".into(), skin: "klasik".into(), bg: "gunbatimi".into(), surface: String::new() }
    }
}

pub const PRESETS: [(&str, &str); 8] = [
    ("Lacivert", "#1F4E79"),
    ("Mavi", "#2563EB"),
    ("Turkuaz", "#0E7490"),
    ("Yeşil", "#15803D"),
    ("Turuncu", "#C2410C"),
    ("Bordo", "#9F1239"),
    ("Mor", "#6D28D9"),
    ("Antrasit", "#374151"),
];

pub const SURFACES: [(&str, &str); 12] = [
    ("Açık gri", "#F3F5F9"),
    ("Pastel mavi", "#E4EEFA"),
    ("Pastel yeşil", "#E4F4E9"),
    ("Pastel turuncu", "#FDF0E2"),
    ("Pastel pembe", "#FBE8EE"),
    ("Pastel mor", "#EEE9FA"),
    ("Krem", "#FAF3E3"),
    ("Taş", "#E6E8EC"),
    ("Antrasit", "#15181E"),
    ("Gece mavisi", "#0E192E"),
    ("Siyah", "#0B0C0F"),
    ("Koyu yeşil", "#10201A"),
];

pub const SCALES: [(&str, u32); 4] = [("Küçük", 90), ("Normal", 100), ("Büyük", 115), ("Çok büyük", 130)];

pub fn parse_hex(h: &str) -> Option<Color32> {
    let h = h.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

fn lum(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// Verilen zemin üzerinde okunaklı yazı rengi.
pub fn on_color(bg: Color32) -> Color32 {
    if lum(bg) > 0.62 { Color32::from_rgb(20, 24, 32) } else { Color32::WHITE }
}

#[derive(Clone)]
struct Palette {
    dark: bool,
    skin: u8,
    accent: Color32,
    button: Color32,
    surface: Option<Color32>,
}

static PAL: RwLock<Palette> = RwLock::new(Palette { dark: false, skin: 0, accent: Color32::from_rgb(31, 78, 121), button: Color32::from_rgb(31, 78, 121), surface: None });

fn pal() -> Palette {
    PAL.read().map(|p| p.clone()).unwrap_or(Palette { dark: false, skin: 0, accent: Color32::from_rgb(31, 78, 121), button: Color32::from_rgb(31, 78, 121), surface: None })
}

fn pick(light: Color32, dark: Color32) -> Color32 {
    if pal().dark { dark } else { light }
}

/// Görünüm paketi (0 klasik, 1 altın&lacivert, 2 gece) — koyu temada ayrı renkler.
fn sk(light: Color32, dark: Color32, gold: Color32, night: Color32) -> Color32 {
    let p = pal();
    if !p.dark {
        light
    } else {
        match p.skin {
            1 => gold,
            2 => night,
            _ => dark,
        }
    }
}
fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
/// Altın (#D4AF37) ve lacivert uyumlu görünümde ikincil mavi vurgu.
#[allow(dead_code)]
pub fn blue_accent() -> Color32 {
    rgb(74, 128, 214)
}
pub const SKINS: [(&str, &str); 6] = [
    ("klasik", "Klasik"),
    ("altin", "Altın & Lacivert"),
    ("gece", "Gece (derin koyu)"),
    ("pastel_mavi", "Açık pastel – Mavi"),
    ("pastel_yesil", "Açık pastel – Yeşil"),
    ("pastel_turuncu", "Açık pastel – Turuncu"),
];

/// Açık pastel paketler (skin 3 mavi, 4 yeşil, 5 turuncu): (panel, kart, hover, çizgi, küçük resim, yeni, bitmiş, faint, ib, hb, ab)
fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Kullanıcının seçtiği zemin rengi varsa tüm yüzey renklerini ondan türetir.
fn derived(s: Color32, accent: Color32) -> [Color32; 11] {
    let (w, k) = (Color32::WHITE, Color32::BLACK);
    if lum(s) > 0.5 {
        [s, mix(s, w, 0.7), mix(s, accent, 0.12), mix(s, k, 0.12), mix(s, k, 0.07), mix(s, accent, 0.16), mix(s, w, 0.5), mix(s, w, 0.55), mix(s, k, 0.06), mix(s, k, 0.1), mix(s, k, 0.17)]
    } else {
        [s, mix(s, w, 0.06), mix(s, w, 0.11), mix(s, w, 0.16), mix(s, w, 0.09), mix(s, accent, 0.25), mix(s, k, 0.15), mix(s, w, 0.03), mix(s, w, 0.08), mix(s, w, 0.13), mix(s, w, 0.2)]
    }
}

/// Kenar çubuğu zemini açık mı (pastel paketler veya açık özel zemin)?
fn side_light() -> bool {
    let p = pal();
    match p.surface {
        Some(s) => lum(s) > 0.5,
        None => !p.dark, // açık temalarda yan menü de açık tonda
    }
}

fn pastel() -> Option<[Color32; 11]> {
    let p = pal();
    if let Some(s) = p.surface {
        return Some(derived(s, p.accent));
    }
    if p.dark {
        return None;
    }
    match p.skin {
        3 => Some([rgb(228, 238, 250), rgb(246, 250, 255), rgb(219, 234, 252), rgb(190, 210, 235), rgb(208, 223, 242), rgb(206, 228, 255), rgb(236, 242, 250), rgb(238, 245, 253), rgb(212, 226, 244), rgb(198, 216, 240), rgb(180, 202, 232)]),
        4 => Some([rgb(228, 244, 233), rgb(247, 252, 248), rgb(219, 242, 226), rgb(190, 222, 200), rgb(208, 232, 214), rgb(206, 240, 218), rgb(238, 246, 240), rgb(238, 248, 241), rgb(212, 236, 220), rgb(198, 228, 208), rgb(178, 216, 192)]),
        5 => Some([rgb(253, 240, 226), rgb(255, 250, 244), rgb(255, 234, 212), rgb(240, 216, 188), rgb(246, 226, 205), rgb(255, 228, 196), rgb(251, 243, 234), rgb(255, 246, 237), rgb(250, 228, 208), rgb(246, 215, 186), rgb(238, 200, 164)]),
        _ => None,
    }
}
pub const GOLD: &str = "#D4AF37";

#[allow(dead_code)]
pub fn is_dark() -> bool {
    pal().dark
}
pub fn accent() -> Color32 {
    let p = pal();
    if p.dark && lum(p.accent) < 0.3 { lighten(p.accent, 0.35) } else { p.accent }
}
pub fn btn() -> Color32 {
    pal().button
}
pub fn on_btn() -> Color32 {
    on_color(pal().button)
}
pub fn muted() -> Color32 {
    sk(rgb(105, 112, 125), rgb(150, 158, 172), rgb(150, 166, 194), rgb(150, 156, 168))
}
pub fn t_strong() -> Color32 {
    sk(rgb(24, 29, 38), rgb(236, 239, 244), rgb(240, 243, 250), rgb(238, 240, 244))
}
pub fn t_mid() -> Color32 {
    sk(rgb(70, 77, 90), rgb(196, 202, 213), rgb(200, 211, 230), rgb(198, 203, 212))
}
pub fn t_dim() -> Color32 {
    sk(rgb(120, 127, 140), rgb(145, 152, 166), rgb(140, 156, 184), rgb(140, 146, 158))
}
pub fn panel() -> Color32 {
    if let Some(t) = pastel() {
        return t[0];
    }
    sk(rgb(243, 245, 249), rgb(21, 24, 30), rgb(14, 25, 46), rgb(11, 12, 15))
}
pub fn card() -> Color32 {
    if let Some(t) = pastel() {
        return t[1];
    }
    sk(Color32::WHITE, rgb(33, 37, 46), rgb(21, 35, 62), rgb(22, 24, 29))
}
pub fn card_hover() -> Color32 {
    if let Some(t) = pastel() {
        return t[2];
    }
    sk(rgb(240, 246, 254), rgb(42, 48, 60), rgb(26, 41, 70), rgb(31, 34, 41))
}
pub fn stroke() -> Color32 {
    if let Some(t) = pastel() {
        return t[3];
    }
    sk(rgb(220, 224, 232), rgb(58, 64, 77), rgb(38, 54, 86), rgb(44, 48, 57))
}
pub fn thumb_bg() -> Color32 {
    if let Some(t) = pastel() {
        return t[4];
    }
    sk(rgb(228, 231, 237), rgb(45, 50, 60), rgb(25, 38, 64), rgb(34, 37, 44))
}
pub fn note_bg() -> Color32 {
    pick(Color32::from_rgb(255, 248, 230), Color32::from_rgb(58, 50, 30))
}
pub fn new_bg() -> Color32 {
    if let Some(t) = pastel() {
        return t[5];
    }
    sk(rgb(232, 243, 255), rgb(30, 48, 72), rgb(22, 48, 92), rgb(26, 40, 62))
}
pub fn done_bg() -> Color32 {
    if let Some(t) = pastel() {
        return t[6];
    }
    sk(rgb(246, 247, 249), rgb(27, 31, 38), rgb(14, 24, 43), rgb(18, 20, 24))
}
pub fn sidebar_bg() -> Color32 {
    if side_light() {
        return mix(panel(), Color32::BLACK, 0.07);
    }
    if let Some(t) = pastel() {
        return mix(t[0], Color32::BLACK, 0.35);
    }
    sk(rgb(23, 32, 48), rgb(14, 17, 22), rgb(3, 7, 15), rgb(7, 8, 10))
}
pub fn nav_sel() -> Color32 {
    let a = pal().accent;
    if pal().skin == 1 && pal().dark {
        return rgb(212, 175, 55);
    }
    if lum(a) < 0.16 { lighten(a, 0.3) } else { a }
}
pub fn sidebar_text() -> Color32 {
    if side_light() {
        return rgb(40, 48, 62);
    }
    sk(rgb(205, 213, 226), rgb(205, 213, 226), rgb(222, 229, 242), rgb(214, 218, 226))
}
pub fn sidebar_dim() -> Color32 {
    if side_light() {
        return rgb(95, 106, 122);
    }
    sk(rgb(128, 140, 160), rgb(128, 140, 160), rgb(190, 160, 80), rgb(120, 128, 142))
}
/// Grafik ve çubuk renkleri: koyu temada daha parlak, okunaklı tonlar.
pub fn chart(i: usize) -> Color32 {
    let light = [rgb(45, 100, 190), rgb(34, 139, 70), rgb(130, 70, 170), rgb(190, 55, 55), rgb(200, 130, 20), rgb(31, 120, 100)];
    let dark = [rgb(96, 150, 235), rgb(70, 190, 115), rgb(180, 130, 225), rgb(240, 100, 100), rgb(240, 175, 60), rgb(70, 190, 165)];
    let gold = [rgb(212, 175, 55), rgb(86, 156, 245), rgb(70, 200, 135), rgb(235, 110, 110), rgb(170, 140, 235), rgb(90, 205, 215)];
    let v = if !pal().dark { &light } else if pal().skin == 1 { &gold } else { &dark };
    v[i % 6]
}

fn lighten(c: Color32, k: f32) -> Color32 {
    let f = |v: u8| (v as f32 + (255.0 - v as f32) * k) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

/// Birincil düğme çizimi için ortak çerçeve (kart).
pub fn card_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(card())
        .stroke(Stroke::new(1.0_f32, stroke()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(12.0)
}

// ------------------------------------------------------------------ yazı tipleri
const DEJAVU: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
const LIBERATION: &[u8] = include_bytes!("../assets/fonts/LiberationSans-Regular.ttf");

/// (görünen ad, anahtar, dosya yolları)
fn system_candidates() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    vec![
        ("Segoe UI", "sistem:segoe", vec!["C:\\Windows\\Fonts\\segoeui.ttf"]),
        ("Calibri", "sistem:calibri", vec!["C:\\Windows\\Fonts\\calibri.ttf"]),
        ("Arial", "sistem:arial", vec!["C:\\Windows\\Fonts\\arial.ttf"]),
        ("Tahoma", "sistem:tahoma", vec!["C:\\Windows\\Fonts\\tahoma.ttf"]),
        ("Verdana", "sistem:verdana", vec!["C:\\Windows\\Fonts\\verdana.ttf"]),
        (
            "Noto Sans",
            "sistem:noto",
            vec!["/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", "/usr/share/fonts/noto/NotoSans-Regular.ttf", "/usr/share/fonts/TTF/NotoSans-Regular.ttf"],
        ),
        ("Ubuntu", "sistem:ubuntu", vec!["/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf", "/usr/share/fonts/ubuntu/Ubuntu-R.ttf"]),
        ("Open Sans", "sistem:opensans", vec!["/usr/share/fonts/truetype/open-sans/OpenSans-Regular.ttf"]),
        ("FreeSans", "sistem:freesans", vec!["/usr/share/fonts/truetype/freefont/FreeSans.ttf"]),
    ]
}

/// Bu bilgisayarda gerçekten bulunan yazı tipleri: (görünen ad, anahtar)
pub fn available_fonts() -> Vec<(String, String)> {
    let mut v = vec![
        ("Varsayılan (Ubuntu Light)".to_string(), "varsayilan".to_string()),
        ("DejaVu Sans (geniş, okunaklı)".to_string(), "dejavu".to_string()),
        ("Liberation Sans (Arial benzeri)".to_string(), "liberation".to_string()),
    ];
    for (name, key, paths) in system_candidates() {
        if paths.iter().any(|p| std::path::Path::new(p).exists()) {
            v.push((format!("{name} (sistem)"), key.to_string()));
        }
    }
    v
}

fn build_fonts(key: &str) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let bytes: Option<Vec<u8>> = match key {
        "dejavu" => Some(DEJAVU.to_vec()),
        "liberation" => Some(LIBERATION.to_vec()),
        k if k.starts_with("sistem:") => system_candidates().into_iter().find(|(_, kk, _)| *kk == k).and_then(|(_, _, paths)| paths.iter().find_map(|p| std::fs::read(p).ok())),
        _ => None,
    };
    if let Some(b) = bytes {
        fonts.font_data.insert("secilen".into(), Arc::new(FontData::from_owned(b)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "secilen".into());
    }
    fonts
}

// ------------------------------------------------------------------ uygulama
pub fn apply(ctx: &egui::Context, look: &Look) {
    ctx.set_fonts(build_fonts(&look.font));
    apply_fast(ctx, look);
}

/// Yazı tipini yeniden yüklemeden renk/tema/boyut uygular (renk seçerken akıcı kalsın diye).
pub fn apply_fast(ctx: &egui::Context, look: &Look) {
    let skin: u8 = match look.skin.as_str() {
        "altin" => 1,
        "gece" => 2,
        "pastel_mavi" => 3,
        "pastel_yesil" => 4,
        "pastel_turuncu" => 5,
        _ => 0,
    };
    let surface = parse_hex(&look.surface);
    let dark_mode = match surface {
        Some(sf) => lum(sf) < 0.5,
        None => if skin >= 3 { false } else { look.dark || skin != 0 },
    };
    let mut accent = parse_hex(&look.accent).unwrap_or(Color32::from_rgb(31, 78, 121));
    let mut button = accent; // buton rengi = ana renk
    if skin == 1 {
        accent = parse_hex(GOLD).unwrap();
        button = accent;
    }
    if let Ok(mut p) = PAL.write() {
        *p = Palette { dark: dark_mode, skin, accent, button, surface };
    }
    let look = &Look { dark: dark_mode, ..look.clone() };
    ctx.set_zoom_factor((look.scale.clamp(80, 160) as f32) / 100.0);

    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Heading, FontId::new(23.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 26.0;
    ctx.set_style(style);

    let mut v = if look.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = panel();
    v.window_fill = card();
    v.extreme_bg_color = if let Some(sf) = surface { if look.dark { mix(sf, Color32::BLACK, 0.3) } else { mix(sf, Color32::WHITE, 0.85) } } else if look.dark { sk(Color32::WHITE, rgb(16, 19, 24), rgb(7, 13, 26), rgb(9, 10, 12)) } else { Color32::WHITE };
    v.faint_bg_color = if let Some(t) = pastel() { t[7] } else if look.dark { sk(Color32::WHITE, rgb(28, 32, 40), rgb(14, 24, 44), rgb(18, 20, 24)) } else { Color32::from_rgb(248, 249, 251) };
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0_f32, on_color(accent));
    v.hyperlink_color = if look.dark { lighten(accent, 0.45) } else { accent };
    v.window_stroke = Stroke::new(1.0_f32, stroke());
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    let r = CornerRadius::same(6);
    let txt = t_strong();
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, stroke());
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, if look.dark { Color32::from_rgb(210, 216, 226) } else { Color32::from_rgb(50, 56, 68) });
    let (ib, hb, ab) = if look.dark {
        match skin {
            1 => (rgb(27, 42, 72), rgb(36, 56, 94), rgb(46, 70, 114)),
            2 => (rgb(38, 41, 49), rgb(50, 54, 64), rgb(62, 67, 79)),
            _ => (rgb(48, 54, 66), rgb(62, 69, 84), rgb(74, 82, 100)),
        }
    } else if let Some(t) = pastel() {
        (t[8], t[9], t[10])
    } else {
        (Color32::from_rgb(232, 236, 243), Color32::from_rgb(220, 227, 238), Color32::from_rgb(205, 214, 230))
    };
    v.widgets.inactive.corner_radius = r;
    v.widgets.inactive.weak_bg_fill = ib;
    v.widgets.inactive.bg_fill = ib;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, txt);
    v.widgets.hovered.corner_radius = r;
    v.widgets.hovered.weak_bg_fill = hb;
    v.widgets.hovered.bg_fill = hb;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, txt);
    v.widgets.active.corner_radius = r;
    v.widgets.active.weak_bg_fill = ab;
    v.widgets.active.bg_fill = ab;
    v.widgets.active.fg_stroke = Stroke::new(1.5_f32, txt);
    v.widgets.open.corner_radius = r;
    ctx.set_visuals(v);
}
