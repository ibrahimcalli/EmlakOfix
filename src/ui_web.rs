//! Web sitesi arayüzü: etiketler, web yayın kartı, Ayarlar > Web sitesi, Blog yayınla.
use crate::app::*;
use crate::common::*;
use crate::i18n_ui::ti;
use crate::theme::*;
use crate::ui_ext::{panel_box, primary};
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Default)]
pub struct WebUi {
    pub tags: Vec<Value>,
    pub tags_loaded: bool,
    pub new_tag: String,
    pub url: String,
    pub token: String,
    pub info: Value,
    pub loaded: bool,
    pub msg: String,
    // blog
    pub b_title: String,
    pub b_cat: String,
    pub b_excerpt: String,
    pub b_body: String,
    pub b_draft: bool,
    pub b_img: Option<PathBuf>,
    pub b_busy: bool,
    pub b_msg: String,
}

pub fn hex_color(s: &str) -> Color32 {
    let t = s.trim_start_matches('#');
    if t.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&t[0..2], 16), u8::from_str_radix(&t[2..4], 16), u8::from_str_radix(&t[4..6], 16)) {
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::from_rgb(212, 175, 55)
}
fn text_on(c: Color32) -> Color32 {
    if 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32 > 150.0 { Color32::from_rgb(25, 25, 30) } else { Color32::WHITE }
}

impl App {
    pub fn web_load_tags(&mut self) {
        self.call(|api| api.get::<Vec<Value>>("/api/tags"), |app, v| {
            app.web.tags = v;
            app.web.tags_loaded = true;
        });
    }

    /// Portföy formunda etiket seçimi (Müstakil, Villa, Fırsat, Yeni…)
    pub fn form_tags_ui(&mut self, ui: &mut Ui) {
        ui.label(ti("Etiketler (ilan kartında görünür)"));
        let all = self.web.tags.clone();
        ui.horizontal_wrapped(|ui| {
            for t in &all {
                let name = t["name"].as_str().unwrap_or("").to_string();
                let col = hex_color(t["color"].as_str().unwrap_or("#d4af37"));
                let on = self.form.p.tags.iter().any(|x| x.eq_ignore_ascii_case(&name));
                let txt = RichText::new(if on { format!("✓ {name}") } else { name.clone() }).color(if on { text_on(col) } else { t_strong() });
                let b = egui::Button::new(txt).fill(if on { col } else { Color32::TRANSPARENT }).stroke(egui::Stroke::new(1.0_f32, col)).corner_radius(10);
                if ui.add(b).clicked() {
                    if on {
                        self.form.p.tags.retain(|x| !x.eq_ignore_ascii_case(&name));
                    } else {
                        self.form.p.tags.push(name);
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.web.new_tag).hint_text(ti("Yeni etiket yaz…")).desired_width(180.0));
            if ui.button(ti("Etiket ekle")).clicked() {
                let n = self.web.new_tag.trim().to_string();
                if !n.is_empty() && !self.form.p.tags.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
                    self.form.p.tags.push(n);
                }
                self.web.new_tag.clear();
            }
        });
        ui.add_space(4.0);
    }

    /// Portföy detayında: web sitesi yayın durumu ve onay düğmeleri
    pub fn web_card_ui(&mut self, ui: &mut Ui, p: &Portfolio) {
        let lead = self.me.is_lead();
        let mine = p.creator_id == self.me.id;
        if !lead && !mine {
            return;
        }
        let id = p.id;
        let mut act: Option<&'static str> = None;
        panel_box(ui, ti("Web sitesi"), ti("Yayın isteğini portföy sahibi gönderir; yönetici onaylayınca ilan siteye gider ve 'Web yayını' hedefine sayılır."), |ui| {
            ui.horizontal_wrapped(|ui| {
                for t in &p.tags {
                    let col = self.web.tags.iter().find(|x| x["name"].as_str().map(|n| n.eq_ignore_ascii_case(t)).unwrap_or(false)).map(|x| hex_color(x["color"].as_str().unwrap_or(""))).unwrap_or(Color32::from_rgb(212, 175, 55));
                    badge(ui, t, col);
                }
            });
            match p.web_status.as_str() {
                "Yayında" => {
                    ui.label(RichText::new(format!("{} {}", ti("Web sitesinde yayında —"), p.web_at)).color(Color32::from_rgb(37, 140, 80)).strong());
                    if lead && ui.button(ti("Web yayınından kaldır")).clicked() {
                        act = Some("unpublish");
                    }
                }
                "Bekliyor" => {
                    ui.label(RichText::new(ti("Yönetici onayı bekliyor")).color(Color32::from_rgb(200, 130, 20)).strong());
                    if lead {
                        ui.horizontal(|ui| {
                            if primary(ui, ti("Onayla ve web'de yayınla")) {
                                act = Some("approve");
                            }
                            if ui.button(ti("Reddet")).clicked() {
                                act = Some("reject");
                            }
                        });
                    }
                }
                _ => {
                    ui.label(RichText::new(ti("Web sitesinde yayında değil")).color(muted()));
                    if p.status == "Taslak" {
                        ui.label(RichText::new(ti("Önce portföy durumunu 'Yayında' yapın.")).small().color(muted()));
                    } else if lead {
                        if primary(ui, ti("Web'de yayınla (onayla)")) {
                            act = Some("approve");
                        }
                    } else if primary(ui, ti("Web'de yayınlama isteği gönder")) {
                        act = Some("request");
                    }
                }
            }
        });
        if let Some(a) = act {
            self.call(move |api| api.post(&format!("/api/portfolios/{id}/web"), &json!({"action": a})), move |app, _| {
                app.toast(ti("Web yayın durumu güncellendi."));
                app.open_detail(id);
            });
        }
    }

