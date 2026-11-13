//! Sürüm 1.2 arayüzleri (1/2): ortak durum, panel, alıcılar, takvim, duyurular.
use crate::app::*;
use crate::common::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::ui_portfolio::{combo_opt, combo_pick, strs};
use chrono::{Datelike, Duration as CDur, NaiveDate};
use eframe::egui::{self, vec2, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Ui};
use egui_extras::DatePickerButton;
use serde_json::{json, Value};

pub const MONTHS_TR: [&str; 12] = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];
pub const DAYS_TR: [&str; 7] = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"];
pub const APPT_KINDS: [&str; 5] = ["Yer gösterme", "Tapu işlemi", "Toplantı", "Telefon görüşmesi", "Diğer"];
pub const LEAVE_KINDS: [&str; 5] = ["Yıllık izin", "Hastalık izni", "Mazeret izni", "Ücretsiz izin", "Diğer"];

pub fn js(v: &Value, k: &str) -> String {
    match &v[k] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
pub fn jf(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or(0.0)
}
pub fn ji(v: &Value, k: &str) -> i64 {
    v[k].as_i64().unwrap_or(0)
}
pub fn month_label(d: NaiveDate) -> String {
    format!("{} {}", MONTHS_TR[(d.month() - 1) as usize], d.year())
}
pub fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let m0 = d.year() * 12 + d.month() as i32 - 1 + n;
    NaiveDate::from_ymd_opt(m0.div_euclid(12), (m0.rem_euclid(12) + 1) as u32, 1).unwrap_or(d)
}
pub fn csv_cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub fn num_tr(v: f64) -> String {
    fmt_thousands(v.round())
}

#[derive(Default, Clone)]
pub struct BuyerForm {
    pub id: i64,
    pub full_name: String,
    pub phone: String,
    pub email: String,
    pub listing_type: String,
    pub category: String,
    pub district: String,
    pub neighborhood: String,
    pub min_s: String,
    pub max_s: String,
    pub currency: String,
    pub rooms: String,
    pub note: String,
    pub status: String,
    pub consent: bool,
    pub consent_date: String,
}

impl BuyerForm {
    pub fn blank() -> Self {
        BuyerForm { listing_type: "Satılık".into(), currency: "TL".into(), status: "Aktif".into(), ..Default::default() }
    }
    pub fn from_value(v: &Value) -> Self {
        BuyerForm {
            id: ji(v, "id"),
            full_name: js(v, "full_name"),
            phone: js(v, "phone"),
            email: js(v, "email"),
            listing_type: js(v, "listing_type"),
            category: js(v, "category"),
            district: js(v, "district"),
            neighborhood: js(v, "neighborhood"),
            min_s: if jf(v, "budget_min") > 0.0 { num_to_text(jf(v, "budget_min")) } else { String::new() },
            max_s: if jf(v, "budget_max") > 0.0 { num_to_text(jf(v, "budget_max")) } else { String::new() },
            currency: js(v, "currency"),
            rooms: js(v, "rooms"),
            note: js(v, "note"),
            status: js(v, "status"),
            consent: ji(v, "consent") != 0,
            consent_date: js(v, "consent_date"),
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "full_name": self.full_name, "phone": self.phone, "email": self.email, "listing_type": self.listing_type,
            "category": self.category, "district": self.district, "neighborhood": self.neighborhood,
            "budget_min": parse_num(&self.min_s, true), "budget_max": parse_num(&self.max_s, true),
            "currency": self.currency, "rooms": self.rooms, "note": self.note, "status": self.status, "consent": self.consent,
        })
    }
}

#[derive(Clone)]
pub struct ApptForm {
    pub id: i64,
    pub title: String,
    pub kind: String,
    pub date: NaiveDate,
    pub hour: i64,
    pub minute: i64,
    pub duration: i64,
    pub user_id: i64,
    pub portfolio_id: i64,
    pub buyer_id: i64,
    pub note: String,
    pub status: String,
    pub force: bool,
    pub warn: String,
}

impl ApptForm {
    pub fn blank(date: NaiveDate, user_id: i64) -> Self {
        ApptForm {
            id: 0, title: String::new(), kind: "Yer gösterme".into(), date, hour: 10, minute: 0, duration: 60, user_id,
            portfolio_id: 0, buyer_id: 0, note: String::new(), status: "Planlı".into(), force: false, warn: String::new(),
        }
    }
    pub fn from_value(v: &Value) -> Self {
        let st = js(v, "start_at");
        let date = NaiveDate::parse_from_str(st.get(..10).unwrap_or(""), "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive());
        let hh = st.get(11..13).and_then(|s| s.parse().ok()).unwrap_or(10);
        let mm = st.get(14..16).and_then(|s| s.parse().ok()).unwrap_or(0);
        ApptForm {
            id: ji(v, "id"), title: js(v, "title"), kind: js(v, "kind"), date, hour: hh, minute: mm, duration: ji(v, "duration_min").max(5),
            user_id: ji(v, "user_id"), portfolio_id: ji(v, "portfolio_id"), buyer_id: ji(v, "buyer_id"), note: js(v, "note"),
            status: js(v, "status"), force: false, warn: String::new(),
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "title": self.title, "kind": self.kind, "start_at": format!("{} {:02}:{:02}", self.date.format("%Y-%m-%d"), self.hour, self.minute),
            "duration_min": self.duration, "user_id": self.user_id, "portfolio_id": self.portfolio_id, "buyer_id": self.buyer_id,
            "note": self.note, "status": self.status, "force": self.force,
        })
    }
}

#[derive(Clone)]
pub struct LeaveForm {
    pub kind: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub note: String,
    pub user_id: i64,
}

#[derive(Clone)]
pub struct CommForm {
    pub id: i64,
    pub portfolio_id: i64,
    pub user_id: i64,
    pub deal_type: String,
    pub deal_s: String,
    pub currency: String,
    pub rate_s: String,
    pub comm_s: String,
    pub pct_s: String,
    pub status: String,
    pub date: NaiveDate,
    pub note: String,
}

#[derive(Default, Clone)]
pub struct AnnForm {
    pub show: bool,
    pub title: String,
    pub body: String,
    pub pinned: bool,
}

#[derive(Default, Clone)]
pub struct ExtSetEdit {
    pub dir: String,
    pub pw: String,
    pub clear_pw: bool,
    pub pw_set: bool,
    pub work_start: String,
    pub match_enabled: bool,
    pub loaded: bool,
}

