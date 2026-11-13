//! Portföy ekleme / düzenleme formu.
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::common::*;
use crate::ui_portfolio::{combo_pick, cover_uv, strs};
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use std::path::PathBuf;

const IMG_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "JPG", "JPEG", "PNG"];

fn num_opt(s: &str, what: &str) -> Result<Option<f64>, String> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what} sayı olmalı (örn: 36.624512)."))
}

impl App {
    pub fn start_new_portfolio(&mut self) {
        self.form = FormState { p: Portfolio::new_blank(), is_new: true, ..Default::default() };
        self.view = View::Form;
        self.load_locations();
    }

    pub fn start_edit_portfolio(&mut self, p: &Portfolio) {
        let known = self
            .locations
            .iter()
            .find(|d| d.name == p.district)
            .map(|d| d.neighborhoods.iter().any(|n| n.name == p.neighborhood))
            .unwrap_or(false);
        let manual = !p.neighborhood.is_empty() && !known;
        self.form = FormState {
            p: p.clone(),
            is_new: false,
            price_s: num_to_text(p.price),
            gross_s: num_to_text(p.gross_m2),
            net_s: num_to_text(p.net_m2),
            lat_s: p.lat.map(|v| format!("{:.6}", v)).unwrap_or_default(),
            lon_s: p.lon.map(|v| format!("{:.6}", v)).unwrap_or_default(),
            manual_neigh: manual,
            ..Default::default()
        };
        self.view = View::Form;
        self.load_locations();
    }

    fn cancel_form(&mut self) {
        let was_new = self.form.is_new;
        self.form = FormState::default();
        if was_new || self.detail.is_none() {
            self.goto(View::Home);
        } else {
            self.view = View::Detail;
        }
    }

