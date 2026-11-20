//! Ana ekran (portföy listesi: küçük resim / liste, konuma göre gruplama) ve portföy detay ekranı.
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::common::*;
use crate::export::{self, Agent, ExportData};
use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, RichText, Sense, Stroke, StrokeKind, Ui,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ------------------------------------------------------------------ ortak çizim yardımcıları

/// Tek satır / sınırlı satır metni verilen genişliğe sığdırarak çizer; yüksekliği döner.
pub fn txt(painter: &Painter, pos: egui::Pos2, s: &str, size: f32, color: Color32, width: f32, max_rows: usize) -> f32 {
    if s.is_empty() || width < 8.0 {
        return 0.0;
    }
    let mut job = LayoutJob::simple(s.to_string(), FontId::proportional(size), color, width);
    job.wrap.max_rows = max_rows;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    let h = galley.size().y;
    painter.galley(pos, galley, color);
    h
}

/// Seçim kutusu: ilk seçenek "Tümü" (boş değer).
pub fn combo_opt(ui: &mut Ui, id: &str, all_label: &str, value: &mut String, options: &[String], width: f32) -> bool {
    let mut changed = false;
    let shown = if value.is_empty() { all_label.to_string() } else { value.clone() };
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(width).show_ui(ui, |ui| {
        if ui.selectable_label(value.is_empty(), all_label).clicked() {
            value.clear();
            changed = true;
        }
        for o in options {
            if ui.selectable_label(value == o, o).clicked() {
                *value = o.clone();
                changed = true;
            }
        }
    });
    changed
}

/// Zorunlu seçim kutusu.
pub fn combo_pick(ui: &mut Ui, id: &str, value: &mut String, options: &[String], width: f32) -> bool {
    let mut changed = false;
    let shown = if value.is_empty() { "Seçiniz…".to_string() } else { value.clone() };
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(width).show_ui(ui, |ui| {
        for o in options {
            if ui.selectable_label(value == o, o).clicked() {
                *value = o.clone();
                changed = true;
            }
        }
    });
    changed
}