pub struct ExtState {
    pub dash: Option<Value>,
    pub dash_month: NaiveDate,
    pub buyers: Vec<Value>,
    pub buyer_q: String,
    pub buyer_form: Option<BuyerForm>,
    pub buyer_matches: Vec<Value>,
    pub buyer_sel: i64,
    pub appts: Vec<Value>,
    pub cal_month: NaiveDate,
    pub cal_sel: NaiveDate,
    pub cal_user: i64,
    pub appt_form: Option<ApptForm>,
    pub leaves: Vec<Value>,
    pub leave_year: i32,
    pub leave_form: Option<LeaveForm>,
    pub leave_tab: u8,
    pub ts: Option<Value>,
    pub ts_month: NaiveDate,
    pub comms: Vec<Value>,
    pub comm_sum: Vec<Value>,
    pub comm_month: NaiveDate,
    pub comm_all: bool,
    pub comm_form: Option<CommForm>,
    pub audit: Vec<Value>,
    pub audit_q: String,
    pub anns: Vec<Value>,
    pub ann_form: AnnForm,
    pub ann_hidden: bool,
    pub map_pts: Vec<Value>,
    pub map: crate::ui_map::MapState,
    pub hist: Vec<Value>,
    pub notes: Vec<Value>,
    pub matches: Vec<Value>,
    pub note_in: String,
    pub upd: Option<Value>,
    pub upd_busy: bool,
    pub upd_msg: String,
    pub ext_set: ExtSetEdit,
    pub restore_pw: String,
    pub restore_pick: Option<std::path::PathBuf>,
    pub born: std::time::Instant,
    pub verify_pending: bool,
    pub upd_banner_hidden: bool,
    pub upd_pkg: Option<crate::updater::Package>,
}

impl ExtState {
    pub fn new() -> Self {
        let today = chrono::Local::now().date_naive();
        let first = today.with_day(1).unwrap_or(today);
        ExtState {
            dash: None,
            dash_month: first,
            buyers: vec![],
            buyer_q: String::new(),
            buyer_form: None,
            buyer_matches: vec![],
            buyer_sel: 0,
            appts: vec![],
            cal_month: first,
            cal_sel: today,
            cal_user: 0,
            appt_form: None,
            leaves: vec![],
            leave_year: today.year(),
            leave_form: None,
            leave_tab: 0,
            ts: None,
            ts_month: first,
            comms: vec![],
            comm_sum: vec![],
            comm_month: first,
            comm_all: false,
            comm_form: None,
            audit: vec![],
            audit_q: String::new(),
            anns: vec![],
            ann_form: AnnForm::default(),
            ann_hidden: false,
            map_pts: vec![],
            map: Default::default(),
            hist: vec![],
            notes: vec![],
            matches: vec![],
            note_in: String::new(),
            upd: None,
            upd_busy: false,
            upd_msg: String::new(),
            ext_set: ExtSetEdit { match_enabled: true, ..Default::default() },
            restore_pw: String::new(),
            restore_pick: None,
            born: std::time::Instant::now(),
            verify_pending: true,
            upd_banner_hidden: false,
            upd_pkg: None,
        }
    }
}

pub fn panel_box(ui: &mut Ui, title: &str, hint: &str, add: impl FnOnce(&mut Ui)) {
    card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        if !title.is_empty() {
            ui.label(RichText::new(title).size(16.0).strong().color(t_strong()));
        }
        if !hint.is_empty() {
            ui.label(RichText::new(hint).small().color(muted()));
        }
        if !title.is_empty() || !hint.is_empty() {
            ui.add_space(4.0);
        }
        add(ui);
    });
    ui.add_space(8.0);
}

pub fn primary(ui: &mut Ui, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).strong().color(on_btn())).fill(btn())).clicked()
}

pub fn month_nav(ui: &mut Ui, d: &mut NaiveDate) -> bool {
    let mut changed = false;
    if ui.button(ti("◀")).clicked() {
        *d = add_months(*d, -1);
        changed = true;
    }
    ui.label(RichText::new(month_label(*d)).size(16.0).strong());
    if ui.button(ti("▶")).clicked() {
        *d = add_months(*d, 1);
        changed = true;
    }
    if ui.small_button(ti("Bu ay")).clicked() {
        let t = chrono::Local::now().date_naive();
        *d = t.with_day(1).unwrap_or(t);
        changed = true;
    }
    changed
}

fn status_badge_color(s: &str) -> Color32 {
    match s {
        "Onaylandı" | "Tamamlandı" | "Ödendi" | "Aktif" => chart(1),
        "Reddedildi" | "İptal" | "Pasif" => chart(3),
        "Bekliyor" => chart(4),
        _ => chart(0),
    }
}