    fn web_reload(&mut self) {
        self.call(|api| api.get::<Value>("/api/web/settings"), |app, v| {
            app.web.url = v["url"].as_str().unwrap_or("").to_string();
            app.web.info = v;
            app.web.loaded = true;
        });
    }

    /// Ayarlar > Web sitesi (yönetici)
    pub fn web_settings_ui(&mut self, ui: &mut Ui) {
        if !self.me.is_admin() {
            return;
        }
        if !self.web.loaded {
            self.web.loaded = true;
            self.web_reload();
            self.web_load_tags();
        }
        panel_box(ui, ti("Web sitesi (sahibindenfethiye.com.tr)"), ti("Cloudflare üzerinde yayınlanan siteye ilan, fotoğraf, etiket ve blog yazısı gönderir. Adres ve anahtar sitenin kurulum rehberinde yazar."), |ui| {
            ui.horizontal(|ui| {
                ui.label(ti("Site adresi:"));
                ui.add(egui::TextEdit::singleline(&mut self.web.url).hint_text("https://sahibindenfethiye.com.tr").desired_width(300.0));
            });
            ui.horizontal(|ui| {
                ui.label(ti("Site anahtarı:"));
                ui.add(egui::TextEdit::singleline(&mut self.web.token).password(true).hint_text(if self.web.info["token_set"].as_bool().unwrap_or(false) { ti("(kayıtlı — değiştirmek için yazın)") } else { "" }).desired_width(300.0));
            });
            ui.horizontal_wrapped(|ui| {
                if primary(ui, ti("Kaydet")) {
                    let body = json!({"url": self.web.url.trim(), "token": self.web.token.trim()});
                    self.call(move |api| api.put("/api/web/settings", &body), |app, _| {
                        app.web.token.clear();
                        app.toast(ti("Web sitesi ayarları kaydedildi."));
                        app.web_reload();
                    });
                }
                if ui.button(ti("Bağlantıyı dene")).clicked() {
                    self.call(|api| api.post("/api/web/ping", &json!({})), |app, v| {
                        app.web.msg = format!("Bağlantı tamam — sitede {} ilan, {} yazı.", v["listings"], v["posts"]);
                    });
                }
                if ui.button(ti("Web yönetim paneline gir")).clicked() {
                    self.call(|api| api.post("/api/web/sso", &json!({})), |app, v| {
                        if let Some(u) = v["url"].as_str() {
                            app.open_url(u);
                        }
                    });
                }
                if ui.button(ti("Tüm yayındaki ilanları yeniden gönder")).clicked() {
                    self.call(|api| api.post("/api/web/resync", &json!({})), |app, v| {
                        app.toast(format!("{} ilan siteye gönderilmek üzere sıraya alındı.", v["queued"]));
                    });
                }
            });
            if !self.web.msg.is_empty() {
                ui.label(RichText::new(&self.web.msg).color(Color32::from_rgb(37, 140, 80)));
            }
            let i = &self.web.info;
            ui.label(RichText::new(format!("{} {} • {} {}", ti("Web'de yayında:"), i["live"], ti("onay bekleyen:"), i["pending"])).small().color(muted()));
            if let Some(l) = i["last"].as_str().filter(|s| !s.is_empty()) {
                ui.label(RichText::new(format!("{} {l}", ti("Son işlem:"))).small().color(muted()));
            }
            if ui.small_button(ti("Durumu yenile")).clicked() {
                self.web_reload();
            }
        });
        panel_box(ui, ti("Etiketler"), ti("Portföy kartlarında görünen etiketler. Hem programda hem sitede aynıdır; ekleyip silebilir, renk (#RRGGBB) ve İngilizce/Rusça karşılık yazabilirsiniz."), |ui| {
            let mut del: Option<usize> = None;
            egui::Grid::new("tag_grid").num_columns(5).spacing([8.0, 4.0]).show(ui, |ui| {
                for (i, t) in self.web.tags.iter_mut().enumerate() {
                    for (k, w) in [("name", 150.0), ("name_en", 120.0), ("name_ru", 120.0), ("color", 80.0)] {
                        let mut s = t[k].as_str().unwrap_or("").to_string();
                        if ui.add(egui::TextEdit::singleline(&mut s).desired_width(w)).changed() {
                            t[k] = Value::String(s);
                        }
                    }
                    if ui.small_button("✕").clicked() {
                        del = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = del {
                self.web.tags.remove(i);
            }
            ui.horizontal(|ui| {
                if ui.button(ti("+ Etiket")).clicked() {
                    self.web.tags.push(json!({"name": "Yeni etiket", "name_en": "", "name_ru": "", "color": "#d4af37"}));
                }
                if primary(ui, ti("Etiketleri kaydet")) {
                    let body = Value::Array(self.web.tags.clone());
                    self.call(move |api| api.put("/api/tags", &body), |app, _| app.toast(ti("Etiketler kaydedildi.")));
                }
            });
        });
    }

    /// Blog yayınla sekmesi
    pub fn ui_blog(&mut self, ui: &mut Ui) {
        page_title(ui, "Blog yayınla");
        ui.label(RichText::new(ti("Yazıyı doğrudan web sitesinde (sahibindenfethiye.com.tr) yayınlar. Gelişmiş düzenleme için web yönetim paneli kullanılabilir.")).small().color(muted()));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            panel_box(ui, ti("Yeni yazı"), "", |ui| {
                ui.label(ti("Başlık"));
                ui.add(egui::TextEdit::singleline(&mut self.web.b_title).desired_width(f32::INFINITY));
                ui.label(ti("Kategori"));
                ui.add(egui::TextEdit::singleline(&mut self.web.b_cat).hint_text(ti("Aşağıdan seçin veya yeni kategori yazın")).desired_width(360.0));
                ui.horizontal_wrapped(|ui| {
                    for c in ["ARAZİ ÇEŞİTLERİ VE İMAR DURUMLARI", "GAYRİMENKUL SATIN ALMA SÜREÇLERİ", "HİSSELİ ARAZİLERLE İLGİLİ YAZILAR", "SEKTÖRE DAİR ÇIKAN HABERLER", "PROGRAM HABERLERİ"] {
                        if ui.small_button(c).clicked() {
                            self.web.b_cat = c.to_string();
                        }
                    }
                });
                ui.label(ti("Kısa özet (liste ve arama sonuçlarında görünür)"));
                ui.add(egui::TextEdit::multiline(&mut self.web.b_excerpt).desired_rows(2).desired_width(f32::INFINITY));
                ui.label(ti("Yazı metni (boş satır = yeni paragraf; '## ' başlık; '- ' madde)"));
                ui.add(egui::TextEdit::multiline(&mut self.web.b_body).desired_rows(16).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    if ui.button(ti("Görsel seç…")).clicked() {
                        self.pick_files("Yazı görseli", &["jpg", "jpeg", "png", "webp"], false, |app, f| app.web.b_img = f.into_iter().next());
                    }
                    match &self.web.b_img {
                        Some(p) => {
                            ui.label(p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                            if ui.small_button("✕").clicked() {
                                self.web.b_img = None;
                            }
                        }
                        None => {
                            ui.label(RichText::new(ti("Görsel seçilmedi")).color(muted()));
                        }
                    }
                });
                ui.checkbox(&mut self.web.b_draft, ti("Taslak olarak kaydet (yayınlama)"));
                if !self.web.b_msg.is_empty() {
                    ui.label(RichText::new(&self.web.b_msg).color(Color32::from_rgb(37, 140, 80)));
                }
                ui.add_enabled_ui(!self.web.b_busy, |ui| {
                    if primary(ui, if self.web.b_draft { ti("Taslağı gönder") } else { ti("Yayınla") }) {
                        self.send_blog();
                    }
                });
            });
        });
    }

    fn send_blog(&mut self) {
        if self.web.b_title.trim().is_empty() || self.web.b_body.trim().is_empty() {
            return self.err(ti("Başlık ve yazı metni boş olamaz."));
        }
        self.web.b_busy = true;
        let (title, cat, ex, body, draft, img) = (self.web.b_title.clone(), self.web.b_cat.clone(), self.web.b_excerpt.clone(), self.web.b_body.clone(), self.web.b_draft, self.web.b_img.clone());
        self.call(
            move |api| {
                use base64::Engine;
                let mut b64 = String::new();
                if let Some(p) = img {
                    let im = image::open(&p).map_err(|e| format!("Görsel açılamadı: {e}"))?;
                    let im = if im.width() > 1600 || im.height() > 1600 { im.resize(1600, 1600, image::imageops::FilterType::Triangle) } else { im };
                    let mut out = Vec::new();
                    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
                    image::DynamicImage::ImageRgb8(im.to_rgb8()).write_with_encoder(enc).map_err(|e| e.to_string())?;
                    b64 = base64::engine::general_purpose::STANDARD.encode(out);
                }
                api.post("/api/web/post", &json!({"title": title, "category": cat, "excerpt": ex, "body": body, "status": if draft { "taslak" } else { "yayinda" }, "image_b64": b64}))
            },
            |app, v| {
                app.web.b_busy = false;
                app.web.b_msg = format!("Gönderildi: {}", v["url"].as_str().unwrap_or("yazı sitede"));
                app.web.b_title.clear();
                app.web.b_excerpt.clear();
                app.web.b_body.clear();
                app.web.b_img = None;
                app.toast(ti("Yazı web sitesine gönderildi."));
            },
        );
    }
}
