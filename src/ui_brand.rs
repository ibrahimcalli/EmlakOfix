//! Marka görünümü: tam ekran arka plan, logo, giriş ekranı bilgi paneli, arka plan ayarı.
use crate::app::*;
use crate::common::APP_NAME;
use crate::i18n_ui::ti;
use crate::theme::*;
use eframe::egui::{self, vec2, Align2, Color32, FontId, RichText, Sense, TextureHandle, TextureOptions};

pub const SITE: &str = "https://emlakfethiye.com.tr";
pub const PHONE: &str = "0090 532 402 1434";
pub const MAIL: &str = "ibrahimcallifethiye@gmail.com";
pub const APP_NAME_MOBILE: &str = "Gayrimenkul Portföyleri";
pub const PLAY_URL: &str = "https://play.google.com/store/search?q=Gayrimenkul%20Portf%C3%B6yleri&c=apps";

impl App {
    pub fn logo_handle(&mut self) -> Option<TextureHandle> {
        if self.logo_tex.is_none() {
            if let Ok(img) = decode_image(crate::bgimg::LOGO_GOLD, 512) {
                self.logo_tex = Some(self.ctx.load_texture("logo_altin", img, TextureOptions::LINEAR));
            }
        }
        self.logo_tex.clone()
    }

    /// Verilen alanı arka plan resmiyle (kırparak, oranı bozmadan) veya düz zeminle doldurur.
    pub fn draw_backdrop(&mut self, ui: &egui::Ui) {
        let rect = ui.max_rect();
        let key = format!("{}#{}", self.cfg.look.bg, self.bg_ver);
        if self.bg_key != key {
            self.bg_key = key;
            self.bg_tex = crate::bgimg::bytes_for(&self.cfg.look.bg).and_then(|b| decode_image(&b, 2200).ok()).map(|img| self.ctx.load_texture("login_bg", img, TextureOptions::LINEAR));
        }
        let p = ui.painter();
        match &self.bg_tex {
            Some(t) => {
                let ts = t.size_vec2();
                let k = (rect.width() / ts.x).max(rect.height() / ts.y);
                let vis = vec2(rect.width() / k / ts.x, rect.height() / k / ts.y);
                let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), vis);
                p.image(t.id(), rect, uv, Color32::WHITE);
                // soldan sağa koyulaşan örtü: yazılar okunaklı kalsın
                let mut mesh = egui::Mesh::default();
                let (a, b) = (Color32::from_rgba_unmultiplied(6, 12, 24, 225), Color32::from_rgba_unmultiplied(6, 12, 24, 70));
                let i = mesh.vertices.len() as u32;
                for (pos, c) in [(rect.left_top(), a), (rect.right_top(), b), (rect.right_bottom(), b), (rect.left_bottom(), a)] {
                    mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color: c });
                }
                mesh.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
                p.add(egui::Shape::mesh(mesh));
            }
            None => {
                p.rect_filled(rect, 0.0, Color32::from_rgb(8, 14, 28));
            }
        }
    }

    /// Sol bilgi paneli: logo, kısa açıklama, iletişim, mobil uygulama.
    pub fn brand_panel(&mut self, ui: &mut egui::Ui) {
        let white = Color32::from_rgb(240, 244, 252);
        let soft = Color32::from_rgb(190, 202, 224);
        let gold = Color32::from_rgb(212, 175, 55);
        if let Some(t) = self.logo_handle() {
            let h = 96.0;
            let w = h * t.size_vec2().x / t.size_vec2().y;
            ui.image((t.id(), vec2(w, h)));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(self.settings.office_name.clone()).size(34.0).strong().color(white));
        ui.label(RichText::new(APP_NAME.to_uppercase()).size(12.5).strong().color(gold));
        ui.add_space(14.0);
        ui.label(RichText::new(ti("Portföy, müşteri ilişkileri, mesai ve görev yönetimi — ofisinizin tüm işleri tek, güvenli ve hızlı bir çatı altında.")).size(16.0).color(soft));
        ui.add_space(22.0);

        let row = |ui: &mut egui::Ui, icon: &str, text: &str, url: Option<String>| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(16.0).color(gold));
                match url {
                    Some(u) => {
                        ui.hyperlink_to(RichText::new(text).size(15.0).color(white), u);
                    }
                    None => {
                        ui.label(RichText::new(text).size(15.0).color(white));
                    }
                }
            });
        };
        row(ui, "🌐", SITE.trim_start_matches("https://"), Some(SITE.into()));
        row(ui, "☎", PHONE, None);
        row(ui, "✉", MAIL, Some(format!("mailto:{MAIL}")));
        ui.add_space(18.0);

        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(8, 14, 28, 175))
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(212, 175, 55, 120)))
            .corner_radius(10)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_max_width(400.0);
                ui.label(RichText::new(format!("📱  {}", ti("Mobil uygulamamız"))).strong().color(gold));
                ui.label(RichText::new(APP_NAME_MOBILE).size(18.0).strong().color(white));
                ui.label(RichText::new(ti("Portföylerinize cebinizden de ulaşın. Android için Google Play'de.")).size(13.5).color(soft));
                ui.hyperlink_to(RichText::new(ti("Google Play'de görüntüle")).size(14.0).color(gold), PLAY_URL);
            });
    }

    /// Ayarlar > Görünüm: giriş ekranı arka planı.
    pub fn bg_section(&mut self, ui: &mut egui::Ui, look: &mut Look) {
        crate::ui_settings::section(ui, ti("Giriş ekranı arka planı"), ti("Açılış ekranında tam ekran görünür. Kendi resminizi seçebilirsiniz."), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (k, name) in [("gunbatimi", "Gün batımı (varsayılan)"), ("gol", "Göl evi"), ("ozel", "Kendi resmim"), ("yok", "Düz zemin")] {
                    if ui.selectable_label(look.bg == k, ti(name)).clicked() {
                        look.bg = k.to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button(ti("Resim seç…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().set_title("Arka plan resmi").add_filter("Resim", &["jpg", "jpeg", "png", "webp"]).pick_file() {
                        match crate::bgimg::import(&p) {
                            Ok(m) => {
                                self.bg_msg = m;
                                self.bg_ver += 1;
                                look.bg = "ozel".into();
                            }
                            Err(e) => self.bg_msg = e,
                        }
                    }
                }
                ui.label(RichText::new(format!("{} {}×{}", ti("En az"), crate::bgimg::MIN_W, crate::bgimg::MIN_H)).small().color(muted()));
            });
            ui.label(RichText::new(ti("Büyük resimler otomatik küçültülür (en uzun kenar 2560 px); çok küçük resimler kabul edilmez.")).small().color(muted()));
            if !self.bg_msg.is_empty() {
                ui.label(RichText::new(self.bg_msg.clone()).color(accent()));
            }
        });
    }
}

#[allow(dead_code)]
fn _unused(_: Sense, _: Align2, _: FontId) {}
