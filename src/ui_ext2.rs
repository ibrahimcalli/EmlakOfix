//! Sürüm 1.2 arayüzleri (2/2): izin & puantaj, komisyon, denetim, portföy ekleri, güncelleme ve yedek ayarları.
use crate::app::*;
use crate::common::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::ui_ext::*;
use crate::ui_portfolio::{combo_pick, strs};
use chrono::NaiveDate;
use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::DatePickerButton;
use serde_json::{json, Value};

impl App {
    // ---------------------------------------------------------- İZİN & PUANTAJ
    pub fn ui_leaves(&mut self, ui: &mut Ui) {
        page_title(ui, "İzin ve puantaj");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.ext.leave_tab, 0, ti("İzinler"));
            ui.selectable_value(&mut self.ext.leave_tab, 1, ti("Aylık puantaj"));
        });
        ui.add_space(6.0);
        if self.ext.leave_tab == 0 {
            self.leaves_tab(ui);
        } else {
            self.timesheet_tab(ui);
        }
    }

    fn leaves_tab(&mut self, ui: &mut Ui) {
        let today = chrono::Local::now().date_naive();
        ui.horizontal(|ui| {
            if ui.button(ti("◀")).clicked() {
                self.ext.leave_year -= 1;
                self.load_leaves();
            }
            ui.label(RichText::new(self.ext.leave_year.to_string()).strong().size(16.0));
            if ui.button(ti("▶")).clicked() {
                self.ext.leave_year += 1;
                self.load_leaves();
            }
            if primary(ui, ti("+ İzin talebi")) {
                self.ext.leave_form = Some(LeaveForm { kind: LEAVE_KINDS[0].into(), start: today, end: today, note: String::new(), user_id: self.me.id });
            }
        });
        if let Some(mut f) = self.ext.leave_form.clone() {
            let users: Vec<UserInfo> = self.users.iter().filter(|u| u.active).cloned().collect();
            let mut act = 0;
            panel_box(ui, "İzin talebi", if self.me.is_lead() { "Yönetici/ekip lideri adına girilen izin doğrudan onaylanır." } else { "Talebiniz yöneticiye bildirim olarak gider." }, |ui| {
                combo_pick(ui, "lv_kind", &mut f.kind, &strs(&LEAVE_KINDS), 180.0);
                ui.horizontal(|ui| {
                    ui.add(DatePickerButton::new(&mut f.start).id_salt("lv_s"));
                    ui.label(ti("→"));
                    ui.add(DatePickerButton::new(&mut f.end).id_salt("lv_e"));
                });
                if self.me.is_lead() {
                    let names: Vec<String> = users.iter().map(|u| u.full_name()).collect();
                    let mut sel = users.iter().find(|u| u.id == f.user_id).map(|u| u.full_name()).unwrap_or_default();
                    if combo_pick(ui, "lv_user", &mut sel, &names, 200.0) {
                        f.user_id = users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(f.user_id);
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut f.note).hint_text(ti("Not")).desired_width(360.0));
                ui.horizontal(|ui| {
                    if primary(ui, ti("Gönder")) {
                        act = 1;
                    }
                    if ui.button(ti("Vazgeç")).clicked() {
                        act = 2;
                    }
                });
            });
            if act == 1 {
                let body = json!({"kind": f.kind, "start_date": f.start.format("%Y-%m-%d").to_string(), "end_date": f.end.format("%Y-%m-%d").to_string(), "note": f.note, "user_id": f.user_id});
                self.call(move |api| api.post("/api/leaves", &body), |app, _| {
                    app.toast(ti("İzin kaydedildi."));
                    app.ext.leave_form = None;
                    app.load_leaves();
                });
            } else if act == 2 {
                self.ext.leave_form = None;
            } else {
                self.ext.leave_form = Some(f);
            }
        }
        let list = self.ext.leaves.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if list.is_empty() {
                ui.label(RichText::new(ti("Bu yıl için izin kaydı yok.")).color(muted()));
            }
            for l in &list {
                let id = ji(l, "id");
                let st = js(l, "status");
                card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(js(l, "user_name")).strong());
                        ui.label(format!("{} • {} → {}", js(l, "kind"), date_tr(&js(l, "start_date")), date_tr(&js(l, "end_date"))));
                        let col = match st.as_str() {
                            "Onaylandı" => Color32::from_rgb(34, 139, 70),
                            "Reddedildi" => Color32::from_rgb(190, 55, 55),
                            _ => Color32::from_rgb(200, 130, 20),
                        };
                        badge(ui, &st, col);
                        if self.me.is_lead() && st == "Bekliyor" {
                            if ui.small_button(ti("Onayla")).clicked() {
                                self.call(move |api| api.put(&format!("/api/leaves/{id}/status"), &json!({"status": "Onaylandı"})), |app, _| app.load_leaves());
                            }
                            if ui.small_button(ti("Reddet")).clicked() {
                                self.call(move |api| api.put(&format!("/api/leaves/{id}/status"), &json!({"status": "Reddedildi"})), |app, _| app.load_leaves());
                            }
                        }
                        if (self.me.is_lead() || (ji(l, "user_id") == self.me.id && st == "Bekliyor")) && confirm_btn(ui, &format!("lv{id}"), "Sil") {
                            self.call(move |api| api.delete(&format!("/api/leaves/{id}")), |app, _| app.load_leaves());
                        }
                    });
                    if !js(l, "note").is_empty() {
                        ui.label(RichText::new(js(l, "note")).small().color(muted()));
                    }
                });
                ui.add_space(4.0);
            }
        });
    }

    fn timesheet_tab(&mut self, ui: &mut Ui) {
        let mut d = self.ext.ts_month;
        if ui.horizontal(|ui| month_nav(ui, &mut d)).inner {
            self.ext.ts_month = d;
            self.load_leaves();
        }
        let Some(ts) = self.ext.ts.clone() else {
            ui.label(RichText::new(ti("Yükleniyor…")).color(muted()));
            return;
        };
        let rows = ts["rows"].as_array().cloned().unwrap_or_default();
        ui.label(RichText::new(format!("Mesai başlangıcı: {} (bu saatten sonraki girişler 'geç' sayılır). Gelecek günler hesaba katılmaz.", js(&ts, "work_start"))).small().color(muted()));
        if ui.button(ti("CSV olarak kaydet")).clicked() {
            let mut s = String::from("Çalışan;Çalışılan gün;Saat;Geç kalma;İzin günü;Hastalık günü;Beklenen gün;Devamsız\n");
            for r in &rows {
                s.push_str(&format!("{};{};{};{};{};{};{};{}\n", csv_cell(&js(r, "user")), ji(r, "days"), jf(r, "hours"), ji(r, "late"), ji(r, "leave_days"), ji(r, "sick_days"), ji(r, "expected"), ji(r, "absent")));
            }
            let name = format!("puantaj_{}.csv", self.ext.ts_month.format("%Y-%m"));
            self.save_csv(&name, s);
        }
        egui::Grid::new("ts").striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
            for h in ["Çalışan", "Gün", "Saat", "Geç", "İzin", "Hastalık", "Beklenen", "Devamsız"] {
                ui.label(RichText::new(h).strong().color(muted()));
            }
            ui.end_row();
            for r in &rows {
                ui.label(RichText::new(js(r, "user")).strong());
                ui.label(ji(r, "days").to_string());
                ui.label(format!("{:.1}", jf(r, "hours")));
                ui.label(ji(r, "late").to_string());
                ui.label(ji(r, "leave_days").to_string());
                ui.label(ji(r, "sick_days").to_string());
                ui.label(ji(r, "expected").to_string());
                let a = ji(r, "absent");
                ui.label(RichText::new(a.to_string()).color(if a > 0 { Color32::from_rgb(190, 55, 55) } else { t_mid() }));
                ui.end_row();
            }
        });
    }

    // ---------------------------------------------------------- KOMİSYON
    pub fn ui_commissions(&mut self, ui: &mut Ui) {
        page_title(ui, "Komisyon takibi");
        let mut reload = false;
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.ext.comm_all, ti("Tüm zamanlar")).changed() {
                reload = true;
            }
            if !self.ext.comm_all {
                let mut d = self.ext.comm_month;
                if month_nav(ui, &mut d) {
                    self.ext.comm_month = d;
                    reload = true;
                }
            }
            if primary(ui, ti("+ Komisyon kaydı")) {
                self.ext.comm_form = Some(CommForm {
                    id: 0, portfolio_id: 0, user_id: self.me.id, deal_type: "Satış".into(), deal_s: String::new(), currency: "TL".into(),
                    rate_s: "2".into(), comm_s: String::new(), pct_s: "50".into(), status: "Bekliyor".into(), date: chrono::Local::now().date_naive(), note: String::new(),
                });
            }
            if ui.button(ti("CSV")).clicked() {
                let mut s = String::from("Tarih;Portföy;Çalışan;İşlem;Tutar;Para birimi;Oran %;Komisyon;Çalışan payı;Durum\n");
                for c in &self.ext.comms {
                    s.push_str(&format!("{};{};{};{};{};{};{};{};{};{}\n", js(c, "date"), csv_cell(&js(c, "portfolio_title")), csv_cell(&js(c, "user_name")), js(c, "deal_type"), jf(c, "deal_amount"), js(c, "currency"), jf(c, "rate"), jf(c, "commission"), jf(c, "emp_amount"), js(c, "status")));
                }
                self.save_csv("komisyonlar.csv", s);
            }
        });
        if reload {
            self.load_comms();
        }
        if let Some(mut f) = self.ext.comm_form.clone() {
            let users: Vec<UserInfo> = self.users.iter().filter(|u| u.active).cloned().collect();
            let pf: Vec<(i64, String)> = self.portfolios.iter().take(300).map(|p| (p.id, format!("P-{:05} {}", p.id, p.title))).collect();
            let mut act = 0;
            panel_box(ui, if f.id == 0 { "Yeni komisyon kaydı" } else { "Kaydı düzenle" }, "Komisyon = işlem tutarı × oran; boş bırakırsanız otomatik hesaplanır.", |ui| {
                let pn: Vec<String> = pf.iter().map(|x| x.1.clone()).collect();
                let mut ps = pf.iter().find(|x| x.0 == f.portfolio_id).map(|x| x.1.clone()).unwrap_or_default();
                if crate::ui_portfolio::combo_opt(ui, "cm_pf", "Portföy (isteğe bağlı)", &mut ps, &pn, 280.0) {
                    f.portfolio_id = pf.iter().find(|x| x.1 == ps).map(|x| x.0).unwrap_or(0);
                }
                let names: Vec<String> = users.iter().map(|u| u.full_name()).collect();
                let mut sel = users.iter().find(|u| u.id == f.user_id).map(|u| u.full_name()).unwrap_or_default();
                if combo_pick(ui, "cm_user", &mut sel, &names, 220.0) {
                    f.user_id = users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(f.user_id);
                }
                ui.horizontal(|ui| {
                    for t in ["Satış", "Kiralama"] {
                        ui.radio_value(&mut f.deal_type, t.to_string(), t);
                    }
                    ui.add(egui::TextEdit::singleline(&mut f.deal_s).hint_text(ti("işlem tutarı")).desired_width(130.0));
                    combo_pick(ui, "cm_cur", &mut f.currency, &strs(&CURRENCIES), 70.0);
                });
                ui.horizontal(|ui| {
                    ui.label(ti("Oran %"));
                    ui.add(egui::TextEdit::singleline(&mut f.rate_s).desired_width(50.0));
                    ui.label(ti("veya komisyon"));
                    ui.add(egui::TextEdit::singleline(&mut f.comm_s).desired_width(100.0));
                    ui.label(ti("Çalışan payı %"));
                    ui.add(egui::TextEdit::singleline(&mut f.pct_s).desired_width(50.0));
                });
                ui.horizontal(|ui| {
                    for s in ["Bekliyor", "Ödendi", "İptal"] {
                        ui.radio_value(&mut f.status, s.to_string(), s);
                    }
                    ui.add(DatePickerButton::new(&mut f.date).id_salt("cm_date"));
                });
                ui.add(egui::TextEdit::singleline(&mut f.note).hint_text(ti("Not")).desired_width(360.0));
                ui.horizontal(|ui| {
                    if primary(ui, ti("Kaydet")) {
                        act = 1;
                    }
                    if ui.button(ti("Vazgeç")).clicked() {
                        act = 2;
                    }
                });
            });
            if act == 1 {
                let body = json!({
                    "portfolio_id": f.portfolio_id, "user_id": f.user_id, "deal_type": f.deal_type, "deal_amount": parse_num(&f.deal_s, true),
                    "currency": f.currency, "rate": parse_num(&f.rate_s, false), "commission": parse_num(&f.comm_s, true),
                    "emp_pct": parse_num(&f.pct_s, false), "status": f.status, "date": f.date.format("%Y-%m-%d").to_string(), "note": f.note,
                });
                let id = f.id;
                self.call(move |api| if id == 0 { api.post("/api/commissions", &body) } else { api.put(&format!("/api/commissions/{id}"), &body) }, |app, _| {
                    app.toast(ti("Kaydedildi."));
                    app.ext.comm_form = None;
                    app.load_comms();
                });
            } else if act == 2 {
                self.ext.comm_form = None;
            } else {
                self.ext.comm_form = Some(f);
            }
        }
        let sum = self.ext.comm_sum.clone();
        if !sum.is_empty() {
            panel_box(ui, ti("Çalışan bazında özet"), "", |ui| {
                egui::Grid::new("cm_sum").striped(true).spacing([18.0, 4.0]).show(ui, |ui| {
                    for h in ["Çalışan", "Toplam komisyon", "Çalışan payı"] {
                        ui.label(RichText::new(h).strong().color(muted()));
                    }
                    ui.end_row();
                    for s in &sum {
                        ui.label(js(s, "user_name"));
                        ui.label(format!("{} {}", num_tr(jf(s, "total")), js(s, "currency")));
                        ui.label(format!("{} {}", num_tr(jf(s, "emp_total")), js(s, "currency")));
                        ui.end_row();
                    }
                });
            });
        }
        let list = self.ext.comms.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for c in &list {
                let id = ji(c, "id");
                card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(date_tr(&js(c, "date"))).color(muted()));
                        ui.label(RichText::new(js(c, "user_name")).strong());
                        ui.label(format!("{} • {} {} → komisyon {} {} (çalışan {} )", js(c, "deal_type"), num_tr(jf(c, "deal_amount")), js(c, "currency"), num_tr(jf(c, "commission")), js(c, "currency"), num_tr(jf(c, "emp_amount"))));
                        badge(ui, &js(c, "status"), if js(c, "status") == "Ödendi" { Color32::from_rgb(34, 139, 70) } else if js(c, "status") == "İptal" { Color32::from_rgb(190, 55, 55) } else { Color32::from_rgb(200, 130, 20) });
                    });
                    if !js(c, "portfolio_title").is_empty() {
                        ui.label(RichText::new(format!("Portföy: {}", js(c, "portfolio_title"))).small().color(muted()));
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button(ti("Düzenle")).clicked() {
                            self.ext.comm_form = Some(CommForm {
                                id, portfolio_id: ji(c, "portfolio_id"), user_id: ji(c, "user_id"), deal_type: js(c, "deal_type"),
                                deal_s: num_to_text(jf(c, "deal_amount")), currency: js(c, "currency"), rate_s: num_to_text(jf(c, "rate")),
                                comm_s: String::new(), pct_s: num_to_text(jf(c, "emp_pct")), status: js(c, "status"),
                                date: NaiveDate::parse_from_str(&js(c, "date"), "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive()), note: js(c, "note"),
                            });
                        }
                        if confirm_btn(ui, &format!("cm{id}"), "Sil") {
                            self.call(move |api| api.delete(&format!("/api/commissions/{id}")), |app, _| app.load_comms());
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });
    }

    // ---------------------------------------------------------- DENETİM
    pub fn ui_audit(&mut self, ui: &mut Ui) {
        page_title(ui, "Denetim kaydı");
        ui.label(RichText::new(ti("Kim, ne zaman, ne yaptı — son 400 kayıt.")).small().color(muted()));
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.ext.audit_q).hint_text(ti("Ara…")).desired_width(240.0));
            if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button(ti("Ara")).clicked() {
                self.load_audit();
            }
            if ui.button(ti("Yenile")).clicked() {
                self.load_audit();
            }
        });
        let list = self.ext.audit.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("audit").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                for a in &list {
                    ui.label(RichText::new(dt_tr(&js(a, "at"))).color(muted()));
                    ui.label(RichText::new(js(a, "user_name")).strong());
                    ui.label(js(a, "action"));
                    ui.label(RichText::new(js(a, "detail")).color(t_mid()));
                    ui.end_row();
                }
            });
        });
    }

    // ---------------------------------------------------------- İLAN METNİ
    pub fn listing_text_ui(&mut self, ui: &mut Ui, p: &Portfolio) {
        use crate::i18n_doc::Lang;
        // Türkçe: mevcut hazır metin; İngilizce/Rusça: çevrilmiş etiketler + o dildeki açıklama
        let tr_text = {
            let mut t = format!("{}\n{} {} — {} / {}\n", p.title, p.listing_type, p.category, p.district, p.neighborhood);
            t.push_str(&format!("Fiyat: {}\n", fmt_price(p.price, &p.currency)));
            if p.gross_m2 > 0.0 {
                t.push_str(&format!("Brüt {} m²", p.gross_m2));
                if p.net_m2 > 0.0 {
                    t.push_str(&format!(" / Net {} m²", p.net_m2));
                }
                t.push('\n');
            }
            if !p.rooms.is_empty() {
                t.push_str(&format!("Oda: {}\n", p.rooms));
            }
            for (l, v) in [("Bina yaşı", &p.building_age), ("Kat", &p.floor), ("Isıtma", &p.heating), ("Tapu", &p.deed_status)] {
                if !v.is_empty() {
                    t.push_str(&format!("{l}: {v}\n"));
                }
            }
            if !p.description.trim().is_empty() {
                t.push_str(&format!("\n{}\n", p.description.trim()));
            }
            t
        };
        let foreign = |lg: Lang| -> String {
            let mut t = format!("{}\n", p.title);
            t.push_str(&format!("{}\n", crate::export::price_str(lg, p.price, &p.currency)));
            for (l, v) in crate::export::detail_rows(p, lg) {
                if !v.trim().is_empty() {
                    t.push_str(&format!("{l}: {v}\n"));
                }
            }
            let d = match lg { Lang::En => p.description_en.trim(), _ => p.description_ru.trim() };
            if !d.is_empty() {
                t.push_str(&format!("\n{d}\n"));
            }
            t
        };
        let tabs = [("Türkçe", p.description.trim().is_empty()), ("English", p.description_en.trim().is_empty()), ("Русский", p.description_ru.trim().is_empty())];
        let mut t = match self.listing_lang { 1 => foreign(Lang::En), 2 => foreign(Lang::Ru), _ => tr_text };
        t.push_str(&format!("\n{} • {}", self.settings.office_name, self.me.full_name()));
        panel_box(ui, ti("İlan metni"), ti("Sahibinden / Hepsiemlak / sosyal medya için hazır metin. Dil sekmesini seçin."), |ui| {
            ui.horizontal(|ui| {
                for (i, (name, empty)) in tabs.iter().enumerate() {
                    let label = if *empty && i > 0 { format!("{name} ·") } else { name.to_string() };
                    if ui.selectable_label(self.listing_lang == i as u8, label).clicked() {
                        self.listing_lang = i as u8;
                    }
                }
            });
            if self.listing_lang == 1 && p.description_en.trim().is_empty() || self.listing_lang == 2 && p.description_ru.trim().is_empty() {
                ui.label(RichText::new(ti("Bu dilde açıklama girilmemiş (Düzenle > açıklama sekmeleri).")).small().color(muted()));
            }
            ui.add(egui::TextEdit::multiline(&mut t.clone()).desired_rows(8).desired_width(f32::INFINITY).interactive(false));
            if ui.button(ti("Metni kopyala")).clicked() {
                ui.ctx().copy_text(t.clone());
                self.toast(ti("İlan metni panoya kopyalandı."));
            }
        });
    }

    // ---------------------------------------------------------- PORTFÖY DETAY EKLERİ
    pub fn detail_extras(&mut self, ui: &mut Ui) {
        let id = self.detail_id;
        let hist = self.ext.hist.clone();
        if !hist.is_empty() {
            panel_box(ui, ti("Fiyat geçmişi"), "", |ui| {
                for h in &hist {
                    let (o, n) = (jf(h, "old_price"), jf(h, "new_price"));
                    let col = if n < o { Color32::from_rgb(34, 139, 70) } else { Color32::from_rgb(190, 55, 55) };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(dt_tr(&js(h, "changed_at"))).color(muted()));
                        ui.label(format!("{} → {}", fmt_price(o, &js(h, "currency")), fmt_price(n, &js(h, "currency"))));
                        ui.label(RichText::new(if n < o { "▼ düştü" } else { "▲ arttı" }).color(col));
                        ui.label(RichText::new(js(h, "user_name")).small().color(muted()));
                    });
                }
            });
        }
        let ms = self.ext.matches.clone();
        if !ms.is_empty() {
            panel_box(ui, &format!("Eşleşen alıcılar ({})", ms.len()), "Yalnızca erişiminiz olan alıcılar görünür.", |ui| {
                for m in &ms {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(js(m, "full_name")).strong());
                        ui.label(js(m, "phone"));
                        if !wa_number(&js(m, "phone")).is_empty() && ui.small_button(ti("WhatsApp")).clicked() {
                            let t = format!("Merhaba {}, kriterlerinize uygun bir portföyümüz var: {}", js(m, "full_name"), self.detail.as_ref().map(|d| d.title.clone()).unwrap_or_default());
                            self.open_url(&format!("https://wa.me/{}?text={}", wa_number(&js(m, "phone")), url_encode(&t)));
                        }
                    });
                }
            });
        }
        panel_box(ui, ti("Dahili notlar"), ti("Müşteriye verilen çıktılarda görünmez; tüm çalışanlar görür."), |ui| {
            for n in self.ext.notes.clone() {
                let nid = ji(&n, "id");
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{} • {}", js(&n, "user_name"), dt_tr(&js(&n, "created_at")))).small().color(muted()));
                    ui.label(js(&n, "text"));
                    if (self.me.is_lead() || ji(&n, "user_id") == self.me.id) && ui.small_button(ti("✕")).clicked() {
                        self.call(move |api| api.delete(&format!("/api/notes/{nid}")), move |app, _| app.load_detail_extras(id));
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.ext.note_in).hint_text(ti("Not ekle…")).desired_width(340.0));
                if ui.button(ti("Ekle")).clicked() && !self.ext.note_in.trim().is_empty() {
                    let body = json!({"text": self.ext.note_in.trim()});
                    self.ext.note_in.clear();
                    self.call(move |api| api.post(&format!("/api/portfolios/{id}/notes"), &body), move |app, _| app.load_detail_extras(id));
                }
            });
        });
    }

    // ---------------------------------------------------------- AYARLAR EKLERİ
    pub fn load_ext_settings(&mut self) {
        self.call(|api| api.get::<Value>("/api/settings/ext"), |app, v| {
            let e = &mut app.ext.ext_set;
            e.dir = js(&v, "ext_backup_dir");
            e.safe_dir = js(&v, "safe_backup_dir");
            e.keep = v["backup_keep"].as_i64().unwrap_or(14);
            e.rclone = js(&v, "rclone_remote");
            e.rclone_status = js(&v, "rclone_status");
            e.overwrite = v["ext_backup_overwrite"].as_bool().unwrap_or(true);
            e.pw_set = v["ext_backup_pw_set"].as_bool().unwrap_or(false);
            e.work_start = js(&v, "work_start");
            e.match_enabled = v["match_enabled"].as_bool().unwrap_or(true);
            e.ip_allow = js(&v, "ip_allow");
            e.offline_enabled = v["offline_enabled"].as_bool().unwrap_or(false);
            e.remind_wa = js(&v, "remind_wa");
            e.pw.clear();
            e.clear_pw = false;
            e.loaded = true;
        });
    }
    pub fn load_update_info(&mut self) {
        self.call(|api| api.get::<Value>("/api/update/info"), |app, v| app.ext.upd = Some(v));
    }

    /// Ayarlar > Yedekleme sekmesine eklenen bölüm (yalnızca yönetici).
    pub fn ext_backup_section(&mut self, ui: &mut Ui) {
        if !self.ext.ext_set.loaded {
            self.load_ext_settings();
        }
        let mut e = self.ext.ext_set.clone();
        let mut act = 0;
        panel_box(ui, ti("Güvenli yerel yedek (1. kopya)"), ti("Her yedek ayrıca Belgeler içindeki bu klasöre kopyalanır; program kaldırılsa/yeniden kurulsa bile silinmez. Tutulacak kopya sayısı aşağıda ayarlanır."), |ui| {
            ui.horizontal(|ui| {
                ui.label(ti("Klasör"));
                ui.add(egui::TextEdit::singleline(&mut e.safe_dir).desired_width(420.0));
            });
            ui.horizontal(|ui| {
                ui.label(ti("Tutulacak yedek sayısı"));
                ui.add(egui::DragValue::new(&mut e.keep).range(2..=365));
                ui.label(RichText::new(ti("(her yedek veritabanının tam kopyasıdır; fotoğraflar çoksa küçük tutun)")).small().color(muted()));
            });
        });
        panel_box(ui, ti("Bulut / harici yedek (2. kopya, şifreli)"), ti("Her yedek ayrıca bu klasöre kopyalanır: Google Drive/OneDrive/Dropbox eşitleme klasörü, USB bellek, NAS vb. Parola girilirse dosya şifrelenir (.db.enc), buluta güvenle koyabilirsiniz."), |ui| {
            ui.checkbox(&mut e.overwrite, ti("Yeni yedek eskisinin üzerine yazsın (tek dosya: emlak_son_yedek)"));
            ui.horizontal(|ui| {
                ui.label(ti("rclone hedefi (ör. gdrive:EmlakOfisYedek)"));
                ui.add(egui::TextEdit::singleline(&mut e.rclone).desired_width(260.0).hint_text("gdrive:EmlakOfisYedek"));
            });
            ui.label(RichText::new(ti("Google Drive'a doğrudan yükleme: ana bilgisayarda rclone kurup bir kez 'rclone config' ile Drive hesabınızı bağlayın; yedek parolası şarttır (veri şifreli gider).")).small().color(muted()));
            if !e.rclone_status.is_empty() {
                ui.label(RichText::new(format!("rclone: {}", e.rclone_status)).small().color(muted()));
            }
            ui.horizontal(|ui| {
                ui.label(ti("Klasör"));
                ui.add(egui::TextEdit::singleline(&mut e.dir).desired_width(340.0).hint_text(ti("/media/usb/yedek")));
            });
            ui.horizontal(|ui| {
                ui.label(ti("Parola"));
                ui.add(egui::TextEdit::singleline(&mut e.pw).password(true).desired_width(200.0).hint_text(if e.pw_set { "(kayıtlı — değiştirmek için yazın)" } else { "(boş = şifresiz)" }));
                if e.pw_set {
                    ui.checkbox(&mut e.clear_pw, ti("Parolayı kaldır"));
                }
            });
            ui.horizontal(|ui| {
                ui.label(ti("Mesai başlangıç saati"));
                ui.add(egui::TextEdit::singleline(&mut e.work_start).desired_width(60.0));
                ui.label(RichText::new(ti("(SS:DD — geç kalma hesabı)")).small().color(muted()));
            });
            ui.checkbox(&mut e.match_enabled, ti("Yeni/uygun portföy olunca alıcı sahibine eşleşme bildirimi gönder"));
            ui.horizontal(|ui| {
                if primary(ui, ti("Kaydet")) {
                    act = 1;
                }
                if ui.button(ti("Şimdi yedekle")).clicked() {
                    act = 2;
                }
            });
        });
        panel_box(ui, ti("Çevrimdışı giriş ve yerel veri kopyası"), ti("Açıkken her kullanıcının bilgisayarı, son bilinen verileri KENDİ parolasından türetilen anahtarla şifreli saklar. Ana bilgisayar kapalıyken bile aynı parolayla giriş yapılır, kayıtlar görüntülenir, yeni işlemler kuyruğa alınır ve bağlantı gelince gönderilir. Kapatılırsa kullanıcı bilgisayarlarındaki kopyalar bir sonraki girişte silinir. Not: açık olduğunda müşteri/portföy verisinin şifreli bir kopyası çalışan bilgisayarlarında bulunur."), |ui| {
            if ui.checkbox(&mut e.offline_enabled, ti("Çevrimdışı giriş ve yerel kopya açık")).changed() {
                act = 5;
            }
        });
        panel_box(ui, ti("IP kısıtlama"), ti("Boş bırakırsanız tüm bağlantılara izin verilir. Doluysa yalnızca listedeki IP/ağlardan bağlanılabilir (her satıra bir kayıt: 203.0.113.7 veya 192.168.1.0/24). Ana bilgisayarın kendisi her zaman erişebilir. DİKKAT: kendi IP'nizi listeye eklemeyi unutmayın; Tailscale kullanıyorsanız 100.64.0.0/10 ekleyin."), |ui| {
            ui.add(egui::TextEdit::multiline(&mut e.ip_allow).desired_rows(3).desired_width(360.0).hint_text("192.168.1.0/24\n100.64.0.0/10"));
            if primary(ui, ti("IP listesini kaydet")) {
                act = 3;
            }
        });
        panel_box(ui, ti("Hatırlatma metni (WhatsApp / e-posta)"), ti("Randevu kartındaki 'WhatsApp hatırlat' ve 'E-posta hatırlat' düğmeleri bu metni kullanır. Yer tutucular: {ad} {baslik} {tarih} {saat} {ofis}"), |ui| {
            ui.add(egui::TextEdit::multiline(&mut e.remind_wa).desired_rows(3).desired_width(420.0));
            if primary(ui, ti("Metni kaydet")) {
                act = 4;
            }
        });
        if act == 3 || act == 4 || act == 5 {
            let body = match act { 3 => json!({"ip_allow": e.ip_allow}), 4 => json!({"remind_wa": e.remind_wa}), _ => json!({"offline_enabled": e.offline_enabled}) };
            self.call(move |api| api.put("/api/settings/ext", &body), |app, _| {
                app.toast(ti("Kaydedildi."));
                app.ext.ext_set.loaded = false;
            });
        } else if act == 1 {
            let body = json!({"ext_backup_dir": e.dir, "safe_backup_dir": e.safe_dir, "backup_keep": e.keep, "rclone_remote": e.rclone, "ext_backup_overwrite": e.overwrite, "ext_backup_pw": e.pw, "clear_pw": e.clear_pw, "work_start": e.work_start, "match_enabled": e.match_enabled});
            self.call(move |api| api.put("/api/settings/ext", &body), |app, _| {
                app.toast(ti("Kaydedildi."));
                app.ext.ext_set.loaded = false;
            });
        } else if act == 2 {
            self.call(|api| api.post("/api/backup/external", &json!({})), |app, v| {
                let ext = js(&v, "external");
                app.toast(if ext.is_empty() { "Yedek alındı (harici klasör tanımlı değil).".to_string() } else { format!("Yedek alındı: {ext}") });
            });
        }
        self.ext.ext_set = e;

        if self.server_state.is_some() {
            let mut start: Option<std::path::PathBuf> = None;
            panel_box(ui, ti("Yedekten geri yükle (yalnızca ana bilgisayar)"), ti("Seçilen yedek doğrulanır; mevcut veri 'yedekler' klasörüne ayrıca saklanır. Program kapanır, geri yükler ve yeniden açılır. Şifreli (.db.enc) yedek için aşağıdaki parola kutusunu doldurun. DİKKAT: geri yüklenen yedekten sonra eklenen kayıtlar yeni veride olmaz; diğer bilgisayarlar bağlantıyı yeniden kurar."), |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.ext.restore_pw).password(true).hint_text(ti("Yedek parolası (yalnızca şifreli yedek için)")).desired_width(260.0));
                if let Some(p) = self.ext.restore_pick.clone() {
                    ui.label(RichText::new(format!("Seçilen yedek: {}", p.display())).strong());
                    ui.horizontal(|ui| {
                        if confirm_btn(ui, "do_restore", "Geri yükle ve yeniden başlat") {
                            start = Some(p.clone());
                        }
                        if ui.button(ti("Vazgeç")).clicked() {
                            self.ext.restore_pick = None;
                        }
                    });
                } else if ui.button(ti("Yedek dosyası seç…")).clicked() {
                    self.spawn(
                        || -> Result<Option<std::path::PathBuf>, String> { Ok(rfd::FileDialog::new().add_filter("Yedek", &["db", "enc"]).pick_file()) },
                        |app, r| {
                            if let Ok(Some(p)) = r {
                                app.ext.restore_pick = Some(p);
                            }
                        },
                    );
                }
            });
            if let Some(p) = start {
                let dir = if self.cfg.data_dir.is_empty() { crate::app::default_data_dir() } else { std::path::PathBuf::from(&self.cfg.data_dir) };
                match std::env::current_exe() {
                    Ok(exe) => {
                        let mut cmd = std::process::Command::new(exe);
                        cmd.arg("--geri-yukle").arg(&p).arg("--veri").arg(&dir).arg("--sonra-ac").env("EMLAK_YEDEK_PAROLA", &self.ext.restore_pw);
                        match crate::updater::spawn_detached(&mut cmd) {
                            Ok(_) => std::process::exit(0),
                            Err(e) => self.err(format!("Geri yükleme başlatılamadı: {e}")),
                        }
                    }
                    Err(e) => self.err(e.to_string()),
                }
            }
        }

        panel_box(ui, ti("Şifreli yedeği çöz"), ti("Parolayla şifrelenmiş .db.enc dosyasını normal .db dosyasına çevirir. Doğrudan geri yüklemek için yukarıdaki bölümü kullanın (şifreli dosyayı çözmeden seçebilirsiniz)."), |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.ext.restore_pw).password(true).hint_text(ti("Yedek parolası")).desired_width(220.0));
            if ui.button(ti("Dosya seç ve çöz…")).clicked() {
                let pw = self.ext.restore_pw.clone();
                self.spawn(
                    move || -> Result<String, String> {
                        let Some(src) = rfd::FileDialog::new().add_filter("Şifreli yedek", &["enc"]).pick_file() else { return Ok(String::new()) };
                        let data = std::fs::read(&src).map_err(|e| e.to_string())?;
                        let plain = crate::ext_db::decrypt_bytes(&data, &pw)?;
                        let out = src.with_extension("");
                        let out = if out.extension().is_some() { out } else { out.with_extension("db") };
                        std::fs::write(&out, plain).map_err(|e| e.to_string())?;
                        Ok(out.display().to_string())
                    },
                    |app, r| match r {
                        Ok(p) if p.is_empty() => {}
                        Ok(p) => app.toast(format!("Çözüldü: {p}")),
                        Err(e) => app.err(format!("Çözülemedi (parola yanlış olabilir): {e}")),
                    },
                );
            }
        });
    }

    /// Sunucuda bu bilgisayardan daha yeni sürüm varsa açılış ekranında bilgilendirir.
    pub fn newer_on_server(&self) -> Option<String> {
        let info = self.ext.upd.as_ref()?;
        for p in info["packages"].as_array()? {
            if js(p, "platform") == crate::updater::platform() && crate::updater::is_newer(&js(p, "version"), crate::updater::current_version()) {
                return Some(js(p, "version"));
            }
        }
        None
    }

    pub fn update_banner(&mut self, ui: &mut Ui) {
        if self.ext.upd_banner_hidden {
            return;
        }
        let Some(v) = self.newer_on_server() else { return };
        let mut hide = false;
        let mut go = false;
        egui::Frame::new().fill(note_bg()).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::symmetric(12, 8)).stroke(egui::Stroke::new(1.5_f32, accent())).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("⬆ Yeni sürüm hazır: v{v}")).strong().color(accent()));
                ui.label(RichText::new(format!("(bu bilgisayarda v{}) — şu an eski sürümle çalışmaya devam edebilirsiniz.", crate::updater::current_version())).small().color(muted()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(ti("Daha sonra")).clicked() {
                        hide = true;
                    }
                    if ui.add(egui::Button::new(RichText::new(ti("Ayarlar › Güncelleme")).strong().color(on_btn())).fill(btn())).clicked() {
                        go = true;
                    }
                });
            });
        });
        ui.add_space(6.0);
        if hide {
            self.ext.upd_banner_hidden = true;
        }
        if go {
            self.set_tab = 2;
            self.goto(View::Settings);
        }
    }

    /// Paketi kurar: yönetici ise önce sunucuya da yükler (diğer bilgisayarlar görsün), sonra yeniden başlatır.
    fn run_install(&mut self, pkg: crate::updater::Package) {
        self.ext.upd_busy = true;
        self.ext.upd_msg = "Kuruluyor… program kapanıp kendiliğinden yeniden açılacak.".into();
        let admin = self.me.is_admin();
        self.bg(
            move |api| -> Result<(), String> {
                if admin {
                    let _ = api.post::<Value>("/api/backup", &json!({}));
                    for (pl, bytes) in &pkg.bins {
                        let sig = pkg.sigs.get(pl).cloned().unwrap_or_default();
                        let _ = api.send_bytes("POST", &format!("/api/update/upload?platform={pl}&version={}&sig={}", pkg.version, url_encode(&sig)), bytes);
                    }
                }
                let me = pkg.for_this_platform().ok_or_else(|| format!("Pakette {} için program dosyası yok.", crate::updater::platform()))?;
                crate::updater::install_and_restart(me)
            },
            |app, r| {
                app.ext.upd_busy = false;
                if let Err(e) = r {
                    app.ext.upd_msg.clear();
                    app.err(e);
                }
            },
        );
    }

    /// Ayarlar > Hakkında sekmesi: uygulama içi güncelleme.
    pub fn ext_update_section(&mut self, ui: &mut Ui) {
        if self.ext.upd.is_none() {
            self.load_update_info();
        }
        let plat = crate::updater::platform();
        let cur = crate::updater::current_version();
        let info = self.ext.upd.clone();
        let mut install: Option<crate::updater::Package> = None;
        let mut dl = false;
        panel_box(ui, ti("Güncelleme"), ti("Güncelleme paketini (EmlakOfis-Guncelleme-….zip) seçin; program kapanır, kurar ve kendiliğinden yeniden açılır. Yeni sürüm açılmazsa eski sürüme otomatik dönülür."), |ui| {
            ui.label(format!("Bu bilgisayardaki sürüm: v{cur} ({plat})"));
            ui.add_space(4.0);
            if !self.ext.upd_busy && primary(ui, ti("Güncelleme paketi seç (.zip)…")) {
                self.spawn(
                    || -> Result<Option<crate::updater::Package>, String> {
                        let Some(f) = rfd::FileDialog::new().add_filter("Güncelleme paketi", &["zip"]).pick_file() else { return Ok(None) };
                        crate::updater::read_package(&f).map(Some)
                    },
                    |app, r| match r {
                        Ok(Some(p)) => {
                            app.ext.upd_msg.clear();
                            app.ext.upd_pkg = Some(p);
                        }
                        Ok(None) => {}
                        Err(e) => app.err(e),
                    },
                );
            }
            if let Some(p) = self.ext.upd_pkg.clone() {
                ui.add_space(6.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("Paket: v{}", p.version)).strong());
                    ui.label(RichText::new(format!("İçerdiği platformlar: {}", p.platforms().join(", "))).small().color(muted()));
                    if p.signed {
                        ui.colored_label(Color32::from_rgb(34, 139, 70), "✔ Dijital imza doğrulandı (güvenilir yayıncı).");
                    } else {
                        ui.label(RichText::new(ti("İmza denetlenmedi (güvenilir yayıncı anahtarı tanımlı değil).")).small().color(Color32::from_rgb(200, 130, 20)));
                    }
                    if !p.notes.is_empty() {
                        ui.label(p.notes.clone());
                    }
                    if p.for_this_platform().is_none() {
                        ui.colored_label(Color32::from_rgb(190, 55, 55), format!("Bu pakette {plat} için program dosyası yok."));
                    } else if crate::updater::is_newer(&p.version, cur) {
                        ui.colored_label(Color32::from_rgb(34, 139, 70), format!("v{cur} → v{} güncellemesi kurulabilir.", p.version));
                        if self.me.is_admin() {
                            ui.label(RichText::new(ti("Yönetici olarak kurarsanız paket ayrıca ana bilgisayara yüklenir; diğer bilgisayarlar açılışta 'yeni sürüm hazır' bildirimi görür.")).small().color(muted()));
                        }
                        ui.horizontal(|ui| {
                            if !self.ext.upd_busy && primary(ui, ti("Kur ve yeniden başlat")) {
                                install = Some(p.clone());
                            }
                            if ui.button(ti("Vazgeç")).clicked() {
                                self.ext.upd_pkg = None;
                            }
                        });
                    } else {
                        ui.label(RichText::new(format!("Bu paket (v{}) mevcut sürümden (v{cur}) yeni değil; kurulmaz.", p.version)).color(muted()));
                        if ui.button(ti("Kapat")).clicked() {
                            self.ext.upd_pkg = None;
                        }
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new(ti("Ana bilgisayardaki paket")).strong());
            if let Some(i) = &info {
                let mut any = false;
                for p in i["packages"].as_array().cloned().unwrap_or_default() {
                    any = true;
                    ui.label(format!("{}: v{} ({:.1} MB)", js(&p, "platform"), js(&p, "version"), jf(&p, "size") / 1_048_576.0));
                }
                if !any {
                    ui.label(RichText::new(ti("Ana bilgisayara henüz paket yüklenmemiş. Yönetici paketi bu ekrandan kurunca otomatik yüklenir.")).small().color(muted()));
                }
            }
            if let Some(v) = self.newer_on_server() {
                ui.colored_label(Color32::from_rgb(34, 139, 70), format!("Ana bilgisayarda daha yeni sürüm var: v{v}"));
                if !self.ext.upd_busy && primary(ui, ti("Ana bilgisayardan indir ve kur")) {
                    dl = true;
                }
            }
            if !self.ext.upd_msg.is_empty() {
                ui.label(RichText::new(self.ext.upd_msg.clone()).color(accent()));
            }
            if ui.small_button(ti("Denetle")).clicked() {
                self.load_update_info();
            }
            ui.label(RichText::new(ti("Kurulum sırasında eski program '.eski' adıyla program klasöründe saklanır. Güvenlik: aşağıya güvenilir yayıncı anahtarı girerseniz yalnızca imzalı paketler kurulur.")).small().color(muted()));
        });
        panel_box(ui, ti("Güncelleme güvenliği (dijital imza)"), ti("Paketi siz imzalarsınız; bilgisayarlar yalnızca sizin imzanızı taşıyan paketi kurar. Anahtar yoksa imza denetlenmez."), |ui| {
            let mut key = self.cfg.trusted_key.clone();
            ui.label(ti("Güvenilir yayıncı anahtarı (açık anahtar):"));
            if ui.add(egui::TextEdit::singleline(&mut key).desired_width(f32::INFINITY).hint_text(ti("boş = imza denetimi kapalı"))).changed() {
                self.cfg.trusted_key = key.trim().to_string();
                crate::updater::set_trusted_key(&self.cfg.trusted_key);
                self.cfg.save();
            }
            if self.cfg.trusted_key.is_empty() {
                ui.label(RichText::new(ti("İmza denetimi KAPALI.")).small().color(Color32::from_rgb(200, 130, 20)));
            } else {
                ui.label(RichText::new(ti("İmza denetimi AÇIK: imzasız veya başka anahtarla imzalı paket reddedilir.")).small().color(Color32::from_rgb(34, 139, 70)));
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button(ti("Yeni anahtar çifti üret")).clicked() {
                    let dir = directories::UserDirs::new().and_then(|u| u.document_dir().map(|d| d.to_path_buf())).unwrap_or_else(|| std::path::PathBuf::from(".")).join("Emlak Ofis Anahtar");
                    match crate::sign::generate_into(&dir) {
                        Ok((p, public)) => {
                            self.cfg.trusted_key = public;
                            crate::updater::set_trusted_key(&self.cfg.trusted_key);
                            self.cfg.save();
                            self.toast(format!("Gizli anahtar yazıldı: {} — YEDEKLEYİN, kimseyle paylaşmayın.", p.display()));
                        }
                        Err(e) => self.err(e),
                    }
                }
                if ui.button(ti("Güncelleme paketini imzala…")).clicked() {
                    self.pick_files("İmzalanacak güncelleme paketi (.zip)", &["zip"], false, |app, zips| {
                        if let Some(z) = zips.into_iter().next() {
                            app.pick_files("Gizli anahtar dosyası (.gizli)", &["gizli", "txt", "key"], false, move |app, keys| {
                                if let Some(k) = keys.into_iter().next() {
                                    match crate::sign::cli_sign_package(&z.to_string_lossy(), &k.to_string_lossy()) {
                                        Ok(()) => app.toast(ti("Paket imzalandı.")),
                                        Err(e) => app.err(e),
                                    }
                                }
                            });
                        }
                    });
                }
            });
            ui.label(RichText::new(ti("Anahtar üretme (bir kez): emlak-ofis --anahtar-uret anahtarim  → anahtarim.gizli (saklayın, paylaşmayın) + açık anahtar ekrana basılır.\nPaket imzalama: emlak-ofis --paket-imzala EmlakOfis-Guncelleme-X.zip anahtarim.gizli")).small().color(muted()));
        });
        self.web_settings_ui(ui);
        panel_box(ui, ti("Kurulum ve lisans kodları"), ti("Her yeni kurulum bir kodla açılır: ana bilgisayar için YÖNETİCİ kodu, diğer bilgisayarlar için KULLANICI kodu. Kodsuz kurulum yapılamaz. Kodda iki ayrı süre vardır: (1) kodun kurulumda kullanılabileceği gün sayısı, (2) programın çalışabileceği gün sayısı (lisans bitiş tarihi). Lisans bitince program kilitlenir; yeni kod girilince devam eder, veri silinmez."), |ui| {
            if crate::sign::host_key(&self.cfg.trusted_key).is_empty() {
                ui.label(RichText::new(ti("Koruma KAPALI: önce yukarıya güvenilir yayıncı (açık) anahtarını girin.")).small().color(Color32::from_rgb(200, 130, 20)));
            }
            match crate::sign::license_days_left(&self.cfg.lic_until) {
                Some(d) => {
                    ui.label(RichText::new(format!("{} {} ({} {})", ti("Bu kurulumun lisans bitişi:"), self.cfg.lic_until, d.max(0), ti("gün kaldı"))).color(if d <= 14 { Color32::from_rgb(200, 90, 20) } else { muted() }));
                }
                None => {
                    ui.label(RichText::new(ti("Bu kurulumun lisansı: süresiz")).color(muted()));
                }
            }
            ui.horizontal(|ui| {
                ui.label(ti("Kod türü:"));
                ui.selectable_value(&mut self.ext.code_kind, 0, ti("Yönetici (ana bilgisayar)"));
                ui.selectable_value(&mut self.ext.code_kind, 1, ti("Kullanıcı (istemci)"));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(ti("Kod kaç gün içinde kullanılabilsin (0 = süresiz):"));
                ui.add(egui::DragValue::new(&mut self.ext.host_days).range(0..=3650));
                ui.label(ti("Program kaç gün çalışabilsin (0 = süresiz):"));
                ui.add(egui::DragValue::new(&mut self.ext.lic_days).range(0..=3650));
            });
            ui.horizontal(|ui| {
                ui.label(ti("Not (kime/hangi ofis):"));
                ui.add(egui::TextEdit::singleline(&mut self.ext.host_note).desired_width(200.0));
            });
            if primary(ui, ti("Gizli anahtar dosyasını seç ve kod üret…")) {
                self.pick_files("Gizli anahtar dosyası", &["gizli", "txt", "key"], false, |app, files| {
                    if let Some(f) = files.into_iter().next() {
                        let kind = if app.ext.code_kind == 0 { 'H' } else { 'K' };
                        match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|k| crate::sign::make_code(&k, kind, app.ext.host_days, app.ext.lic_days, &app.ext.host_note)) {
                            Ok(c) => app.ext.host_code = c,
                            Err(e) => app.err(e),
                        }
                    }
                });
            }
            if !self.ext.host_code.is_empty() {
                let mut c = self.ext.host_code.clone();
                ui.add(egui::TextEdit::multiline(&mut c).desired_rows(3).desired_width(f32::INFINITY));
                if ui.button(ti("Kodu kopyala")).clicked() {
                    ui.ctx().copy_text(self.ext.host_code.clone());
                    self.toast(ti("Kod kopyalandı."));
                }
                ui.label(RichText::new(ti("Kodu yalnızca ilgili kişiye iletin. Üretilen kod çevrimdışı geri çağrılamaz; bu yüzden 'program kaç gün çalışabilsin' sınırı asıl denetimdir.")).small().color(muted()));
            }
            ui.separator();
            ui.label(RichText::new(ti("Bu kuruluma yeni/uzatılmış lisans kodu gir (yenileme):")).small());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.lic_code).hint_text("EO-…").desired_width(300.0));
                if ui.button(ti("Uygula")).clicked() {
                    let key = crate::sign::host_key(&self.cfg.trusted_key);
                    match crate::sign::verify_code(&key, &self.lic_code) {
                        Ok(i) if i.kind == 'H' || self.cfg.mode != "server" => {
                            self.cfg.lic_until = if i.license_until == crate::sign::NO_LIMIT { String::new() } else { i.license_until };
                            self.cfg.lic_kind = i.kind.to_string();
                            self.cfg.save();
                            self.lic_code.clear();
                            self.toast(ti("Lisans güncellendi."));
                        }
                        Ok(_) => self.err("Ana bilgisayar için YÖNETİCİ kodu gerekir.".to_string()),
                        Err(e) => self.err(e),
                    }
                }
            });
        });
        if let Some(p) = install {
            self.ext.upd_pkg = None;
            self.run_install(p);
        }
        if dl {
            self.ext.upd_busy = true;
            self.ext.upd_msg = "İndiriliyor…".into();
            let p = plat.to_string();
            let ver = self.newer_on_server().unwrap_or_default();
            self.bg(
                move |api| -> Result<(), String> {
                    let bytes = api.get_bytes(&format!("/api/update/download?platform={p}"))?;
                    if bytes.len() < 1_000_000 {
                        return Err("İndirilen dosya beklenenden küçük.".into());
                    }
                    // Güvenilir yayıncı anahtarı varsa ana bilgisayardan inen dosya da imzayla doğrulanır.
                    let tk = crate::updater::trusted_key();
                    if !tk.is_empty() {
                        let info: Value = api.get("/api/update/info")?;
                        let sig = info["packages"].as_array().and_then(|a| a.iter().find(|x| x["platform"] == p.as_str())).and_then(|x| x["sig"].as_str()).unwrap_or("").to_string();
                        if sig.is_empty() {
                            return Err("Ana bilgisayardaki paket imzasız; güvenilir yayıncı anahtarı tanımlı olduğu için kurulmadı.".into());
                        }
                        crate::sign::verify(&tk, &ver, &p, &bytes, &sig)?;
                    }
                    crate::updater::install_and_restart(&bytes)
                },
                |app, r| {
                    app.ext.upd_busy = false;
                    app.ext.upd_msg.clear();
                    if let Err(e) = r {
                        app.err(e);
                    }
                },
            );
        }
    }
}