    fn save_form(&mut self) {
        let f = &self.form;
        let mut p = f.p.clone();
        if p.title.trim().is_empty() {
            return self.err(ti("Portföy başlığı boş olamaz."));
        }
        if p.district.trim().is_empty() {
            return self.err(ti("İlçe seçin."));
        }
        p.price = parse_num(&f.price_s, true);
        p.gross_m2 = parse_num(&f.gross_s, false);
        p.net_m2 = parse_num(&f.net_s, false);
        match (num_opt(&f.lat_s, "Enlem"), num_opt(&f.lon_s, "Boylam")) {
            (Ok(la), Ok(lo)) => {
                if la.is_some() != lo.is_some() {
                    return self.err(ti("Enlem ve boylamın ikisini birden girin (ya da ikisini de boş bırakın)."));
                }
                if let (Some(a), Some(b)) = (la, lo) {
                    if a.abs() > 90.0 || b.abs() > 180.0 {
                        return self.err(ti("Koordinat değerleri geçersiz."));
                    }
                }
                p.lat = la;
                p.lon = lo;
            }
            (Err(e), _) | (_, Err(e)) => return self.err(e),
        }
        let pending: Vec<PathBuf> = f.pending.clone();
        let remove = f.remove.clone();
        let cover_existing = f.cover_existing;
        let cover_pending = f.cover_pending;
        let is_new = f.is_new;
        self.form.saving = true;
        self.bg(
            move |api| -> Result<(i64, Vec<String>), String> {
                let id = if is_new {
                    let v = api.post("/api/portfolios", &p)?;
                    v["id"].as_i64().ok_or("Sunucu kayıt numarası döndürmedi.")?
                } else {
                    api.put(&format!("/api/portfolios/{}", p.id), &p)?;
                    p.id
                };
                let mut warns = vec![];
                for pid in remove {
                    if let Err(e) = api.delete(&format!("/api/photos/{pid}")) {
                        warns.push(e);
                    }
                }
                let mut new_ids: Vec<Option<i64>> = vec![];
                for path in &pending {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| api.send_bytes("POST", &format!("/api/portfolios/{id}/photos"), &b)) {
                        Ok(v) => new_ids.push(v["id"].as_i64()),
                        Err(e) => {
                            new_ids.push(None);
                            warns.push(format!("{name}: {e}"));
                        }
                    }
                }
                let cover = cover_existing.or_else(|| cover_pending.and_then(|i| new_ids.get(i).cloned().flatten()));
                if let Some(c) = cover {
                    if let Err(e) = api.put(&format!("/api/photos/{c}/cover"), &serde_json::json!({})) {
                        warns.push(format!("Kapak fotoğrafı ayarlanamadı: {e}"));
                    }
                }
                Ok((id, warns))
            },
            |app, r| {
                app.form.saving = false;
                match r {
                    Ok((id, warns)) => {
                        app.form = FormState::default();
                        app.toast(ti("Portföy kaydedildi."));
                        for w in warns {
                            app.err(w);
                        }
                        app.open_detail(id);
                    }
                    Err(e) => app.err(e),
                }
            },
        );
    }

    fn photo_tile(&mut self, ui: &mut Ui, key: TexKey, is_cover: bool, removed: bool) -> egui::Rect {
        let (r, _) = ui.allocate_exact_size(vec2(130.0, 96.0), Sense::hover());
        if ui.is_rect_visible(r) {
            let painter = ui.painter_at(r);
            painter.rect_filled(r, 0.0, thumb_bg());
            if let Some(t) = self.tex(key) {
                painter.image(t.id(), r, cover_uv(t.size_vec2(), r), if removed { Color32::from_white_alpha(70) } else { Color32::WHITE });
            }
            if is_cover && !removed {
                let rr = Rect::from_min_size(r.min + vec2(4.0, 4.0), vec2(52.0, 18.0));
                painter.rect_filled(rr, CornerRadius::same(4), accent());
                painter.text(rr.center(), egui::Align2::CENTER_CENTER, "KAPAK", egui::FontId::proportional(11.5), Color32::WHITE);
                painter.rect_stroke(r, 0.0, Stroke::new(3.0_f32, accent()), StrokeKind::Inside);
            }
            if removed {
                painter.text(r.center(), egui::Align2::CENTER_CENTER, "silinecek", egui::FontId::proportional(14.0), Color32::from_rgb(190, 40, 40));
            }
        }
        r
    }

    fn form_photos(&mut self, ui: &mut Ui) {
        let existing: Vec<i64> = self.form.p.photos.clone();
        let pending: Vec<PathBuf> = self.form.pending.clone();
        let removed = self.form.remove.clone();
        // Kapak olacak fotoğraf
        let first_existing = existing.iter().find(|i| !removed.contains(i)).cloned();
        let cover_ex = self.form.cover_existing.or(if self.form.cover_pending.is_none() { first_existing } else { None });
        let cover_pe = self.form.cover_pending.or(if cover_ex.is_none() && !pending.is_empty() { Some(0) } else { None });

        let mut set_cover_ex: Option<i64> = None;
        let mut set_cover_pe: Option<usize> = None;
        let mut toggle_remove: Option<i64> = None;
        let mut drop_pending: Option<usize> = None;
        let mut move_pending: Option<(usize, i64)> = None;

        // Sürükle-bırak ile fotoğraf ekleme
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for f in dropped {
            let ok = f.extension().map(|e| IMG_EXTS.contains(&e.to_string_lossy().to_lowercase().as_str())).unwrap_or(false);
            if ok && !self.form.pending.contains(&f) {
                self.form.pending.push(f);
            }
        }
        let mut reorder: Option<(usize, i64)> = None;
        let mut rotate: Option<(i64, i32)> = None;
        ui.horizontal_wrapped(|ui| {
            for (pos, id) in existing.iter().enumerate() {
                let rem = removed.contains(id);
                ui.vertical(|ui| {
                    self.photo_tile(ui, TexKey::Thumb(*id), cover_ex == Some(*id), rem);
                    if !rem && !self.form.is_new {
                        ui.horizontal(|ui| {
                            if ui.small_button(ti("◀")).on_hover_text(ti("Sola al")).clicked() {
                                reorder = Some((pos, -1));
                            }
                            if ui.small_button(ti("▶")).on_hover_text(ti("Sağa al")).clicked() {
                                reorder = Some((pos, 1));
                            }
                            if ui.small_button(ti("⟲")).on_hover_text(ti("Sola döndür")).clicked() {
                                rotate = Some((*id, 270));
                            }
                            if ui.small_button(ti("⟳")).on_hover_text(ti("Sağa döndür")).clicked() {
                                rotate = Some((*id, 90));
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if !rem && ui.small_button(ti("Kapak yap")).clicked() {
                            set_cover_ex = Some(*id);
                        }
                        if ui.small_button(if rem { "Geri al" } else { "Sil" }).clicked() {
                            toggle_remove = Some(*id);
                        }
                    });
                });
            }
            for (i, path) in pending.iter().enumerate() {
                ui.vertical(|ui| {
                    self.photo_tile(ui, TexKey::Local(path.to_string_lossy().to_string()), cover_pe == Some(i), false);
                    ui.horizontal(|ui| {
                        if i > 0 && ui.small_button(ti("◀")).on_hover_text(ti("Sola al")).clicked() {
                            move_pending = Some((i, -1));
                        }
                        if i + 1 < pending.len() && ui.small_button(ti("▶")).on_hover_text(ti("Sağa al")).clicked() {
                            move_pending = Some((i, 1));
                        }
                        if ui.small_button(ti("Kapak yap")).clicked() {
                            set_cover_pe = Some(i);
                        }
                        if ui.small_button(ti("Çıkar")).clicked() {
                            drop_pending = Some(i);
                        }
                    });
                });
            }
        });

        if let Some((pos, dir)) = reorder {
            let np = pos as i64 + dir;
            if np >= 0 && (np as usize) < self.form.p.photos.len() {
                self.form.p.photos.swap(pos, np as usize);
                let ids = self.form.p.photos.clone();
                let pid = self.form.p.id;
                self.call(move |api| api.put(&format!("/api/portfolios/{pid}/photos/order"), &serde_json::json!({"ids": ids})), |_, _| {});
            }
        }
        if let Some((id, deg)) = rotate {
            self.call(move |api| api.post(&format!("/api/photos/{id}/rotate?deg={deg}"), &serde_json::json!({})), move |app, _| {
                app.tex.remove(&TexKey::Thumb(id));
                app.tex.remove(&TexKey::Full(id));
                app.toast(ti("Fotoğraf döndürüldü."));
            });
        }
        if let Some(id) = set_cover_ex {
            self.form.cover_existing = Some(id);
            self.form.cover_pending = None;
        }
        if let Some(i) = set_cover_pe {
            self.form.cover_pending = Some(i);
            self.form.cover_existing = None;
        }
        if let Some(id) = toggle_remove {
            if let Some(pos) = self.form.remove.iter().position(|x| *x == id) {
                self.form.remove.remove(pos);
            } else {
                self.form.remove.push(id);
                if self.form.cover_existing == Some(id) {
                    self.form.cover_existing = None;
                }
            }
        }
        if let Some((i, dir)) = move_pending {
            let j = (i as i64 + dir) as usize;
            if j < self.form.pending.len() {
                self.form.pending.swap(i, j);
                self.form.cover_pending = match self.form.cover_pending {
                    Some(c) if c == i => Some(j),
                    Some(c) if c == j => Some(i),
                    other => other,
                };
            }
        }
        if let Some(i) = drop_pending {
            self.form.pending.remove(i);
            self.form.cover_pending = match self.form.cover_pending {
                Some(c) if c == i => None,
                Some(c) if c > i => Some(c - 1),
                other => other,
            };
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(ti("Fotoğraf ekle…")).clicked() {
                self.pick_files("Fotoğrafları seçin", IMG_EXTS, true, |app, files| {
                    for f in files {
                        if !app.form.pending.contains(&f) {
                            app.form.pending.push(f);
                        }
                    }
                });
            }
            ui.label(RichText::new(ti("Birden fazla seçebilir veya dosyaları pencereye sürükleyip bırakabilirsiniz. Kapak fotoğrafı ana ekranda ve çıktıda ilk görünen resimdir.")).small().color(muted()));
        });
    }

    pub fn ui_form(&mut self, ui: &mut Ui) {
        let districts: Vec<String> = self.locations.iter().map(|d| d.name.clone()).collect();
        let neighs: Vec<String> = self
            .locations
            .iter()
            .find(|d| d.name == self.form.p.district)
            .map(|d| d.neighborhoods.iter().map(|n| n.name.clone()).collect())
            .unwrap_or_default();
        let is_new = self.form.is_new;
        let saving = self.form.saving;

        ui.horizontal(|ui| {
            page_title(ui, if is_new { "Yeni portföy" } else { "Portföyü düzenle" });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(!saving, egui::Button::new(RichText::new(ti("Kaydet")).strong().color(on_btn())).fill(btn()).min_size(vec2(110.0, 30.0))).clicked() {
                    self.save_form();
                }
                if ui.add_enabled(!saving, egui::Button::new("Vazgeç").min_size(vec2(80.0, 30.0))).clicked() {
                    self.cancel_form();
                }
                if saving {
                    ui.spinner();
                    ui.label(ti("Kaydediliyor (fotoğraflar yükleniyor)…"));
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(980.0);
            // ---- Temel bilgiler
            card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("1. Temel bilgiler")).size(17.0).strong());
                egui::Grid::new("f_basic").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Başlık *"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.p.title).hint_text(ti("Örn: Çalış'ta deniz manzaralı 3+1 daire")).desired_width(520.0));
                    ui.end_row();
                    ui.label(ti("İlan türü"));
                    ui.horizontal(|ui| {
                        for t in LISTING_TYPES {
                            ui.radio_value(&mut self.form.p.listing_type, t.to_string(), t);
                        }
                    });
                    ui.end_row();
                    ui.label(ti("Kategori"));
                    combo_pick(ui, "fm_cat", &mut self.form.p.category, &strs(&CATEGORIES), 230.0);
                    ui.end_row();
                    ui.label(ti("Durum"));
                    ui.horizontal(|ui| {
                        combo_pick(ui, "fm_status", &mut self.form.p.status, &strs(&STATUSES), 140.0);
                        ui.label(RichText::new(ti("Taslak: hazırlanıyor • Yayında: aktif ilan")).small().color(muted()));
                    });
                    ui.end_row();
                    ui.label(ti("Fiyat"));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.form.price_s).hint_text(ti("1.250.000")).desired_width(160.0));
                        combo_pick(ui, "fm_cur", &mut self.form.p.currency, &strs(&CURRENCIES), 70.0);
                    });
                    ui.end_row();
                });
            });
            ui.add_space(8.0);

            // ---- Konum
            card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("2. Konum (Muğla)")).size(17.0).strong());
                egui::Grid::new("f_loc").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("İlçe *"));
                    if combo_pick(ui, "fm_dist", &mut self.form.p.district, &districts, 230.0) {
                        self.form.p.neighborhood.clear();
                        self.form.manual_neigh = false;
                    }
                    ui.end_row();
                    ui.label(ti("Mahalle"));
                    ui.horizontal(|ui| {
                        if self.form.manual_neigh {
                            ui.add(egui::TextEdit::singleline(&mut self.form.p.neighborhood).hint_text(ti("Mahalle / köy adı")).desired_width(230.0));
                        } else {
                            ui.add_enabled_ui(!self.form.p.district.is_empty(), |ui| {
                                combo_pick(ui, "fm_neigh", &mut self.form.p.neighborhood, &neighs, 230.0);
                            });
                        }
                        ui.checkbox(&mut self.form.manual_neigh, ti("Listede yok, elle yazacağım"));
                    });
                    ui.end_row();
                    ui.label(ti("Açık adres"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.p.address).hint_text(ti("Cadde / sokak / site adı, kapı no…")).desired_width(520.0));
                    ui.end_row();
                    ui.label(ti("Harita bağlantısı"));
                    ui.horizontal(|ui| {
                        let r = ui.add(egui::TextEdit::singleline(&mut self.form.link_s).hint_text(ti("Google Haritalar bağlantısını veya \"36.62, 29.11\" yazın")).desired_width(380.0));
                        let mut go = ui.button(ti("Koordinatı çöz")).clicked();
                        if r.changed() && self.form.link_s.contains(|c: char| c == '@' || c == ',' || c == '=') {
                            go = true;
                        }
                        if go && !self.form.link_s.trim().is_empty() {
                            match parse_coords(&self.form.link_s) {
                                Some((la, lo)) => {
                                    self.form.lat_s = format!("{:.6}", la);
                                    self.form.lon_s = format!("{:.6}", lo);
                                }
                                None => {
                                    if !r.changed() {
                                        self.err(ti("Bu bağlantıdan koordinat çıkarılamadı. Haritada yere sağ tıklayıp koordinatı kopyalayın."));
                                    }
                                }
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(ti("Enlem / Boylam"));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.form.lat_s).hint_text(ti("enlem")).desired_width(120.0));
                        ui.add(egui::TextEdit::singleline(&mut self.form.lon_s).hint_text(ti("boylam")).desired_width(120.0));
                        let coords = match (num_opt(&self.form.lat_s, "").ok().flatten(), num_opt(&self.form.lon_s, "").ok().flatten()) {
                            (Some(a), Some(b)) => Some((a, b)),
                            _ => None,
                        };
                        if ui.add_enabled(coords.is_some(), egui::Button::new("Haritada göster")).clicked() {
                            if let Some((a, b)) = coords {
                                self.open_url(&map_url(a, b));
                            }
                        }
                        if ui.button(ti("Haritada yer ara")).clicked() {
                            let q = format!("{} {} {} Muğla", self.form.p.address, self.form.p.neighborhood, self.form.p.district);
                            self.open_url(&format!("https://www.google.com/maps/search/?api=1&query={}", url_encode(q.trim())));
                        }
                    });
                    ui.end_row();
                });
                ui.label(RichText::new(ti("İpucu: \"Haritada yer ara\" ile yeri bulun, işaretçiye sağ tıklayıp koordinatı kopyalayın ve yukarıdaki kutuya yapıştırın.")).small().color(muted()));
            });
            ui.add_space(8.0);

            // ---- Özellikler
            card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("3. Özellikler")).size(17.0).strong());
                egui::Grid::new("f_feat").num_columns(4).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Brüt alan (m²)"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.gross_s).desired_width(110.0));
                    ui.label(ti("Net alan (m²)"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.net_s).desired_width(110.0));
                    ui.end_row();
                    ui.label(ti("Oda sayısı"));
                    combo_pick(ui, "fm_rooms", &mut self.form.p.rooms, &strs(&ROOMS), 110.0);
                    ui.label(ti("Bina yaşı"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.p.building_age).hint_text(ti("örn: 5")).desired_width(110.0));
                    ui.end_row();
                    ui.label(ti("Bulunduğu kat"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.p.floor).hint_text(ti("örn: 3")).desired_width(110.0));
                    ui.label(ti("Toplam kat"));
                    ui.add(egui::TextEdit::singleline(&mut self.form.p.total_floors).hint_text(ti("örn: 5")).desired_width(110.0));
                    ui.end_row();
                    ui.label(ti("Isıtma"));
                    combo_pick(ui, "fm_heat", &mut self.form.p.heating, &strs(&HEATINGS), 170.0);
                    ui.label(ti("Tapu durumu"));
                    combo_pick(ui, "fm_deed", &mut self.form.p.deed_status, &strs(&DEEDS), 170.0);
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.form.p.furnished, ti("Eşyalı"));
                    ui.checkbox(&mut self.form.p.credit_ok, ti("Krediye uygun"));
                    ui.checkbox(&mut self.form.p.swap_ok, ti("Takasa uygun"));
                });
                ui.add_space(4.0);
                ui.label(ti("Açıklama"));
                ui.add(egui::TextEdit::multiline(&mut self.form.p.description).desired_rows(5).desired_width(f32::INFINITY).hint_text(ti("Portföyün tanıtım metni…")));
            });
            ui.add_space(8.0);

            // ---- Fotoğraflar
            card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("4. Fotoğraflar")).size(17.0).strong());
                self.form_photos(ui);
            });
            ui.add_space(8.0);

            // ---- Mülk sahibi (CRM)
            card_frame(ui.style()).fill(note_bg()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("5. Mülk sahibi bilgileri (CRM)")).size(17.0).strong());
                if is_new || self.me.is_admin() {
                    ui.label(RichText::new(ti("Bu bilgiler yalnızca yöneticinin göreceği CRM kaydı olarak saklanır; diğer çalışanlar göremez.")).small().color(muted()));
                    egui::Grid::new("f_owner").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                        ui.label(ti("Ad Soyad"));
                        ui.add(egui::TextEdit::singleline(&mut self.form.p.owner_name).desired_width(320.0));
                        ui.end_row();
                        ui.label(ti("Telefon"));
                        ui.add(egui::TextEdit::singleline(&mut self.form.p.owner_phone).hint_text(ti("0532 123 45 67")).desired_width(320.0));
                        ui.end_row();
                        ui.label(ti("E-posta"));
                        ui.add(egui::TextEdit::singleline(&mut self.form.p.owner_email).desired_width(320.0));
                        ui.end_row();
                        ui.label(ti("Not"));
                        ui.add(egui::TextEdit::multiline(&mut self.form.p.owner_note).desired_rows(2).desired_width(420.0));
                        ui.end_row();
                    });
                } else {
                    ui.label(RichText::new(ti("Mülk sahibi bilgileri yalnızca yönetici tarafından görülür ve düzenlenir.")).color(muted()));
                }
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!saving, egui::Button::new(RichText::new(ti("Kaydet")).strong().color(on_btn())).fill(btn()).min_size(vec2(130.0, 34.0))).clicked() {
                    self.save_form();
                }
                if ui.add_enabled(!saving, egui::Button::new("Vazgeç").min_size(vec2(90.0, 34.0))).clicked() {
                    self.cancel_form();
                }
            });
            ui.add_space(30.0);
            let _ = pos2(0.0, 0.0);
        });
    }
}