impl App {
    // ------------------------------------------------------------ yükleyiciler
    pub fn load_dashboard(&mut self) {
        let m = self.ext.dash_month.format("%Y-%m").to_string();
        self.call(move |api| api.get::<Value>(&format!("/api/dashboard?month={m}")), |app, v| app.ext.dash = Some(v));
    }
    pub fn load_buyers(&mut self) {
        self.call(|api| api.get::<Vec<Value>>("/api/buyers"), |app, v| app.ext.buyers = v);
    }
    pub fn load_buyer_matches(&mut self, id: i64) {
        self.ext.buyer_matches.clear();
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/buyers/{id}/matches")), move |app, v| {
            if app.ext.buyer_sel == id {
                app.ext.buyer_matches = v;
            }
        });
    }
    pub fn load_appts(&mut self) {
        let first = self.ext.cal_month;
        let from = (first - CDur::days(8)).format("%Y-%m-%d").to_string();
        let to = (add_months(first, 1) + CDur::days(8)).format("%Y-%m-%d").to_string();
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/appointments?from={from}&to={to}")), |app, v| app.ext.appts = v);
    }
    pub fn load_leaves(&mut self) {
        let y = self.ext.leave_year;
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/leaves?year={y}")), |app, v| app.ext.leaves = v);
        let m = self.ext.ts_month.format("%Y-%m").to_string();
        self.call(move |api| api.get::<Value>(&format!("/api/timesheet?month={m}")), |app, v| app.ext.ts = Some(v));
    }
    pub fn load_comms(&mut self) {
        let m = if self.ext.comm_all { String::new() } else { self.ext.comm_month.format("%Y-%m").to_string() };
        let m2 = m.clone();
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/commissions?month={m}")), |app, v| app.ext.comms = v);
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/commissions/summary?month={m2}")), |app, v| app.ext.comm_sum = v);
    }
    pub fn load_audit(&mut self) {
        let q = url_encode(&self.ext.audit_q);
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/audit?limit=400&q={q}")), |app, v| app.ext.audit = v);
    }
    pub fn load_anns(&mut self) {
        self.call(|api| api.get::<Vec<Value>>("/api/announcements"), |app, v| app.ext.anns = v);
    }
    pub fn load_detail_extras(&mut self, id: i64) {
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/portfolios/{id}/history")), move |app, v| {
            if app.detail_id == id {
                app.ext.hist = v;
            }
        });
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/portfolios/{id}/notes")), move |app, v| {
            if app.detail_id == id {
                app.ext.notes = v;
            }
        });
        self.call(move |api| api.get::<Vec<Value>>(&format!("/api/portfolios/{id}/matches")), move |app, v| {
            if app.detail_id == id {
                app.ext.matches = v;
            }
        });
    }

    pub fn save_csv(&mut self, name: &str, content: String) {
        self.save_dialog(name, "csv", move |app, p| {
            if let Some(p) = p {
                let mut bytes = vec![0xEF, 0xBB, 0xBF];
                bytes.extend_from_slice(content.as_bytes());
                match std::fs::write(&p, bytes) {
                    Ok(_) => app.toast(format!("Kaydedildi: {}", p.display())),
                    Err(e) => app.err(format!("Dosya yazılamadı: {e}")),
                }
            }
        });
    }

    // ------------------------------------------------------------ PANEL
    pub fn ui_dashboard(&mut self, ui: &mut Ui) {
        page_title(ui, "Panel");
        let lead = self.me.is_lead();
        let mut reload = false;
        ui.horizontal(|ui| {
            let mut d = self.ext.dash_month;
            if month_nav(ui, &mut d) {
                self.ext.dash_month = d;
                reload = true;
            }
            if ui.button(ti("Yenile")).clicked() {
                reload = true;
            }
            if lead {
                ui.separator();
                if ui.button(ti("Rapor (CSV)")).clicked() {
                    if let Some(v) = self.ext.dash.clone() {
                        let csv = dashboard_csv(&v);
                        let name = format!("rapor_{}.csv", self.ext.dash_month.format("%Y-%m"));
                        self.save_csv(&name, csv);
                    }
                }
                if ui.button(ti("Rapor (PDF)")).clicked() {
                    if let Some(v) = self.ext.dash.clone() {
                        let office = self.settings.office_name.clone();
                        let name = format!("rapor_{}.pdf", self.ext.dash_month.format("%Y-%m"));
                        self.save_dialog(&name, "pdf", move |app, p| {
                            if let Some(p) = p {
                                match crate::report::write_report_pdf(&v, &office, &p) {
                                    Ok(_) => app.toast(format!("Kaydedildi: {}", p.display())),
                                    Err(e) => app.err(e),
                                }
                            }
                        });
                    }
                }
            }
        });
        if reload {
            self.load_dashboard();
        }
        let Some(d) = self.ext.dash.clone() else {
            ui.label(RichText::new(ti("Yükleniyor…")).color(muted()));
            return;
        };
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let month_lbl = month_label(self.ext.dash_month);
            let mut show_stale = false;
            let mut tiles: Vec<(String, i64, Color32)> = vec![
                ("Yayındaki portföy".into(), ji(&d, "active"), accent()),
                (format!("{month_lbl}: eklenen"), ji(&d, "new_month"), chart(0)),
                (format!("{month_lbl}: yayına alınan"), ji(&d, "published_month"), chart(1)),
                (format!("{month_lbl}: kiralanan"), ji(&d, "rented_month"), chart(2)),
                (format!("{month_lbl}: satılan"), ji(&d, "sold_month"), chart(3)),
                ("60+ gündür yayında".into(), ji(&d, "stale"), chart(4)),
                ("Aktif alıcı".into(), ji(&d, "buyers"), chart(5)),
                ("Açık görev".into(), ji(&d, "open_tasks"), Color32::from_rgb(120, 120, 130)),
                ("Bugünkü randevu".into(), ji(&d, "appts_today"), chart(0)),
                ("7 günlük randevu".into(), ji(&d, "appts_week"), chart(0)),
            ];
            if lead {
                tiles.push(("Bekleyen izin talebi".into(), ji(&d, "pending_leaves"), chart(4)));
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for (label, val, col) in &tiles {
                    let is_stale = label.starts_with("60+");
                    let (rect, tresp) = ui.allocate_exact_size(vec2(178.0, 78.0), if is_stale { Sense::click() } else { Sense::hover() });
                    if is_stale {
                        let r = tresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(ti("Listeyi göster"));
                        if r.clicked() {
                            show_stale = true;
                        }
                    }
                    let p = ui.painter();
                    p.rect_filled(rect, CornerRadius::same(10), card());
                    p.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0_f32, stroke()), egui::StrokeKind::Inside);
                    p.rect_filled(egui::Rect::from_min_size(rect.min, vec2(5.0, rect.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, *col);
                    p.text(rect.min + vec2(16.0, 10.0), Align2::LEFT_TOP, val.to_string(), FontId::proportional(28.0), t_strong());
                    p.text(rect.min + vec2(16.0, 50.0), Align2::LEFT_TOP, label, FontId::proportional(12.5), muted());
                }
            });
            ui.add_space(10.0);
            if show_stale {
                self.filt = Filters { list_mode: self.filt.list_mode, group: self.filt.group, stale: true, ..Default::default() };
                self.goto(View::Home);
            }

            let by_user: Vec<Value> = d["by_user"].as_array().cloned().unwrap_or_default();
            panel_box(ui, &format!("Çalışan performansı — {month_lbl}"), "Eklenen, yayına alınan, kiralanan ve satılan portföy sayıları.", |ui| {
                let maxv = by_user.iter().map(|u| ji(u, "created").max(ji(u, "published")).max(ji(u, "rented")).max(ji(u, "sold"))).max().unwrap_or(1).max(1) as f32;
                egui::Grid::new("perf").striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                    for h in ["Çalışan", "Eklenen", "Yayına alınan", "Kiralanan", "Satılan"] {
                        ui.label(RichText::new(h).strong().color(muted()));
                    }
                    ui.end_row();
                    for u in &by_user {
                        ui.label(RichText::new(js(u, "user")).strong());
                        for (k, col) in [("created", accent()), ("published", chart(1)), ("rented", chart(2)), ("sold", chart(3))] {
                            let n = ji(u, k);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(90.0, 14.0), Sense::hover());
                                ui.painter().rect_filled(r, CornerRadius::same(4), thumb_bg());
                                let w = r.width() * (n as f32 / maxv);
                                if n > 0 {
                                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(w.max(4.0), r.height())), CornerRadius::same(4), col);
                                }
                                ui.label(n.to_string());
                            });
                        }
                        ui.end_row();
                    }
                });
            });

            ui.columns(2, |cols| {
                let by_district: Vec<Value> = d["by_district"].as_array().cloned().unwrap_or_default();
                panel_box(&mut cols[0], "İlçelere göre yayındaki portföyler", "", |ui| {
                    let maxv = by_district.iter().map(|x| ji(x, "n")).max().unwrap_or(1).max(1) as f32;
                    if by_district.is_empty() {
                        ui.label(RichText::new(ti("Veri yok.")).color(muted()));
                    }
                    for x in &by_district {
                        ui.horizontal(|ui| {
                            ui.add_sized([110.0, 18.0], egui::Label::new(js(x, "name")));
                            let (r, _) = ui.allocate_exact_size(vec2((ui.available_width() - 40.0).max(60.0), 14.0), Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(4), thumb_bg());
                            let w = r.width() * (ji(x, "n") as f32 / maxv);
                            ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(w.max(4.0), r.height())), CornerRadius::same(4), accent());
                            ui.label(ji(x, "n").to_string());
                        });
                    }
                });
                let trend: Vec<Value> = d["trend"].as_array().cloned().unwrap_or_default();
                panel_box(&mut cols[1], "Son 6 ay: eklenen ve sonuçlanan (satış/kiralama)", "", |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 170.0), Sense::hover());
                    let p = ui.painter();
                    let maxv = trend.iter().map(|t| ji(t, "created").max(ji(t, "closed"))).max().unwrap_or(1).max(1) as f32;
                    let n = trend.len().max(1) as f32;
                    let slot = rect.width() / n;
                    for (i, t) in trend.iter().enumerate() {
                        let x0 = rect.min.x + slot * i as f32 + slot * 0.18;
                        let bw = slot * 0.28;
                        for (j, (k, col)) in [("created", accent()), ("closed", chart(1))].iter().enumerate() {
                            let v = ji(t, k) as f32;
                            let h = (rect.height() - 40.0) * v / maxv;
                            let r = egui::Rect::from_min_max(egui::pos2(x0 + bw * j as f32 + 2.0 * j as f32, rect.max.y - 22.0 - h), egui::pos2(x0 + bw * (j + 1) as f32 + 2.0 * j as f32, rect.max.y - 22.0));
                            p.rect_filled(r, CornerRadius::same(3), *col);
                            if v > 0.0 {
                                p.text(egui::pos2(r.center().x, r.min.y - 2.0), Align2::CENTER_BOTTOM, format!("{}", v as i64), FontId::proportional(11.0), t_mid());
                            }
                        }
                        let m: u32 = js(t, "month").get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
                        p.text(egui::pos2(rect.min.x + slot * (i as f32 + 0.5), rect.max.y - 4.0), Align2::CENTER_BOTTOM, MONTHS_TR[(m - 1) as usize], FontId::proportional(11.5), muted());
                    }
                    ui.horizontal(|ui| {
                        ui.colored_label(accent(), ti("■ Eklenen"));
                        ui.colored_label(chart(1), "■ Sonuçlanan");
                    });
                });
            });
        });
    }

    // ------------------------------------------------------------ ALICILAR
    pub fn ui_buyers(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            page_title(ui, "Alıcılar / Müşteri adayları");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary(ui, ti("+ Yeni alıcı")) {
                    self.ext.buyer_form = Some(BuyerForm::blank());
                    self.ext.buyer_sel = 0;
                    self.ext.buyer_matches.clear();
                }
                ui.add(egui::TextEdit::singleline(&mut self.ext.buyer_q).hint_text(ti("Ara: ad, telefon, bölge…")).desired_width(220.0));
            });
        });
        ui.label(
            RichText::new(if self.me.is_lead() { "Tüm alıcıları görüyorsunuz." } else { "Yalnızca sizin eklediğiniz alıcılar görünür." })
                .small()
                .color(muted()),
        );
        ui.add_space(4.0);
        let q = self.ext.buyer_q.to_lowercase();
        let list: Vec<Value> = self
            .ext
            .buyers
            .iter()
            .filter(|b| q.is_empty() || format!("{} {} {} {} {}", js(b, "full_name"), js(b, "phone"), js(b, "district"), js(b, "neighborhood"), js(b, "category")).to_lowercase().contains(&q))
            .cloned()
            .collect();
        let mut select: Option<Value> = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(370.0, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("buyer_list").auto_shrink([false, false]).show(ui, |ui| {
                    if list.is_empty() {
                        ui.label(RichText::new(ti("Kayıtlı alıcı yok.")).color(muted()));
                    }
                    for b in &list {
                        let sel = ji(b, "id") == self.ext.buyer_sel;
                        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width() - 6.0, 64.0), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(rect, CornerRadius::same(8), if sel { nav_sel_soft() } else if resp.hovered() { card_hover() } else { card() });
                        p.rect_stroke(rect, CornerRadius::same(8), Stroke::new(if sel { 1.5_f32 } else { 1.0_f32 }, if sel { accent() } else { stroke() }), egui::StrokeKind::Inside);
                        p.text(rect.min + vec2(12.0, 8.0), Align2::LEFT_TOP, js(b, "full_name"), FontId::proportional(15.0), t_strong());
                        let budget = {
                            let (mn, mx) = (jf(b, "budget_min"), jf(b, "budget_max"));
                            match (mn > 0.0, mx > 0.0) {
                                (true, true) => format!("{} – {} {}", num_tr(mn), num_tr(mx), js(b, "currency")),
                                (false, true) => format!("en çok {} {}", num_tr(mx), js(b, "currency")),
                                (true, false) => format!("en az {} {}", num_tr(mn), js(b, "currency")),
                                _ => "bütçe belirtilmemiş".into(),
                            }
                        };
                        let loc = [js(b, "district"), js(b, "neighborhood")].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" / ");
                        p.text(
                            rect.min + vec2(12.0, 30.0),
                            Align2::LEFT_TOP,
                            format!("{} {} • {}", js(b, "listing_type"), js(b, "category"), if loc.is_empty() { "tüm bölgeler".into() } else { loc }),
                            FontId::proportional(12.5),
                            t_mid(),
                        );
                        p.text(rect.min + vec2(12.0, 46.0), Align2::LEFT_TOP, budget, FontId::proportional(12.0), muted());
                        let mc = ji(b, "match_count");
                        if mc > 0 {
                            let r = egui::Rect::from_min_size(egui::pos2(rect.max.x - 92.0, rect.min.y + 8.0), vec2(82.0, 20.0));
                            p.rect_filled(r, CornerRadius::same(10), chart(1));
                            p.text(r.center(), Align2::CENTER_CENTER, format!("{mc} eşleşme"), FontId::proportional(12.0), Color32::WHITE);
                        }
                        if js(b, "status") != "Aktif" {
                            p.text(egui::pos2(rect.max.x - 12.0, rect.max.y - 18.0), Align2::RIGHT_TOP, js(b, "status"), FontId::proportional(11.5), chart(3));
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            select = Some(b.clone());
                        }
                        ui.add_space(4.0);
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().id_salt("buyer_form").auto_shrink([false, false]).show(ui, |ui| self.buyer_form_ui(ui));
            });
        });
        if let Some(b) = select {
            let id = ji(&b, "id");
            self.ext.buyer_form = Some(BuyerForm::from_value(&b));
            self.ext.buyer_sel = id;
            self.load_buyer_matches(id);
        }
    }

    fn buyer_form_ui(&mut self, ui: &mut Ui) {
        let Some(mut f) = self.ext.buyer_form.clone() else {
            ui.add_space(30.0);
            ui.label(RichText::new(ti("Soldan bir alıcı seçin ya da yeni alıcı ekleyin.")).size(16.0).color(muted()));
            ui.label(RichText::new(ti("Alıcının kriterlerine uyan yayındaki portföyler otomatik eşleşir; yeni portföy eklenince size bildirim gelir.")).color(muted()));
            return;
        };
        let districts: Vec<String> = self.locations.iter().map(|d| d.name.clone()).collect();
        let neighs: Vec<String> = self.locations.iter().find(|d| d.name == f.district).map(|d| d.neighborhoods.iter().map(|n| n.name.clone()).collect()).unwrap_or_default();
        panel_box(ui, if f.id == 0 { "Yeni alıcı" } else { "Alıcı bilgileri" }, "", |ui| {
            egui::Grid::new("buyer_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(ti("Ad soyad *"));
                ui.add(egui::TextEdit::singleline(&mut f.full_name).desired_width(300.0));
                ui.end_row();
                ui.label(ti("Telefon"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut f.phone).desired_width(160.0));
                    ui.label(ti("E-posta"));
                    ui.add(egui::TextEdit::singleline(&mut f.email).desired_width(200.0));
                });
                ui.end_row();
                ui.label(ti("Aradığı"));
                ui.horizontal(|ui| {
                    for t in LISTING_TYPES {
                        ui.radio_value(&mut f.listing_type, t.to_string(), t);
                    }
                    combo_opt(ui, "b_cat", "Her tür", &mut f.category, &strs(&CATEGORIES), 150.0);
                    ui.label(ti("Oda"));
                    combo_opt(ui, "b_room", "Fark etmez", &mut f.rooms, &strs(&ROOMS), 100.0);
                });
                ui.end_row();
                ui.label(ti("Bölge"));
                ui.horizontal(|ui| {
                    if combo_opt(ui, "b_dist", "Tüm ilçeler", &mut f.district, &districts, 160.0) {
                        f.neighborhood.clear();
                    }
                    combo_opt(ui, "b_neigh", "Tüm mahalleler", &mut f.neighborhood, &neighs, 200.0);
                });
                ui.end_row();
                ui.label(ti("Bütçe"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut f.min_s).hint_text(ti("en az")).desired_width(120.0));
                    ui.label(ti("–"));
                    ui.add(egui::TextEdit::singleline(&mut f.max_s).hint_text(ti("en çok")).desired_width(120.0));
                    combo_pick(ui, "b_cur", &mut f.currency, &strs(&CURRENCIES), 70.0);
                });
                ui.end_row();
                ui.label(ti("Durum"));
                ui.horizontal(|ui| {
                    for s in ["Aktif", "Görüşülüyor", "Pasif"] {
                        ui.radio_value(&mut f.status, s.to_string(), s);
                    }
                });
                ui.end_row();
                ui.label(ti("Not"));
                ui.add(egui::TextEdit::multiline(&mut f.note).desired_rows(3).desired_width(420.0));
                ui.end_row();
                ui.label(ti("KVKK"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.consent, ti("Kişisel verilerin işlenmesine açık rıza alındı"));
                    if f.consent && !f.consent_date.is_empty() {
                        ui.label(RichText::new(format!("({})", date_tr(&f.consent_date))).small().color(muted()));
                    }
                });
                ui.end_row();
            });
            ui.add_space(6.0);
            let mut act: u8 = 0;
            ui.horizontal(|ui| {
                if primary(ui, ti("Kaydet")) {
                    act = 1;
                }
                if f.id > 0 {
                    if ui.button(ti("Randevu ekle")).clicked() {
                        act = 2;
                    }
                    if ui.button(ti("WhatsApp")).clicked() {
                        act = 3;
                    }
                    if ui.button(ti("Verilerini dışa aktar (KVKK)")).clicked() {
                        act = 4;
                    }
                    if self.me.is_admin() && confirm_btn(ui, "buyer_anon", "Anonimleştir") {
                        act = 5;
                    }
                    if confirm_btn(ui, "buyer_del", "Sil") {
                        act = 6;
                    }
                }
            });
            match act {
                1 => {
                    if f.full_name.trim().is_empty() {
                        self.err(ti("Alıcı adı boş olamaz."));
                    } else {
                        let body = f.to_json();
                        let id = f.id;
                        self.call(
                            move |api| if id == 0 { api.post("/api/buyers", &body) } else { api.put(&format!("/api/buyers/{id}"), &body) },
                            move |app, v| {
                                app.toast(ti("Alıcı kaydedildi."));
                                if id == 0 {
                                    let n = ji(&v, "matches");
                                    if n > 0 {
                                        app.toast(format!("Alıcı kaydedildi — {n} portföy eşleşiyor."));
                                    }
                                    app.ext.buyer_form = None;
                                } else {
                                    app.load_buyer_matches(id);
                                }
                                app.load_buyers();
                            },
                        );
                    }
                }
                2 => {
                    let mut af = ApptForm::blank(chrono::Local::now().date_naive() + CDur::days(1), self.me.id);
                    af.buyer_id = f.id;
                    af.title = format!("{} ile görüşme", f.full_name);
                    self.ext.appt_form = Some(af);
                    self.ext.cal_sel = chrono::Local::now().date_naive() + CDur::days(1);
                    self.goto(View::Calendar);
                }
                3 => {
                    let wa = wa_number(&f.phone);
                    if wa.is_empty() {
                        self.err(ti("Alıcının telefon numarası yok."));
                    } else {
                        let list: Vec<String> = self
                            .ext
                            .buyer_matches
                            .iter()
                            .take(8)
                            .map(|m| format!("• P-{:05} {} — {}", ji(m, "id"), js(m, "title"), fmt_price(jf(m, "price"), &js(m, "currency"))))
                            .collect();
                        let text = if list.is_empty() {
                            format!("Merhaba {}, {}", f.full_name, self.settings.office_name)
                        } else {
                            format!("Merhaba {}, aradığınız kriterlere uygun portföylerimiz:\n{}\n\n{}", f.full_name, list.join("\n"), self.settings.office_name)
                        };
                        self.open_url(&format!("https://wa.me/{}?text={}", wa, url_encode(&text)));
                    }
                }
                4 => {
                    let id = f.id;
                    self.call(move |api| api.get::<Value>(&format!("/api/buyers/{id}/export")), move |app, v| {
                        let txt = serde_json::to_string_pretty(&v).unwrap_or_default();
                        app.save_dialog(&format!("alici_{id}.json"), "json", move |app, p| {
                            if let Some(p) = p {
                                match std::fs::write(&p, txt) {
                                    Ok(_) => app.toast(format!("Kaydedildi: {}", p.display())),
                                    Err(e) => app.err(e.to_string()),
                                }
                            }
                        });
                    });
                }
                5 => {
                    let id = f.id;
                    self.call(move |api| api.post(&format!("/api/buyers/{id}/anonymize"), &json!({})), |app, _| {
                        app.toast(ti("Alıcı anonimleştirildi."));
                        app.ext.buyer_form = None;
                        app.load_buyers();
                    });
                }
                6 => {
                    let id = f.id;
                    self.call(move |api| api.delete(&format!("/api/buyers/{id}")), |app, _| {
                        app.toast(ti("Alıcı silindi."));
                        app.ext.buyer_form = None;
                        app.ext.buyer_sel = 0;
                        app.load_buyers();
                    });
                }
                _ => {}
            }
        });
        if f.id > 0 {
            let matches = self.ext.buyer_matches.clone();
            panel_box(ui, &format!("Eşleşen yayındaki portföyler ({})", matches.len()), "Tür, bölge, bütçe ve oda sayısına göre.", |ui| {
                if matches.is_empty() {
                    ui.label(RichText::new(ti("Şu an eşleşen portföy yok. Yeni uygun portföy eklenince bildirim gelir.")).color(muted()));
                }
                for m in &matches {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("P-{:05}", ji(m, "id"))).color(muted()));
                        ui.label(RichText::new(js(m, "title")).strong());
                        ui.label(format!("{} / {} • {}", js(m, "district"), js(m, "neighborhood"), fmt_price(jf(m, "price"), &js(m, "currency"))));
                        if ui.small_button(ti("Aç")).clicked() {
                            self.open_detail(ji(m, "id"));
                        }
                    });
                }
            });
        }
        self.ext.buyer_form = Some(f);
    }

    // ------------------------------------------------------------ TAKVİM
    pub fn ui_calendar(&mut self, ui: &mut Ui) {
        page_title(ui, "Takvim ve randevular");
        let mut reload = false;
        ui.horizontal(|ui| {
            let mut d = self.ext.cal_month;
            if month_nav(ui, &mut d) {
                self.ext.cal_month = d;
                self.ext.cal_sel = d;
                reload = true;
            }
            ui.separator();
            let names: Vec<String> = self.users.iter().filter(|u| u.active).map(|u| u.full_name()).collect();
            let mut sel = self.users.iter().find(|u| u.id == self.ext.cal_user).map(|u| u.full_name()).unwrap_or_default();
            if combo_opt(ui, "cal_user", "Tüm ofis", &mut sel, &names, 170.0) {
                self.ext.cal_user = self.users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(0);
            }
            if primary(ui, ti("+ Randevu")) {
                self.ext.appt_form = Some(ApptForm::blank(self.ext.cal_sel, self.me.id));
            }
        });
        if reload {
            self.load_appts();
        }
        ui.add_space(4.0);
        let total_w = ui.available_width();
        let grid_w = (total_w * 0.66).max(520.0).min(total_w - 300.0).max(420.0);
        let appts: Vec<Value> = self.ext.appts.iter().filter(|a| self.ext.cal_user == 0 || ji(a, "user_id") == self.ext.cal_user).cloned().collect();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(grid_w, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                self.calendar_grid(ui, &appts);
            });
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().id_salt("cal_day").auto_shrink([false, false]).show(ui, |ui| self.calendar_day(ui, &appts));
            });
        });
    }

    fn calendar_grid(&mut self, ui: &mut Ui, appts: &[Value]) {
        let first = self.ext.cal_month;
        let offset = first.weekday().num_days_from_monday() as i64;
        let start = first - CDur::days(offset);
        let w = ui.available_width();
        let cw = w / 7.0;
        let today = chrono::Local::now().date_naive();
        let (hr, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        for (i, d) in DAYS_TR.iter().enumerate() {
            ui.painter().text(egui::pos2(hr.min.x + cw * (i as f32 + 0.5), hr.center().y), Align2::CENTER_CENTER, *d, FontId::proportional(12.5), muted());
        }
        let rows = ((offset + last_day(first) as i64 + 6) / 7) as i32;
        let ch = ((ui.available_height() - 8.0) / rows as f32).clamp(78.0, 120.0);
        let mut clicked: Option<NaiveDate> = None;
        for r in 0..rows {
            let (_, row_rect) = ui.allocate_space(vec2(w, ch));
            for c in 0..7 {
                let d = start + CDur::days((r * 7 + c) as i64);
                let rect = egui::Rect::from_min_size(egui::pos2(hr.min.x + cw * c as f32, row_rect.min.y), vec2(cw - 3.0, ch - 3.0));
                let in_month = d.month() == first.month();
                let sel = d == self.ext.cal_sel;
                let resp = ui.interact(rect, ui.id().with(("calcell", d.num_days_from_ce())), Sense::click());
                let p = ui.painter();
                let bg = if sel { nav_sel_soft() } else if resp.hovered() { card_hover() } else if in_month { card() } else { thumb_bg() };
                p.rect_filled(rect, CornerRadius::same(6), bg);
                p.rect_stroke(rect, CornerRadius::same(6), Stroke::new(if d == today { 2.0_f32 } else { 1.0_f32 }, if d == today { accent() } else { stroke() }), egui::StrokeKind::Inside);
                p.text(rect.min + vec2(6.0, 4.0), Align2::LEFT_TOP, d.day().to_string(), FontId::proportional(13.0), if in_month { t_strong() } else { t_dim() });
                let key = d.format("%Y-%m-%d").to_string();
                let day: Vec<&Value> = appts.iter().filter(|a| js(a, "start_at").starts_with(&key) && js(a, "status") != "İptal").collect();
                for (i, a) in day.iter().take(3).enumerate() {
                    let col = if js(a, "status") == "Tamamlandı" { t_dim() } else { accent() };
                    let line = format!("{} {}", js(a, "start_at").get(11..16).unwrap_or(""), js(a, "title"));
                    let galley = p.layout(line, FontId::proportional(11.0), col, f32::INFINITY);
                    p.with_clip_rect(rect.shrink(2.0)).galley(rect.min + vec2(6.0, 22.0 + i as f32 * 14.0), galley, col);
                }
                if day.len() > 3 {
                    p.text(rect.min + vec2(6.0, 22.0 + 3.0 * 14.0), Align2::LEFT_TOP, format!("+{} daha", day.len() - 3), FontId::proportional(10.5), muted());
                }
                if resp.clicked() {
                    clicked = Some(d);
                }
            }
        }
        if let Some(d) = clicked {
            self.ext.cal_sel = d;
            if d.month() != first.month() {
                self.ext.cal_month = d.with_day(1).unwrap_or(d);
                self.load_appts();
            }
        }
    }

    fn calendar_day(&mut self, ui: &mut Ui, appts: &[Value]) {
        let day = self.ext.cal_sel;
        ui.label(RichText::new(format!("{} {} {}", day.day(), MONTHS_TR[(day.month() - 1) as usize], DAYS_TR[day.weekday().num_days_from_monday() as usize])).size(18.0).strong().color(accent()));
        if let Some(f) = self.ext.appt_form.clone() {
            self.appt_form_ui(ui, f);
            return;
        }
        let key = day.format("%Y-%m-%d").to_string();
        let list: Vec<&Value> = appts.iter().filter(|a| js(a, "start_at").starts_with(&key)).collect();
        if list.is_empty() {
            ui.label(RichText::new(ti("Bu gün için randevu yok.")).color(muted()));
        }
        let mut edit: Option<Value> = None;
        let mut status: Option<(i64, &'static str)> = None;
        let mut del: Option<i64> = None;
        for a in list {
            let id = ji(a, "id");
            let can = self.me.is_lead() || ji(a, "user_id") == self.me.id;
            card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let st = js(a, "start_at");
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}  ({} dk)", st.get(11..16).unwrap_or(""), ji(a, "duration_min"))).strong().size(15.0));
                    badge(ui, &js(a, "status"), status_badge_color(&js(a, "status")));
                });
                ui.label(RichText::new(js(a, "title")).strong());
                ui.label(RichText::new(format!("{} • {}", js(a, "kind"), js(a, "user_name"))).small().color(muted()));
                if !js(a, "portfolio_title").is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Portföy: {}", js(a, "portfolio_title"))).small());
                        if ui.small_button(ti("Aç")).clicked() {
                            self.open_detail(ji(a, "portfolio_id"));
                        }
                    });
                }
                if !js(a, "buyer_name").is_empty() {
                    ui.label(RichText::new(format!("Alıcı: {}", js(a, "buyer_name"))).small());
                }
                if !js(a, "note").is_empty() {
                    ui.label(RichText::new(js(a, "note")).small().color(t_mid()));
                }
                if can {
                    ui.horizontal(|ui| {
                        if js(a, "status") == "Planlı" && ui.small_button(ti("✓ Tamamla")).clicked() {
                            status = Some((id, "Tamamlandı"));
                        }
                        if js(a, "status") == "Planlı" && ui.small_button(ti("İptal et")).clicked() {
                            status = Some((id, "İptal"));
                        }
                        if ui.small_button(ti("Düzenle")).clicked() {
                            edit = Some(a.clone());
                        }
                        if confirm_btn(ui, &format!("appt{id}"), "Sil") {
                            del = Some(id);
                        }
                    });
                }
            });
            ui.add_space(4.0);
        }
        if let Some(a) = edit {
            self.ext.appt_form = Some(ApptForm::from_value(&a));
        }
        if let Some((id, st)) = status {
            if let Some(a) = self.ext.appts.iter().find(|a| ji(a, "id") == id).cloned() {
                let mut f = ApptForm::from_value(&a);
                f.status = st.to_string();
                f.force = true;
                let body = f.to_json();
                self.call(move |api| api.put(&format!("/api/appointments/{id}"), &body), |app, _| app.load_appts());
            }
        }
        if let Some(id) = del {
            self.call(move |api| api.delete(&format!("/api/appointments/{id}")), |app, _| app.load_appts());
        }
    }

    fn appt_form_ui(&mut self, ui: &mut Ui, mut f: ApptForm) {
        let users: Vec<UserInfo> = self.users.iter().filter(|u| u.active).cloned().collect();
        let pf: Vec<(i64, String)> = self.portfolios.iter().take(300).map(|p| (p.id, format!("P-{:05} {}", p.id, p.title))).collect();
        let bf: Vec<(i64, String)> = self.ext.buyers.iter().map(|b| (ji(b, "id"), js(b, "full_name"))).collect();
        let mut act = 0;
        card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(if f.id == 0 { "Yeni randevu" } else { "Randevuyu düzenle" }).strong());
            ui.add(egui::TextEdit::singleline(&mut f.title).hint_text(ti("Başlık")).desired_width(f32::INFINITY));
            combo_pick(ui, "af_kind", &mut f.kind, &strs(&APPT_KINDS), 180.0);
            ui.horizontal(|ui| {
                ui.add(DatePickerButton::new(&mut f.date).id_salt("af_date"));
                ui.add(egui::DragValue::new(&mut f.hour).range(0..=23).suffix(" sa"));
                ui.add(egui::DragValue::new(&mut f.minute).range(0..=59).suffix(" dk"));
                ui.add(egui::DragValue::new(&mut f.duration).range(5..=600).suffix(" dk süre"));
            });
            if self.me.is_lead() {
                let names: Vec<String> = users.iter().map(|u| u.full_name()).collect();
                let mut sel = users.iter().find(|u| u.id == f.user_id).map(|u| u.full_name()).unwrap_or_default();
                if combo_pick(ui, "af_user", &mut sel, &names, 200.0) {
                    f.user_id = users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(f.user_id);
                }
            }
            let pnames: Vec<String> = pf.iter().map(|x| x.1.clone()).collect();
            let mut psel = pf.iter().find(|x| x.0 == f.portfolio_id).map(|x| x.1.clone()).unwrap_or_default();
            if combo_opt(ui, "af_pf", "Portföy (isteğe bağlı)", &mut psel, &pnames, 260.0) {
                f.portfolio_id = pf.iter().find(|x| x.1 == psel).map(|x| x.0).unwrap_or(0);
            }
            let bnames: Vec<String> = bf.iter().map(|x| x.1.clone()).collect();
            let mut bsel = bf.iter().find(|x| x.0 == f.buyer_id).map(|x| x.1.clone()).unwrap_or_default();
            if combo_opt(ui, "af_by", "Alıcı (isteğe bağlı)", &mut bsel, &bnames, 260.0) {
                f.buyer_id = bf.iter().find(|x| x.1 == bsel).map(|x| x.0).unwrap_or(0);
            }
            ui.add(egui::TextEdit::multiline(&mut f.note).hint_text(ti("Not")).desired_rows(2).desired_width(f32::INFINITY));
            if !f.warn.is_empty() {
                ui.colored_label(chart(4), &f.warn);
            }
            ui.horizontal(|ui| {
                if primary(ui, if f.force { "Yine de kaydet" } else { "Kaydet" }) {
                    act = 1;
                }
                if ui.button(ti("Kapat")).clicked() {
                    act = 2;
                }
            });
        });
        if act == 2 {
            self.ext.appt_form = None;
            return;
        }
        if act == 1 {
            if f.title.trim().is_empty() {
                self.err(ti("Randevu başlığı gerekli."));
            } else {
                let body = f.to_json();
                let id = f.id;
                self.bg(
                    move |api| if id == 0 { api.post("/api/appointments", &body) } else { api.put(&format!("/api/appointments/{id}"), &body) },
                    |app, r| match r {
                        Ok(_) => {
                            app.toast(ti("Randevu kaydedildi."));
                            app.ext.appt_form = None;
                            app.load_appts();
                        }
                        Err(e) => {
                            if e.contains("Çakışma") {
                                if let Some(f) = app.ext.appt_form.as_mut() {
                                    f.force = true;
                                    f.warn = e;
                                }
                            } else {
                                app.err(e);
                            }
                        }
                    },
                );
            }
        }
        self.ext.appt_form = Some(f);
    }

    // ------------------------------------------------------------ DUYURULAR
    pub fn ui_announcements(&mut self, ui: &mut Ui) {
        page_title(ui, "Duyurular");
        if self.me.is_lead() {
            if !self.ext.ann_form.show {
                if primary(ui, ti("+ Yeni duyuru")) {
                    self.ext.ann_form.show = true;
                }
            } else {
                let mut f = self.ext.ann_form.clone();
                let mut act = 0;
                panel_box(ui, ti("Yeni duyuru"), ti("Yayınlanınca tüm çalışanlara bildirim gider."), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut f.title).hint_text(ti("Başlık")).desired_width(520.0));
                    ui.add(egui::TextEdit::multiline(&mut f.body).hint_text(ti("Ayrıntı")).desired_rows(4).desired_width(520.0));
                    ui.checkbox(&mut f.pinned, ti("Başa sabitle"));
                    ui.horizontal(|ui| {
                        if primary(ui, ti("Yayınla")) {
                            act = 1;
                        }
                        if ui.button(ti("Vazgeç")).clicked() {
                            act = 2;
                        }
                    });
                });
                if act == 1 {
                    let body = json!({"title": f.title, "body": f.body, "pinned": f.pinned});
                    self.call(move |api| api.post("/api/announcements", &body), |app, _| {
                        app.toast(ti("Duyuru yayınlandı."));
                        app.ext.ann_form = AnnForm::default();
                        app.load_anns();
                    });
                } else if act == 2 {
                    f = AnnForm::default();
                }
                self.ext.ann_form = f;
            }
        }
        ui.add_space(6.0);
        let anns = self.ext.anns.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if anns.is_empty() {
                ui.label(RichText::new(ti("Duyuru yok.")).color(muted()));
            }
            for a in &anns {
                let id = ji(a, "id");
                let pinned = ji(a, "pinned") != 0;
                card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if pinned {
                            ui.label(ti("📌"));
                        }
                        ui.label(RichText::new(js(a, "title")).size(16.0).strong());
                    });
                    if !js(a, "body").is_empty() {
                        ui.label(js(a, "body"));
                    }
                    ui.label(RichText::new(format!("{} • {}", js(a, "author"), dt_tr(&js(a, "created_at")))).small().color(muted()));
                    if self.me.is_lead() {
                        ui.horizontal(|ui| {
                            if ui.small_button(if pinned { "Sabiti kaldır" } else { "Sabitle" }).clicked() {
                                let body = json!({"pinned": !pinned});
                                self.call(move |api| api.put(&format!("/api/announcements/{id}/pin"), &body), |app, _| app.load_anns());
                            }
                            if confirm_btn(ui, &format!("ann{id}"), "Sil") {
                                self.call(move |api| api.delete(&format!("/api/announcements/{id}")), |app, _| app.load_anns());
                            }
                        });
                    }
                });
                ui.add_space(6.0);
            }
        });
    }

    /// Karşılama ekranının üstündeki duyuru şeridi.
    pub fn ann_banner(&mut self, ui: &mut Ui) {
        if self.ext.ann_hidden {
            return;
        }
        let cutoff = (chrono::Local::now() - CDur::days(14)).format("%Y-%m-%d").to_string();
        let show: Vec<Value> = self.ext.anns.iter().filter(|a| ji(a, "pinned") != 0 || js(a, "created_at") >= cutoff).take(2).cloned().collect();
        if show.is_empty() {
            return;
        }
        let mut close = false;
        let mut open_all = false;
        egui::Frame::new().fill(note_bg()).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::symmetric(12, 8)).stroke(Stroke::new(1.0_f32, stroke())).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(ti("📢 Duyuru")).strong().color(accent()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(ti("✕")).on_hover_text(ti("Gizle")).clicked() {
                        close = true;
                    }
                    if ui.small_button(ti("Tümü")).clicked() {
                        open_all = true;
                    }
                });
            });
            for a in &show {
                ui.label(RichText::new(format!("{}{}", if ji(a, "pinned") != 0 { "📌 " } else { "" }, js(a, "title"))).strong());
                if !js(a, "body").is_empty() {
                    ui.label(RichText::new(js(a, "body")).small().color(t_mid()));
                }
            }
        });
        ui.add_space(6.0);
        if close {
            self.ext.ann_hidden = true;
        }
        if open_all {
            self.goto(View::Announcements);
        }
    }
}