pub fn strs(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

pub fn cover_uv(tex_size: egui::Vec2, target: Rect) -> Rect {
    let ra = target.width() / target.height();
    let ta = tex_size.x / tex_size.y;
    if ta > ra {
        let w = ra / ta;
        Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
    } else {
        let h = ta / ra;
        Rect::from_min_max(pos2(0.0, (1.0 - h) / 2.0), pos2(1.0, (1.0 + h) / 2.0))
    }
}

impl App {
    /// Kapak resmini (küçük resim) verilen alana "doldur" biçiminde çizer.
    pub fn paint_cover(&mut self, painter: &Painter, rect: Rect, photo: Option<i64>) {
        painter.rect_filled(rect, 0.0, thumb_bg());
        match photo.and_then(|id| self.tex(TexKey::Thumb(id))) {
            Some(t) => {
                painter.image(t.id(), rect, cover_uv(t.size_vec2(), rect), Color32::WHITE);
            }
            None => {
                let msg = if photo.is_some() { "Yükleniyor…" } else { "Fotoğraf yok" };
                painter.text(rect.center(), Align2::CENTER_CENTER, msg, FontId::proportional(13.0), t_dim());
            }
        }
    }

    fn paint_chip(&self, painter: &Painter, anchor: egui::Pos2, right: bool, text: &str, color: Color32) {
        let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::WHITE);
        let size = galley.size() + vec2(12.0, 4.0);
        let min = if right { pos2(anchor.x - size.x, anchor.y) } else { anchor };
        let r = Rect::from_min_size(min, size);
        painter.rect_filled(r, CornerRadius::same(4), color);
        painter.galley(r.min + vec2(6.0, 2.0), galley, Color32::WHITE);
    }

    fn card(&mut self, ui: &mut Ui, p: &PortfolioSummary, w: f32) -> bool {
        let img_h = (w * 0.68).round();
        let h = img_h + 112.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp.clicked();
        }
        let painter = ui.painter_at(rect);
        let hov = resp.hovered();
        painter.rect_filled(rect, CornerRadius::same(8), if hov { card_hover() } else { card() });
        let img = Rect::from_min_size(rect.min + vec2(1.0, 1.0), vec2(w - 2.0, img_h));
        self.paint_cover(&painter, img, p.cover_photo);
        self.paint_chip(&painter, img.min + vec2(8.0, 8.0), false, &p.status, status_color(&p.status));
        let tcol = if p.listing_type == "Kiralık" { Color32::from_rgb(45, 100, 190) } else { Color32::from_rgb(31, 78, 121) };
        self.paint_chip(&painter, pos2(img.max.x - 8.0, img.min.y + 8.0), true, &p.listing_type, tcol);
        if p.photo_count > 1 {
            self.paint_chip(&painter, pos2(img.max.x - 8.0, img.max.y - 24.0), true, &format!("{} fotoğraf", p.photo_count), Color32::from_black_alpha(150));
        }
        let x = rect.min.x + 10.0;
        let tw = w - 20.0;
        let mut y = img.max.y + 7.0;
        y += txt(&painter, pos2(x, y), &p.title, 15.0, t_strong(), tw, 1) + 2.0;
        let loc = if p.neighborhood.is_empty() { p.district.clone() } else { format!("{} / {}", p.district, p.neighborhood) };
        y += txt(&painter, pos2(x, y), &loc, 13.0, muted(), tw, 1) + 2.0;
        y += txt(&painter, pos2(x, y), &fmt_price(p.price, &p.currency), 17.0, accent(), tw, 1) + 2.0;
        let mut meta: Vec<String> = vec![p.category.clone()];
        if p.gross_m2 > 0.0 {
            meta.push(fmt_m2(p.gross_m2));
        }
        if !p.rooms.is_empty() && p.rooms != "-" {
            meta.push(p.rooms.clone());
        }
        let _ = txt(&painter, pos2(x, y), &meta.join(" • "), 12.5, t_mid(), tw, 1);
        txt(&painter, pos2(x, rect.max.y - 21.0), &format!("{} • {}", p.creator_name, date_tr(&p.created_at)), 11.5, t_dim(), tw, 1);
        painter.rect_stroke(rect, CornerRadius::same(8), Stroke::new(if hov { 1.5_f32 } else { 1.0_f32 }, if hov { accent() } else { stroke() }), StrokeKind::Inside);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    fn list_row(&mut self, ui: &mut Ui, p: &PortfolioSummary) -> bool {
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 66.0), Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp.clicked();
        }
        let painter = ui.painter_at(rect);
        let hov = resp.hovered();
        painter.rect_filled(rect, CornerRadius::same(6), if hov { card_hover() } else { card() });
        painter.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0_f32, if hov { accent() } else { stroke() }), StrokeKind::Inside);
        let img = Rect::from_min_size(rect.min + vec2(5.0, 5.0), vec2(80.0, 56.0));
        self.paint_cover(&painter, img, p.cover_photo);
        let inner = w - 100.0;
        let cols = [0.30, 0.20, 0.15, 0.17, 0.18];
        let xs: Vec<f32> = {
            let mut acc = rect.min.x + 95.0;
            cols.iter()
                .map(|c| {
                    let x = acc;
                    acc += c * inner;
                    x
                })
                .collect()
        };
        let cw = |i: usize| cols[i] * inner - 8.0;
        let cy = rect.min.y + 10.0;
        txt(&painter, pos2(xs[0], cy), &p.title, 15.0, t_strong(), cw(0), 1);
        txt(&painter, pos2(xs[0], cy + 22.0), &format!("{} • {}", p.listing_type, p.category), 12.5, muted(), cw(0), 1);
        let loc = if p.neighborhood.is_empty() { p.district.clone() } else { format!("{} / {}", p.district, p.neighborhood) };
        txt(&painter, pos2(xs[1], cy), &loc, 13.5, t_strong(), cw(1), 2);
        txt(&painter, pos2(xs[2], cy), &fmt_price(p.price, &p.currency), 15.0, accent(), cw(2), 1);
        let mut meta: Vec<String> = vec![];
        if p.gross_m2 > 0.0 {
            meta.push(fmt_m2(p.gross_m2));
        }
        if !p.rooms.is_empty() && p.rooms != "-" {
            meta.push(p.rooms.clone());
        }
        txt(&painter, pos2(xs[3], cy), &meta.join(" • "), 13.0, t_mid(), cw(3), 1);
        self.paint_chip(&painter, pos2(xs[4], cy - 1.0), false, &p.status, status_color(&p.status));
        txt(&painter, pos2(xs[4], cy + 22.0), &format!("{} • {}", p.creator_name, date_tr(&p.created_at)), 11.5, t_dim(), cw(4), 1);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    fn draw_items(&mut self, ui: &mut Ui, items: &[&PortfolioSummary], clicked: &mut Option<i64>) {
        if self.filt.list_mode {
            ui.spacing_mut().item_spacing.y = 6.0;
            for p in items {
                if self.list_row(ui, p) {
                    *clicked = Some(p.id);
                }
            }
        } else {
            let gap = 12.0;
            let avail = ui.available_width() - 6.0;
            let tw = self.thumb_w.clamp(170.0, 420.0);
            let cols = (((avail + gap) / (tw + gap)).floor() as usize).max(1);
            let w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).clamp(tw * 0.85, tw * 1.35);
            for row in items.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for p in row {
                        if self.card(ui, p, w) {
                            *clicked = Some(p.id);
                        }
                    }
                });
                ui.add_space(gap - 4.0);
            }
        }
    }

    fn home_filters(&mut self, ui: &mut Ui) {
        let mut changed = false;
        let districts: Vec<String> = self.locations.iter().map(|d| d.name.clone()).collect();
        let neighs: Vec<String> = self
            .locations
            .iter()
            .find(|d| d.name == self.filt.district)
            .map(|d| d.neighborhoods.iter().map(|n| n.name.clone()).collect())
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.filt.q).hint_text(ti("Ara: başlık, adres, mahalle, P-00012…")).desired_width(250.0));
            changed |= r.changed();
            changed |= combo_opt(ui, "f_type", "Tüm türler", &mut self.filt.ltype, &strs(&LISTING_TYPES), 110.0);
            changed |= combo_opt(ui, "f_status", "Tüm durumlar", &mut self.filt.status, &strs(&STATUSES), 120.0);
            if combo_opt(ui, "f_district", "Tüm ilçeler", &mut self.filt.district, &districts, 130.0) {
                self.filt.neighborhood.clear();
                changed = true;
            }
            if !self.filt.district.is_empty() {
                changed |= combo_opt(ui, "f_neigh", "Tüm mahalleler", &mut self.filt.neighborhood, &neighs, 170.0);
            }
            changed |= ui.checkbox(&mut self.filt.mine, ti("Sadece benimkiler")).changed();
            changed |= ui.checkbox(&mut self.filt.stale, ti("60+ gündür yayında")).on_hover_text(ti("60 günden uzun süredir yayında olan portföyler")).changed();
            if ui.button(ti("Temizle")).clicked() {
                let (lm, g) = (self.filt.list_mode, self.filt.group);
                self.filt = Filters::default();
                self.filt.list_mode = lm;
                self.filt.group = g;
                changed = true;
            }
        });
        if changed {
            self.show_n = 60;
            self.load_portfolios();
        }
    }

    pub fn ui_home(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            page_title(ui, "Portföyler");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new(ti("+ Yeni Portföy")).strong().color(on_btn())).fill(btn())).clicked() {
                    self.start_new_portfolio();
                }
                if ui.button(ti("CSV")).on_hover_text(ti("Listeyi Excel için CSV olarak kaydet")).clicked() {
                    let mut c = String::from("No;Başlık;Tür;Kategori;İlçe;Mahalle;Fiyat;Para birimi;m2;Oda;Durum;Ekleyen;Tarih\n");
                    for x in &self.portfolios {
                        c.push_str(&format!("P-{:05};{};{};{};{};{};{};{};{};{};{};{};{}\n", x.id, crate::ui_ext::csv_cell(&x.title), x.listing_type, x.category, x.district, x.neighborhood, x.price, x.currency, x.gross_m2, x.rooms, x.status, crate::ui_ext::csv_cell(&x.creator_name), x.created_at));
                    }
                    self.save_csv("portfoyler.csv", c);
                }
                ui.checkbox(&mut self.filt.group, ti("İlçe / mahalleye göre grupla"));
                if !self.filt.list_mode {
                    ui.add(egui::Slider::new(&mut self.thumb_w, 170.0..=420.0).show_value(false)).on_hover_text(ti("Küçük resim boyutu"));
                    ui.label(RichText::new(ti("Boyut:")).color(muted()));
                }
                ui.selectable_value(&mut self.filt.list_mode, true, ti("Liste"));
                ui.selectable_value(&mut self.filt.list_mode, false, ti("Küçük resimler"));
                ui.label(RichText::new(ti("Görünüm:")).color(muted()));
            });
        });
        self.update_banner(ui);
        self.ann_banner(ui);
        self.home_filters(ui);
        ui.add_space(2.0);
        ui.label(RichText::new(format!("{} portföy (en yeniden en eskiye)", self.portfolios.len())).small().color(muted()));
        ui.separator();

        let items = std::mem::take(&mut self.portfolios);
        let mut clicked: Option<i64> = None;
        if items.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(ti("Gösterilecek portföy yok.")).size(18.0).color(muted()));
                ui.label(RichText::new(ti("Filtreleri temizleyin veya yeni bir portföy ekleyin.")).color(muted()));
            });
        } else {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if self.filt.group {
                    let mut groups: BTreeMap<&str, BTreeMap<&str, Vec<&PortfolioSummary>>> = BTreeMap::new();
                    for p in &items {
                        let d = if p.district.is_empty() { "İlçe belirtilmemiş" } else { p.district.as_str() };
                        let n = if p.neighborhood.is_empty() { "Mahalle belirtilmemiş" } else { p.neighborhood.as_str() };
                        groups.entry(d).or_default().entry(n).or_default().push(p);
                    }
                    for (d, neighs) in &groups {
                        let total: usize = neighs.values().map(|v| v.len()).sum();
                        egui::CollapsingHeader::new(RichText::new(format!("{d}  ({total})")).size(18.0).strong().color(accent()))
                            .id_salt(("grp", d))
                            .default_open(groups.len() == 1)
                            .show(ui, |ui| {
                                for (n, list) in neighs {
                                    egui::CollapsingHeader::new(RichText::new(format!("{n}  ({})", list.len())).size(15.0).strong())
                                        .id_salt(("grp", d, n))
                                        .default_open(true)
                                        .show(ui, |ui| self.draw_items(ui, list, &mut clicked));
                                }
                            });
                    }
                } else {
                    let n = self.show_n.min(items.len());
                    let list: Vec<&PortfolioSummary> = items.iter().take(n).collect();
                    self.draw_items(ui, &list, &mut clicked);
                    if items.len() > n {
                        ui.add_space(6.0);
                        if ui.button(format!("Daha fazla göster ({} kayıt daha)", items.len() - n)).clicked() {
                            self.show_n += 60;
                        }
                    }
                }
            });
        }
        self.portfolios = items;
        if let Some(id) = clicked {
            self.open_detail(id);
        }
    }

    // ------------------------------------------------------------------ detay

    fn agent_user(&self, p: &Portfolio) -> UserInfo {
        if !self.agent_me {
            if let Some(u) = self.users.iter().find(|u| u.id == p.creator_id) {
                return u.clone();
            }
        }
        self.me.clone()
    }

    fn detail_actions(&mut self, ui: &mut Ui, p: &Portfolio) {
        let can_edit = self.me.is_admin() || p.creator_id == self.me.id;
        ui.horizontal_wrapped(|ui| {
            if ui.button(ti("← Portföylere dön")).clicked() {
                self.goto(View::Home);
            }
            ui.separator();
            if can_edit && ui.button(ti("Düzenle")).clicked() {
                self.start_edit_portfolio(p);
            }
            ui.horizontal(|ui| {
                ui.label(ti("Belge dili:"));
                egui::ComboBox::from_id_salt("doc_lang").selected_text(self.doc_lang.label()).show_ui(ui, |ui| {
                    for l in crate::i18n_doc::Lang::ALL {
                        ui.selectable_value(&mut self.doc_lang, l, l.label());
                    }
                });
            });
            ui.add_enabled_ui(!self.exporting, |ui| {
                if ui.button(ti("PDF olarak kaydet")).clicked() {
                    self.export_doc(false);
                }
                if ui.button(ti("Word (DOCX) olarak kaydet")).clicked() {
                    self.export_doc(true);
                }
            });
            if ui.button(ti("WhatsApp'ta paylaş")).clicked() {
                if self.share.is_some() {
                    self.share = None;
                } else {
                    let au = self.agent_user(p);
                    let agent = Some(Agent { name: au.full_name(), phone: au.phone.clone(), email: au.email.clone(), photo: None });
                    self.share = Some(ShareState { text: export::whatsapp_text(p, &self.settings, &agent, self.doc_lang), phone: String::new() });
                }
            }
            if self.exporting {
                ui.spinner();
                ui.label(ti("Belge hazırlanıyor…"));
            }
            if self.me.is_admin() {
                ui.separator();
                if confirm_btn(ui, "del_pf", "Portföyü sil") {
                    let id = p.id;
                    self.call(move |api| api.delete(&format!("/api/portfolios/{id}")), |app, _| {
                        app.toast(ti("Portföy silindi."));
                        app.goto(View::Home);
                    });
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(ti("Belge altındaki iletişim bilgisi:")).small().color(muted()));
            ui.radio_value(&mut self.agent_me, true, format!("Benim bilgilerim ({})", self.me.full_name()));
            if p.creator_id != self.me.id {
                ui.radio_value(&mut self.agent_me, false, format!("Portföyü ekleyen: {}", p.creator_name));
            }
        });
        if let Some(path) = self.last_export.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(30, 120, 60), format!("Kaydedildi: {}", path.display()));
                if ui.small_button(ti("Dosyayı aç")).clicked() {
                    self.open_path(&path);
                }
                if let Some(dir) = path.parent() {
                    if ui.small_button(ti("Klasörü aç")).clicked() {
                        self.open_path(&dir.to_path_buf());
                    }
                }
            });
        }
        self.share_panel(ui);
    }

    fn share_panel(&mut self, ui: &mut Ui) {
        let Some(mut sh) = self.share.take() else { return };
        let mut keep = true;
        card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
            ui.label(RichText::new(ti("WhatsApp ile paylaş")).strong());
            ui.label(RichText::new(ti("Aşağıdaki metin WhatsApp'a aktarılır; göndermeden önce düzenleyebilirsiniz. Fotoğraflı PDF/DOCX göndermek için önce dosyayı kaydedin, sonra WhatsApp'ta ataç (📎) ile ekleyin.")).small().color(muted()));
            ui.add(egui::TextEdit::multiline(&mut sh.text).desired_rows(7).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label(ti("Telefon (isteğe bağlı):"));
                ui.add(egui::TextEdit::singleline(&mut sh.phone).hint_text(ti("0532 123 45 67")).desired_width(160.0));
                if ui.add(egui::Button::new(RichText::new(ti("WhatsApp'ta aç")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(37, 160, 90))).clicked() {
                    let num = wa_number(&sh.phone);
                    let url = if num.is_empty() { format!("https://wa.me/?text={}", url_encode(&sh.text)) } else { format!("https://wa.me/{}?text={}", num, url_encode(&sh.text)) };
                    self.open_url(&url);
                }
                if ui.button(ti("Metni kopyala")).clicked() {
                    ui.ctx().copy_text(sh.text.clone());
                    self.toast(ti("Metin panoya kopyalandı."));
                }
                if let Some(dir) = self.last_export.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf()) {
                    if ui.button(ti("Kaydedilen dosyanın klasörünü aç")).clicked() {
                        self.open_path(&dir);
                    }
                }
                if ui.button(ti("Kapat")).clicked() {
                    keep = false;
                }
            });
        });
        if keep {
            self.share = Some(sh);
        }
    }

    fn export_doc(&mut self, docx: bool) {
        let Some(p) = self.detail.clone() else { return };
        let ext: &'static str = if docx { "docx" } else { "pdf" };
        let slug: String = p
            .title
            .chars()
            .map(|c| match c {
                'ı' | 'İ' => 'i',
                'ş' | 'Ş' => 's',
                'ğ' | 'Ğ' => 'g',
                'ü' | 'Ü' => 'u',
                'ö' | 'Ö' => 'o',
                'ç' | 'Ç' => 'c',
                c if c.is_ascii_alphanumeric() => c,
                _ => '_',
            })
            .collect();
        let name = format!("{}_{}.{}", p.listing_no(), slug.trim_matches('_'), ext);
        self.save_dialog(&name, ext, move |app, path| {
            if let Some(path) = path {
                app.run_export(p, docx, path);
            }
        });
    }

    fn run_export(&mut self, p: Portfolio, docx: bool, path: PathBuf) {
        let au = self.agent_user(&p);
        let settings = self.settings.clone();
        let lang = self.doc_lang;
        self.exporting = true;
        self.bg(
            move |api| -> Result<PathBuf, String> {
                let mut photos = vec![];
                if settings.show_photos {
                    for id in p.photos.iter().take(settings.max_photos.max(1) as usize) {
                        photos.push(api.get_bytes(&format!("/api/photos/{id}"))?);
                    }
                }
                let logo = if settings.show_logo { api.get_bytes("/api/settings/logo").ok() } else { None };
                let photo = if au.has_photo && settings.show_agent_photo { api.get_bytes(&format!("/api/users/{}/photo", au.id)).ok() } else { None };
                let agent = Some(Agent { name: au.full_name(), phone: au.phone.clone(), email: au.email.clone(), photo });
                let d = ExportData { p, photos, settings, agent, logo, lang };
                if docx { export::write_docx(&d, &path)? } else { export::write_pdf(&d, &path)? }
                Ok(path)
            },
            |app, r| {
                app.exporting = false;
                match r {
                    Ok(path) => {
                        app.toast(ti("Belge kaydedildi."));
                        app.last_export = Some(path);
                    }
                    Err(e) => app.err(format!("Belge oluşturulamadı: {e}")),
                }
            },
        );
    }

    fn gallery(&mut self, ui: &mut Ui, p: &Portfolio) {
        let n = p.photos.len();
        let w = ui.available_width();
        let h = (w * 0.68).min(520.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(6), Color32::from_gray(30));
        if n == 0 {
            painter.text(rect.center(), Align2::CENTER_CENTER, "Bu portföyde fotoğraf yok", FontId::proportional(16.0), Color32::from_gray(170));
            return;
        }
        let idx = self.photo_idx.min(n - 1);
        let id = p.photos[idx];
        let tex = self.tex(TexKey::Full(id)).or_else(|| self.tex(TexKey::Thumb(id)));
        match tex {
            Some(t) => {
                let ts = t.size_vec2();
                let scale = (rect.width() / ts.x).min(rect.height() / ts.y);
                let r = Rect::from_center_size(rect.center(), ts * scale);
                painter.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                painter.text(rect.center(), Align2::CENTER_CENTER, "Yükleniyor…", FontId::proportional(15.0), Color32::from_gray(170));
            }
        }
        if resp.clicked() && n > 1 {
            self.photo_idx = (idx + 1) % n;
        }
        ui.horizontal(|ui| {
            if n > 1 {
                if ui.button(ti("‹ Önceki")).clicked() {
                    self.photo_idx = (idx + n - 1) % n;
                }
                ui.label(format!("{} / {}", idx + 1, n));
                if ui.button(ti("Sonraki ›")).clicked() {
                    self.photo_idx = (idx + 1) % n;
                }
            } else {
                ui.label(ti("1 fotoğraf"));
            }
            if idx == 0 {
                ui.label(RichText::new(ti("(kapak fotoğrafı)")).small().color(muted()));
            }
        });
        if n > 1 {
            egui::ScrollArea::horizontal().id_salt("thumbs").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, pid) in p.photos.iter().enumerate() {
                        let (r, rs) = ui.allocate_exact_size(vec2(84.0, 60.0), Sense::click());
                        if ui.is_rect_visible(r) {
                            let painter = ui.painter_at(r);
                            self.paint_cover(&painter, r, Some(*pid));
                            if i == idx {
                                painter.rect_stroke(r, 0.0, Stroke::new(3.0_f32, accent()), StrokeKind::Inside);
                            }
                        }
                        if rs.clicked() {
                            self.photo_idx = i;
                        }
                    }
                });
            });
        }
    }

    fn detail_info(&mut self, ui: &mut Ui, p: &Portfolio) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(p.listing_no()).color(muted()));
            badge(ui, &p.status, status_color(&p.status));
            badge(ui, &p.listing_type, accent());
        });
        ui.label(RichText::new(&p.title).size(24.0).strong());
        ui.label(RichText::new(fmt_price(p.price, &p.currency)).size(28.0).color(accent()).strong());
        ui.add_space(4.0);
        let rows = export::detail_rows(p, crate::i18n_doc::Lang::Tr);
        egui::Grid::new("detail_grid").num_columns(2).spacing([16.0, 5.0]).striped(true).show(ui, |ui| {
            for (k, v) in rows {
                ui.label(RichText::new(k).color(muted()));
                ui.label(RichText::new(v).strong());
                ui.end_row();
            }
        });
        if !p.address.trim().is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(ti("Adres")).color(muted()).small());
            ui.label(&p.address);
        }
        if let (Some(la), Some(lo)) = (p.lat, p.lon) {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Konum: {:.5}, {:.5}", la, lo)).color(muted()));
                if ui.button(ti("Haritada aç")).clicked() {
                    self.open_url(&map_url(la, lo));
                }
                if ui.small_button(ti("Koordinatı kopyala")).clicked() {
                    ui.ctx().copy_text(format!("{:.6}, {:.6}", la, lo));
                    self.toast(ti("Koordinat kopyalandı."));
                }
            });
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Ekleyen: {}  •  Kayıt: {}  •  Güncelleme: {}", p.creator_name, dt_tr(&p.created_at), dt_tr(&p.updated_at)))
                .small()
                .color(muted()),
        );
        if !p.published_at.is_empty() {
            ui.label(RichText::new(format!("Yayına alınma: {}", dt_tr(&p.published_at))).small().color(muted()));
        }
        // Mülk sahibi (CRM) — yalnızca yönetici (sunucu zaten diğerlerine göndermez)
        if self.me.is_admin() && (!p.owner_name.is_empty() || !p.owner_phone.is_empty() || !p.owner_email.is_empty()) {
            ui.add_space(10.0);
            card_frame(ui.style()).fill(note_bg()).inner_margin(10.0).show(ui, |ui| {
                ui.label(RichText::new(ti("Mülk sahibi (yalnızca yönetici görür)")).strong());
                if !p.owner_name.is_empty() {
                    ui.label(format!("Ad Soyad: {}", p.owner_name));
                }
                if !p.owner_phone.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Telefon: {}", p.owner_phone));
                        if ui.small_button(ti("WhatsApp")).clicked() {
                            let url = format!("https://wa.me/{}", wa_number(&p.owner_phone));
                            self.open_url(&url);
                        }
                    });
                }
                if !p.owner_email.is_empty() {
                    ui.label(format!("E-posta: {}", p.owner_email));
                }
                if !p.owner_note.is_empty() {
                    ui.label(format!("Not: {}", p.owner_note));
                }
            });
        }
    }

    pub fn ui_detail(&mut self, ui: &mut Ui) {
        let Some(p) = self.detail.clone() else {
            if ui.button(ti("← Portföylere dön")).clicked() {
                self.goto(View::Home);
            }
            ui.add_space(20.0);
            ui.label(ti("Yükleniyor…"));
            return;
        };
        self.detail_actions(ui, &p);
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.columns(2, |cols| {
                self.gallery(&mut cols[0], &p);
                self.detail_info(&mut cols[1], &p);
            });
            ui.add_space(8.0);
            self.listing_text_ui(ui, &p);
            self.web_card_ui(ui, &p);
            self.detail_extras(ui);
        });
    }
}
