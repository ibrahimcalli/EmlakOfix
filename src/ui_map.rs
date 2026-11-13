//! Basit harita: portföy koordinatlarını çevrimdışı çizer; nokta tıklanınca portföy açılır,
//! isteğe bağlı olarak OpenStreetMap'te açılır (internet gerekir).
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::ui_ext::{jf, ji, js, panel_box};
use eframe::egui::{self, vec2, CornerRadius, FontId, RichText, Sense, Stroke, Ui};
use serde_json::Value;

pub use crate::map_tiles as tiles;
use crate::map_tiles::Source;

#[derive(Default)]
pub struct MapState {
    pub sel: i64,
    /// Harita merkezi (boylam, enlem) ve kesirli yakınlaştırma düzeyi
    pub z: f64,
    pub cx: f64,
    pub cy: f64,
    /// 0 sokak, 1 uydu, 2 altlıksız (çevrimdışı)
    pub src: u8,
    pub store: crate::map_tiles::Store,
    pub init: bool,
    /// true = tüm portföyler, false = yalnızca benimkiler
    pub all: bool,
    pub all_set: bool,
    pub show_sale: bool,
    pub show_rent: bool,
    pub show_land: bool,
    pub labels: bool,
}

/// Haritada işaret türü: 0 satılık, 1 kiralık, 2 arazi
pub fn kind_of(p: &Value) -> usize {
    let cat = js(p, "category");
    if matches!(cat.as_str(), "Arsa" | "Tarla" | "Zeytinlik / Bağ-Bahçe") {
        2
    } else if js(p, "listing_type") == "Kiralık" {
        1
    } else {
        0
    }
}