fn last_day(first: NaiveDate) -> u32 {
    let next = add_months(first, 1);
    (next - CDur::days(1)).day()
}

pub fn nav_sel_soft() -> Color32 {
    let a = accent();
    if is_dark() {
        Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 60)
    } else {
        Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 28)
    }
}

pub fn dashboard_csv(d: &Value) -> String {
    let mut s = String::new();
    s.push_str(&format!("Rapor;{}\n\n", js(d, "month")));
    for (k, l) in [
        ("active", "Yayındaki portföy"), ("new_month", "Ay içinde eklenen"), ("published_month", "Ay içinde yayına alınan"),
        ("rented_month", "Ay içinde kiralanan"), ("sold_month", "Ay içinde satılan"), ("stale", "60+ gündür yayında"),
        ("buyers", "Aktif alıcı"), ("open_tasks", "Açık görev"),
    ] {
        s.push_str(&format!("{};{}\n", csv_cell(l), ji(d, k)));
    }
    s.push_str("\nÇalışan;Eklenen;Yayına alınan;Kiralanan;Satılan\n");
    if let Some(a) = d["by_user"].as_array() {
        for u in a {
            s.push_str(&format!("{};{};{};{};{}\n", csv_cell(&js(u, "user")), ji(u, "created"), ji(u, "published"), ji(u, "rented"), ji(u, "sold")));
        }
    }
    s.push_str("\nİlçe;Yayındaki portföy\n");
    if let Some(a) = d["by_district"].as_array() {
        for u in a {
            s.push_str(&format!("{};{}\n", csv_cell(&js(u, "name")), ji(u, "n")));
        }
    }
    s
}