const KIND_NAMES: [&'static str; 3] = ["Satılık", "Kiralık", "Arazi"];
fn kind_color(k: usize) -> egui::Color32 {
    match k {
        0 => egui::Color32::from_rgb(214, 69, 65),
        1 => egui::Color32::from_rgb(52, 120, 220),
        _ => egui::Color32::from_rgb(46, 160, 90),
    }
}

fn short_price(v: f64, cur: &str) -> String {
    if v <= 0.0 {
        return String::new();
    }
    let c = if cur.is_empty() { "TL" } else { cur };
    if v >= 1_000_000.0 {
        format!("{} M {}", format!("{:.1}", v / 1_000_000.0).replace(".0", "").replace('.', ","), c)
    } else if v >= 1_000.0 {
        format!("{} B {}", (v / 1_000.0).round() as i64, c)
    } else {
        format!("{} {}", v as i64, c)
    }
}

/// İşaret etiketi: "Villa · Ölüdeniz · 12,5 M TL"
fn short_label(p: &Value) -> String {
    let place = {
        let n = js(p, "neighborhood");
        if n.is_empty() { js(p, "district") } else { n }
    };
    let mut parts = vec![js(p, "category")];
    if !place.is_empty() {
        parts.push(place);
    }
    let pr = short_price(jf(p, "price"), &js(p, "currency"));
    if !pr.is_empty() {
        parts.push(pr);
    }
    parts.join(" · ")
}

impl App {
    pub fn load_map(&mut self) {
        self.call(|api| api.get::<Vec<Value>>("/api/map"), |app, v| {
            app.ext.map_pts = v;
            app.ext.map.init = false;
        });
    }

    pub fn ui_map(&mut self, ui: &mut Ui) {
        page_title(ui, "Harita");
        if !self.ext.map.all_set {
            let m = &mut self.ext.map;
            m.all_set = true;
            m.show_sale = true;
            m.show_rent = true;
            m.show_land = true;
            m.labels = true;
            m.src = 0;
        }
        let my_id = self.me.id;
        let total = self.ext.map_pts.len();
        let mine_n = self.ext.map_pts.iter().filter(|p| ji(p, "creator_id") == my_id).count();
        let mut refit = false;
        ui.horizontal_wrapped(|ui| {
            let m = &mut self.ext.map;
            if ui.selectable_label(!m.all, format!("{} ({mine_n})", ti("Benim portföylerim"))).clicked() && m.all {
                m.all = false;
                refit = true;
            }
            if ui.selectable_label(m.all, format!("{} ({total})", ti("Hepsi"))).clicked() && !m.all {
                m.all = true;
                refit = true;
            }
            ui.separator();
            for (k, flag) in [(0usize, &mut m.show_sale), (1, &mut m.show_rent), (2, &mut m.show_land)] {
                let c = kind_color(k);
                let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, c);
                ui.checkbox(flag, ti(KIND_NAMES[k]));
            }
            ui.separator();
            ui.checkbox(&mut m.labels, ti("Etiketler"));
            ui.separator();
            ui.label(ti("Altlık:"));
            for (i, n) in ["Sokak", "Uydu", "Çevrimdışı"].iter().enumerate() {
                if ui.selectable_label(m.src as usize == i, ti(n)).clicked() {
                    m.src = i as u8;
                }
            }
        });
        if refit {
            self.ext.map.init = false;
        }
        let m0 = &self.ext.map;
        let pts: Vec<Value> = self
            .ext
            .map_pts
            .iter()
            .filter(|p| m0.all || ji(p, "creator_id") == my_id)
            .filter(|p| [m0.show_sale, m0.show_rent, m0.show_land][kind_of(p)])
            .cloned()
            .collect();
        if pts.is_empty() {
            ui.label(RichText::new(if self.ext.map_pts.is_empty() {
                "Konumu girilmiş portföy yok. Portföy formunda enlem/boylam ekleyin."
            } else if !self.ext.map.all {
                "Konumu girilmiş kendi portföyünüz yok. \"Hepsi\" seçeneğini deneyin veya portföy formunda enlem/boylam ekleyin."
            } else {
                "Seçili türde konumlu portföy yok."
            }).color(muted()));
            return;
        }
        if !self.ext.map.init {
            let (mut a, mut b, mut c, mut d) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for p in &pts {
                a = a.min(jf(p, "lon"));
                b = b.max(jf(p, "lon"));
                c = c.min(jf(p, "lat"));
                d = d.max(jf(p, "lat"));
            }
            let m = &mut self.ext.map;
            m.cx = (a + b) / 2.0;
            m.cy = (c + d) / 2.0;
            // 700 px'lik alana sığacak yakınlaştırma
            let zx = (700.0 * 360.0 / ((b - a).max(0.0005) * 1.4 * 256.0)).log2();
            let zy = (500.0 * 170.0 / ((d - c).max(0.0005) * 1.6 * 256.0)).log2();
            m.z = zx.min(zy).clamp(4.0, 16.5);
            m.init = true;
        }
        ui.label(RichText::new(ti("Sürükleyerek kaydırın, fare tekerleğiyle yakınlaştırın. Noktaya tıklayın. Harita karoları internetten indirilir ve bilgisayarda saklanır (çevrimdışıyken daha önce görülen alanlar açılır).")).small().color(muted()));
        ui.horizontal_top(|ui| {
            let w = (ui.available_width() - 300.0).max(300.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(w, ui.available_height() - 6.0), Sense::click_and_drag());
            let m = &mut self.ext.map;
            // Web Mercator: dünya piksel boyutu = 256 * 2^z
            let world = |z: f64| 256.0 * 2f64.powf(z);
            let to_world = |lon: f64, lat: f64, z: f64| -> (f64, f64) {
                let (tx, ty) = tiles::lonlat_to_tile(lon, lat, 0);
                (tx * world(z), ty * world(z))
            };
            if resp.dragged() {
                let dl = resp.drag_delta();
                let (wx, wy) = to_world(m.cx, m.cy, m.z);
                let (lon, lat) = tiles::tile_to_lonlat((wx - dl.x as f64) / world(m.z), (wy - dl.y as f64) / world(m.z), 0.0);
                m.cx = lon;
                m.cy = lat.clamp(-84.0, 84.0);
            }
            if resp.hovered() {
                let sc = ui.input(|i| i.smooth_scroll_delta.y);
                if sc != 0.0 {
                    m.z = (m.z + sc as f64 * 0.004).clamp(3.0, 18.0);
                }
            }
            let to_screen = |lon: f64, lat: f64, m: &MapState| {
                let (cx, cy) = to_world(m.cx, m.cy, m.z);
                let (x, y) = to_world(lon, lat, m.z);
                egui::pos2(rect.center().x + (x - cx) as f32, rect.center().y + (y - cy) as f32)
            };
            let p = ui.painter_at(rect);
            p.rect_filled(rect, CornerRadius::same(8), thumb_bg());
            p.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0_f32, stroke()), egui::StrokeKind::Inside);
            if m.src < 2 {
                let src = if m.src == 0 { Source::Street } else { Source::Satellite };
                m.store.poll(ui.ctx());
                let zi = (m.z.round() as i32).clamp(3, 18) as u32;
                let k = 2f64.powf(m.z - zi as f64); // karo ölçeği (1 ≈ 256 px)
                let tsz = (256.0 * k) as f32;
                let (cwx, cwy) = to_world(m.cx, m.cy, m.z);
                let n = 1u32 << zi;
                let (x0, x1) = (((cwx - rect.width() as f64 / 2.0) / (256.0 * k)).floor() as i64, ((cwx + rect.width() as f64 / 2.0) / (256.0 * k)).floor() as i64);
                let (y0, y1) = (((cwy - rect.height() as f64 / 2.0) / (256.0 * k)).floor() as i64, ((cwy + rect.height() as f64 / 2.0) / (256.0 * k)).floor() as i64);
                let tp = ui.painter_at(rect);
                for ty in y0.max(0)..=y1.min(n as i64 - 1) {
                    for tx in x0..=x1 {
                        let txw = tx.rem_euclid(n as i64) as u32;
                        let min = egui::pos2(rect.center().x + (tx as f64 * 256.0 * k - cwx) as f32, rect.center().y + (ty as f64 * 256.0 * k - cwy) as f32);
                        let tr = egui::Rect::from_min_size(min, vec2(tsz + 0.5, tsz + 0.5));
                        if let Some(id) = m.store.get(ui.ctx(), src, zi, txw, ty as u32) {
                            tp.image(id, tr, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        } else {
                            tp.rect_filled(tr, 0.0, thumb_bg());
                        }
                    }
                }
                let att = src.attribution();
                let g = tp.layout_no_wrap(att.to_string(), FontId::proportional(10.5), egui::Color32::from_gray(40));
                let ar = egui::Rect::from_min_size(rect.right_bottom() - g.size() - vec2(10.0, 8.0), g.size() + vec2(8.0, 4.0));
                tp.rect_filled(ar, CornerRadius::same(3), egui::Color32::from_white_alpha(200));
                tp.galley(ar.min + vec2(4.0, 2.0), g, egui::Color32::from_gray(40));
                if m.store.net_errors > 4 {
                    tp.text(rect.left_top() + vec2(10.0, 10.0), egui::Align2::LEFT_TOP, "Harita karoları indirilemedi (internet yok?). Daha önce görülen alanlar önbellekten açılır.", FontId::proportional(12.0), egui::Color32::from_rgb(190, 55, 55));
                }
            }
            let mut hit: Option<i64> = None;
            let mut used: Vec<egui::Rect> = Vec::new();
            let mut sel_pt: Option<(egui::Pos2, String)> = None;
            for pt in &pts {
                let pos = to_screen(jf(pt, "lon"), jf(pt, "lat"), m);
                if !rect.contains(pos) {
                    continue;
                }
                let sel = ji(pt, "id") == m.sel;
                let col = kind_color(kind_of(pt));
                let r = if sel { 9.5 } else { 6.5 };
                p.circle_filled(pos, r, col);
                p.circle_stroke(pos, r, Stroke::new(if sel { 2.5_f32 } else { 1.5_f32 }, egui::Color32::WHITE));
                if sel {
                    sel_pt = Some((pos, js(pt, "title")));
                } else if m.labels {
                    let g = p.layout_no_wrap(short_label(pt), FontId::proportional(11.5), egui::Color32::WHITE);
                    let lr = egui::Rect::from_min_size(pos + vec2(10.0, -9.0), g.size() + vec2(8.0, 4.0));
                    if !used.iter().any(|u| u.intersects(lr)) {
                        p.rect_filled(lr, CornerRadius::same(4), col.gamma_multiply(0.92));
                        p.galley(lr.min + vec2(4.0, 2.0), g, egui::Color32::WHITE);
                        used.push(lr);
                    }
                }
                if resp.clicked() {
                    if let Some(c) = resp.interact_pointer_pos() {
                        if c.distance(pos) < 12.0 {
                            hit = Some(ji(pt, "id"));
                        }
                    }
                }
            }
            if let Some((pos, title)) = sel_pt {
                let g = p.layout_no_wrap(title, FontId::proportional(13.0), t_strong());
                let lr = egui::Rect::from_min_size(pos + vec2(12.0, -24.0), g.size() + vec2(12.0, 6.0));
                p.rect_filled(lr, CornerRadius::same(5), card());
                p.rect_stroke(lr, CornerRadius::same(5), Stroke::new(1.0_f32, stroke()), egui::StrokeKind::Inside);
                p.galley(lr.min + vec2(6.0, 3.0), g, t_strong());
            }
            if let Some(h) = hit {
                m.sel = h;
            }
            ui.vertical(|ui| {
                let sel = pts.iter().find(|x| ji(x, "id") == self.ext.map.sel).cloned();
                panel_box(ui, ti("Seçili portföy"), "", |ui| match &sel {
                    None => {
                        ui.label(RichText::new(ti("Haritadan bir nokta seçin.")).color(muted()));
                    }
                    Some(s) => {
                        ui.label(RichText::new(js(s, "title")).strong().size(15.0));
                        ui.label(format!("{} {} • {} / {}", js(s, "listing_type"), js(s, "category"), js(s, "district"), js(s, "neighborhood")));
                        ui.label(crate::common::fmt_price(jf(s, "price"), &js(s, "currency")));
                        ui.horizontal(|ui| {
                            if ui.button(ti("Portföyü aç")).clicked() {
                                let id = ji(s, "id");
                                self.open_detail(id);
                            }
                            if ui.button(ti("OpenStreetMap")).clicked() {
                                let u = format!("https://www.openstreetmap.org/?mlat={0}&mlon={1}#map=17/{0}/{1}", jf(s, "lat"), jf(s, "lon"));
                                self.open_url(&u);
                            }
                        });
                    }
                });
                ui.label(RichText::new(format!("{} portföy gösteriliyor", pts.len())).small().color(muted()));
            });
        });
    }
}
