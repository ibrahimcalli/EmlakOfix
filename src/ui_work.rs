//! Görevler, mesai, hedefler ve bildirimler.
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::common::*;
use crate::ui_portfolio::combo_opt;
use chrono::{Duration as CDur, NaiveDateTime};
use eframe::egui::{self, vec2, Color32, RichText, Ui};
use egui_extras::DatePickerButton;
use std::collections::BTreeMap;

fn kind_color(k: &str) -> Color32 {
    match k {
        "Toplantı" => Color32::from_rgb(130, 70, 170),
        "Not" => Color32::from_rgb(120, 120, 130),
        "Hedef" => Color32::from_rgb(200, 120, 20),
        "Portföy" => Color32::from_rgb(31, 120, 100),
        _ => Color32::from_rgb(45, 100, 190),
    }
}

fn fmt_dur(secs: i64) -> String {
    let m = secs.max(0) / 60;
    format!("{} sa {:02} dk", m / 60, m % 60)
}

fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")).ok()
}

impl App {
    // ------------------------------------------------------------------ görevler
    fn task_form_ui(&mut self, ui: &mut Ui) {
        if !self.tform.show {
            if ui.add(egui::Button::new(RichText::new(ti("+ Çalışana görev / toplantı / not ata")).strong().color(on_btn())).fill(btn())).clicked() {
                self.tform.show = true;
            }
            return;
        }
        let staff: Vec<UserInfo> = self.users.iter().filter(|u| u.active).cloned().collect();
        card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(ti("Yeni atama")).size(17.0).strong());
            ui.horizontal(|ui| {
                for k in TASK_KINDS {
                    ui.radio_value(&mut self.tform.kind, k.to_string(), k);
                }
            });
            ui.add(egui::TextEdit::singleline(&mut self.tform.title).hint_text(ti("Başlık (örn: Cumartesi toplantısı)")).desired_width(520.0));
            ui.add(egui::TextEdit::multiline(&mut self.tform.details).hint_text(ti("Açıklama / sorumluluk / not")).desired_rows(3).desired_width(520.0));
            ui.label(RichText::new(ti("Kime atanacak?")).strong());
            ui.horizontal_wrapped(|ui| {
                for u in &staff {
                    let mut on = self.tform.selected.contains(&u.id);
                    if ui.checkbox(&mut on, u.full_name()).changed() {
                        if on {
                            self.tform.selected.insert(u.id);
                        } else {
                            self.tform.selected.remove(&u.id);
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.small_button(ti("Tümünü seç")).clicked() {
                    self.tform.selected = staff.iter().map(|u| u.id).collect();
                }
                if ui.small_button(ti("Temizle")).clicked() {
                    self.tform.selected.clear();
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.tform.use_due, ti("Tarih / saat belirle"));
                if self.tform.use_due {
                    ui.add(DatePickerButton::new(&mut self.tform.date).id_salt("task_date"));
                    ui.add(egui::DragValue::new(&mut self.tform.hour).range(0..=23).suffix(" sa"));
                    ui.add(egui::DragValue::new(&mut self.tform.minute).range(0..=59).suffix(" dk"));
                }
            });
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(ti("Ata ve bildirim gönder")).strong().color(on_btn())).fill(btn())).clicked() {
                    let due = if self.tform.use_due { format!("{} {:02}:{:02}", self.tform.date.format("%Y-%m-%d"), self.tform.hour, self.tform.minute) } else { String::new() };
                    let nt = NewTask {
                        title: self.tform.title.trim().to_string(),
                        details: self.tform.details.trim().to_string(),
                        kind: self.tform.kind.clone(),
                        assignees: self.tform.selected.iter().cloned().collect(),
                        due_at: due,
                    };
                    self.call(
                        move |api| api.post("/api/tasks", &nt),
                        |app, _| {
                            app.toast(ti("Atama yapıldı; çalışanlara bildirim gitti."));
                            app.tform.show = false;
                            app.tform.title.clear();
                            app.tform.details.clear();
                            app.tform.selected.clear();
                            app.tform.use_due = false;
                            app.load_tasks();
                        },
                    );
                }
                if ui.button(ti("Kapat")).clicked() {
                    self.tform.show = false;
                }
            });
        });
    }

    pub fn ui_tasks(&mut self, ui: &mut Ui) {
        let admin = self.me.is_admin();
        page_title(ui, if admin { "Görevler, toplantılar ve notlar" } else { "Görevlerim" });
        if admin {
            self.task_form_ui(ui);
            ui.add_space(6.0);
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.task_filter, 0, ti("Açık"));
            ui.selectable_value(&mut self.task_filter, 1, ti("Tamamlanan"));
            ui.selectable_value(&mut self.task_filter, 2, ti("Tümü"));
            if admin {
                ui.separator();
                let names: Vec<String> = self.users.iter().map(|u| u.full_name()).collect();
                let mut sel = self.users.iter().find(|u| u.id == self.task_user).map(|u| u.full_name()).unwrap_or_default();
                if combo_opt(ui, "task_user", "Tüm çalışanlar", &mut sel, &names, 180.0) {
                    self.task_user = self.users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(0);
                }
            }
            if ui.button(ti("Yenile")).clicked() {
                self.load_tasks();
            }
        });
        ui.separator();
        let now = chrono::Local::now().naive_local();
        let tasks: Vec<TaskInfo> = self
            .tasks
            .iter()
            .filter(|t| match self.task_filter {
                0 => t.status != "Tamamlandı",
                1 => t.status == "Tamamlandı",
                _ => true,
            })
            .filter(|t| self.task_user == 0 || t.assigned_to == self.task_user)
            .cloned()
            .collect();
        if tasks.is_empty() {
            ui.add_space(16.0);
            ui.label(RichText::new(ti("Bu filtrede görev yok.")).color(muted()));
            return;
        }
        let mut toggle: Option<(i64, bool)> = None;
        let mut delete: Option<i64> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for t in &tasks {
                let done = t.status == "Tamamlandı";
                let overdue = !done && parse_dt(&t.due_at).map(|d| d < now).unwrap_or(false);
                card_frame(ui.style()).inner_margin(10.0).fill(if done { done_bg() } else { card() }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        badge(ui, &t.kind, kind_color(&t.kind));
                        let title = RichText::new(&t.title).size(16.0).strong();
                        ui.label(if done { title.strikethrough().color(muted()) } else { title });
                        if !t.due_at.is_empty() {
                            let c = if overdue { Color32::from_rgb(190, 40, 40) } else { Color32::from_rgb(30, 100, 160) };
                            ui.label(RichText::new(format!("{}{}", if overdue { "Süresi geçti: " } else { "Zaman: " }, dt_tr(&t.due_at))).color(c).strong());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if admin && confirm_btn(ui, &format!("tdel{}", t.id), "Sil") {
                                delete = Some(t.id);
                            }
                            if t.kind == "Not" && done {
                                if ui.button(ti("Geri aç")).clicked() {
                                    toggle = Some((t.id, false));
                                }
                            } else if done {
                                if ui.button(ti("Geri aç")).clicked() {
                                    toggle = Some((t.id, false));
                                }
                            } else if ui.add(egui::Button::new(RichText::new(if t.kind == "Not" { "Okudum" } else { "Tamamlandı" }).color(Color32::WHITE)).fill(Color32::from_rgb(37, 140, 80))).clicked() {
                                toggle = Some((t.id, true));
                            }
                        });
                    });
                    if !t.details.is_empty() {
                        ui.label(&t.details);
                    }
                    let mut meta = format!("Atayan: {}  •  {}", t.created_by_name, dt_tr(&t.created_at));
                    if admin {
                        meta = format!("Sorumlu: {}  •  {}", t.assignee_name, meta);
                    }
                    ui.label(RichText::new(meta).small().color(muted()));
                });
                ui.add_space(4.0);
            }
        });
        if let Some((id, done)) = toggle {
            let st = if done { "Tamamlandı" } else { "Açık" };
            self.call(move |api| api.put(&format!("/api/tasks/{id}/status"), &serde_json::json!({"status": st})), |app, _| app.load_tasks());
        }
        if let Some(id) = delete {
            self.call(move |api| api.delete(&format!("/api/tasks/{id}")), |app, _| app.load_tasks());
        }
    }

    // ------------------------------------------------------------------ mesai
    pub fn ui_attendance(&mut self, ui: &mut Ui) {
        let admin = self.me.is_admin();
        page_title(ui, "Mesai takibi");
        let my_open = self.att.iter().find(|a| a.user_id == self.me.id && a.check_out.is_empty()).cloned();
        card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                match &my_open {
                    Some(a) => {
                        ui.label(RichText::new(format!("Mesaidesiniz • giriş: {}", dt_tr(&a.check_in))).size(17.0).strong().color(Color32::from_rgb(30, 120, 60)));
                        if ui.add(egui::Button::new(RichText::new(ti("Mesaiyi bitir")).size(16.0).color(Color32::WHITE)).fill(Color32::from_rgb(190, 60, 50)).min_size(vec2(160.0, 34.0))).clicked() {
                            self.call(|api| api.post("/api/attendance/out", &serde_json::json!({})), |app, _| {
                                app.toast(ti("Mesai bitirildi. İyi dinlenmeler!"));
                                app.load_att();
                            });
                        }
                    }
                    None => {
                        ui.label(RichText::new(ti("Şu an mesaide değilsiniz.")).size(17.0).color(muted()));
                        if ui.add(egui::Button::new(RichText::new(ti("Mesaiye başla")).size(16.0).color(Color32::WHITE)).fill(Color32::from_rgb(37, 140, 80)).min_size(vec2(160.0, 34.0))).clicked() {
                            self.call(|api| api.post("/api/attendance/in", &serde_json::json!({})), |app, _| {
                                app.toast(ti("Mesai başladı. Kolay gelsin!"));
                                app.load_att();
                            });
                        }
                    }
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(ti("‹ Önceki ay")).clicked() {
                self.att_month = first_of_month(self.att_month - CDur::days(1));
                self.load_att();
            }
            ui.label(RichText::new(self.att_month.format("%m / %Y").to_string()).size(17.0).strong());
            if ui.button(ti("Sonraki ay ›")).clicked() {
                self.att_month = first_of_month(last_of_month(self.att_month) + CDur::days(1));
                self.load_att();
            }
            if admin {
                ui.separator();
                let names: Vec<String> = self.users.iter().map(|u| u.full_name()).collect();
                let mut sel = self.users.iter().find(|u| u.id == self.att_user).map(|u| u.full_name()).unwrap_or_default();
                if combo_opt(ui, "att_user", "Tüm çalışanlar", &mut sel, &names, 180.0) {
                    self.att_user = self.users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(0);
                    self.load_att();
                }
            }
        });
        let now = chrono::Local::now().naive_local();
        let secs = |a: &AttInfo| -> i64 {
            match parse_dt(&a.check_in) {
                Some(i) => {
                    let o = if a.check_out.is_empty() { now } else { parse_dt(&a.check_out).unwrap_or(now) };
                    (o - i).num_seconds().max(0)
                }
                None => 0,
            }
        };
        let mut totals: BTreeMap<String, i64> = BTreeMap::new();
        for a in &self.att {
            *totals.entry(a.user_name.clone()).or_default() += secs(a);
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(ti("Aylık toplam:")).color(muted()));
            for (n, s) in &totals {
                ui.label(RichText::new(format!("{n}: {}", fmt_dur(*s))).strong());
                ui.add_space(8.0);
            }
        });
        ui.separator();
        let rows = self.att.clone();
        let mut edit: Option<AttInfo> = None;
        let mut del: Option<i64> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("att_grid").striped(true).num_columns(if admin { 6 } else { 5 }).spacing([18.0, 6.0]).show(ui, |ui| {
                for h in if admin { vec!["Çalışan", "Tarih", "Giriş", "Çıkış", "Süre", "İşlem"] } else { vec!["Tarih", "Giriş", "Çıkış", "Süre", "Not"] } {
                    ui.label(RichText::new(h).strong().color(muted()));
                }
                ui.end_row();
                for a in &rows {
                    if admin {
                        ui.label(&a.user_name);
                    }
                    ui.label(date_tr(&a.check_in));
                    ui.label(a.check_in.get(11..16).unwrap_or(""));
                    if a.check_out.is_empty() {
                        ui.label(RichText::new(ti("devam ediyor")).color(Color32::from_rgb(30, 120, 60)));
                    } else {
                        ui.label(a.check_out.get(11..16).unwrap_or(""));
                    }
                    ui.label(fmt_dur(secs(a)));
                    if admin {
                        ui.horizontal(|ui| {
                            if ui.small_button(ti("Düzenle")).clicked() {
                                edit = Some(a.clone());
                            }
                            if confirm_btn(ui, &format!("adel{}", a.id), "Sil") {
                                del = Some(a.id);
                            }
                        });
                    } else {
                        ui.label(&a.note);
                    }
                    ui.end_row();
                }
            });
            if rows.is_empty() {
                ui.label(RichText::new(ti("Bu ay için mesai kaydı yok.")).color(muted()));
            }
        });
        if let Some(a) = edit {
            self.att_edit = Some(a);
        }
        if let Some(id) = del {
            self.call(move |api| api.delete(&format!("/api/attendance/{id}")), |app, _| app.load_att());
        }
        self.att_edit_window(ui.ctx());
    }

    fn att_edit_window(&mut self, ctx: &egui::Context) {
        let Some(mut a) = self.att_edit.take() else { return };
        let mut keep = true;
        let mut save = false;
        egui::Window::new(format!("Mesai kaydını düzenle — {}", a.user_name)).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.label(RichText::new(ti("Biçim: YYYY-AA-GG SS:DD:ss  (örn: 2026-11-09 09:00:00). Çıkışı boş bırakırsanız mesai açık kalır.")).small().color(muted()));
            egui::Grid::new("att_edit").num_columns(2).show(ui, |ui| {
                ui.label(ti("Giriş"));
                ui.add(egui::TextEdit::singleline(&mut a.check_in).desired_width(200.0));
                ui.end_row();
                ui.label(ti("Çıkış"));
                ui.add(egui::TextEdit::singleline(&mut a.check_out).desired_width(200.0));
                ui.end_row();
                ui.label(ti("Not"));
                ui.add(egui::TextEdit::singleline(&mut a.note).desired_width(200.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button(ti("Kaydet")).clicked() {
                    save = true;
                    keep = false;
                }
                if ui.button(ti("Vazgeç")).clicked() {
                    keep = false;
                }
            });
        });
        if save {
            let id = a.id;
            self.call(move |api| api.put(&format!("/api/attendance/{id}"), &a), |app, _| {
                app.toast(ti("Mesai kaydı güncellendi."));
                app.load_att();
            });
        } else if keep {
            self.att_edit = Some(a);
        }
    }

    // ------------------------------------------------------------------ hedefler
    fn goal_form_ui(&mut self, ui: &mut Ui) {
        if !self.gform.show {
            if ui.add(egui::Button::new(RichText::new(ti("+ Yeni hedef belirle")).strong().color(on_btn())).fill(btn())).clicked() {
                self.gform.show = true;
            }
            return;
        }
        let staff: Vec<String> = self.users.iter().filter(|u| u.active).map(|u| u.full_name()).collect();
        card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(ti("Yeni hedef")).size(17.0).strong());
            egui::Grid::new("goal_form").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                ui.label(ti("Başlık"));
                ui.add(egui::TextEdit::singleline(&mut self.gform.title).hint_text(ti("Örn: Kasım ayı hedefi")).desired_width(360.0));
                ui.end_row();
                ui.label(ti("Kimin için?"));
                let mut sel = self.users.iter().find(|u| u.id == self.gform.user_id).map(|u| u.full_name()).unwrap_or_default();
                if combo_opt(ui, "goal_user", "Tüm ekip (toplam)", &mut sel, &staff, 220.0) {
                    self.gform.user_id = self.users.iter().find(|u| u.full_name() == sel).map(|u| u.id).unwrap_or(0);
                }
                ui.end_row();
                ui.label(ti("Dönem"));
                ui.horizontal(|ui| {
                    ui.add(DatePickerButton::new(&mut self.gform.start).id_salt("goal_start"));
                    ui.label(ti("–"));
                    ui.add(DatePickerButton::new(&mut self.gform.end).id_salt("goal_end"));
                    if ui.small_button(ti("Bu ay")).clicked() {
                        let today = chrono::Local::now().date_naive();
                        self.gform.start = first_of_month(today);
                        self.gform.end = last_of_month(today);
                    }
                });
                ui.end_row();
                for (i, name) in ["Portföy kaydı (adet)", "Yayına alma (adet)", "Kiralama (adet)", "Satış (adet)", "Web sitesinde yayına alınan ilan (adet, yönetici onaylı)"].iter().enumerate() {
                    ui.label(*name);
                    ui.add(egui::DragValue::new(&mut self.gform.t[i]).range(0..=1000));
                    ui.end_row();
                }
                ui.label(ti("Not"));
                ui.add(egui::TextEdit::singleline(&mut self.gform.note).desired_width(360.0));
                ui.end_row();
            });
            ui.label(RichText::new(ti("0 bıraktığınız kalem hedefe dahil edilmez. İlerleme sistemdeki kayıtlardan otomatik hesaplanır.")).small().color(muted()));
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(ti("Hedefi ata")).strong().color(on_btn())).fill(btn())).clicked() {
                    let g = GoalInfo {
                        title: self.gform.title.trim().to_string(),
                        user_id: self.gform.user_id,
                        period_start: self.gform.start.format("%Y-%m-%d").to_string(),
                        period_end: self.gform.end.format("%Y-%m-%d").to_string(),
                        t_portfolio: self.gform.t[0],
                        t_published: self.gform.t[1],
                        t_rented: self.gform.t[2],
                        t_sold: self.gform.t[3],
                        t_web: self.gform.t[4],
                        note: self.gform.note.trim().to_string(),
                        ..Default::default()
                    };
                    if g.t_portfolio + g.t_published + g.t_rented + g.t_sold + g.t_web == 0 {
                        self.err(ti("En az bir hedef kalemi girin."));
                    } else {
                        self.call(move |api| api.post("/api/goals", &g), |app, _| {
                            app.toast(ti("Hedef atandı."));
                            app.gform.show = false;
                            app.gform.title.clear();
                            app.gform.note.clear();
                            app.load_goals();
                        });
                    }
                }
                if ui.button(ti("Kapat")).clicked() {
                    self.gform.show = false;
                }
            });
        });
    }

    pub fn ui_goals(&mut self, ui: &mut Ui) {
        let admin = self.me.is_admin();
        page_title(ui, if admin { "Hedefler" } else { "Hedeflerim" });
        if admin {
            self.goal_form_ui(ui);
            ui.add_space(6.0);
        }
        if self.goals.is_empty() {
            ui.add_space(14.0);
            ui.label(RichText::new(ti("Henüz atanmış hedef yok.")).color(muted()));
            return;
        }
        let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        let goals = self.goals.clone();
        let mut delete: Option<i64> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for g in &goals {
                let items = [("Portföy kaydı", g.p_portfolio, g.t_portfolio), ("Yayına alma", g.p_published, g.t_published), ("Kiralama", g.p_rented, g.t_rented), ("Satış", g.p_sold, g.t_sold), ("Web yayını", g.p_web, g.t_web)];
                let active: Vec<_> = items.iter().filter(|i| i.2 > 0).collect();
                let pct = if active.is_empty() { 0.0 } else { active.iter().map(|i| (i.1 as f32 / i.2 as f32).min(1.0)).sum::<f32>() / active.len() as f32 };
                let running = g.period_start <= today && today <= g.period_end;
                let finished = g.period_end < today;
                card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&g.title).size(17.0).strong());
                        badge(ui, if g.user_id == 0 { "Ekip hedefi" } else { &g.user_name }, if g.user_id == 0 { Color32::from_rgb(130, 70, 170) } else { accent() });
                        if running {
                            badge(ui, ti("Devam ediyor"), Color32::from_rgb(37, 140, 80));
                        } else if finished {
                            badge(ui, ti("Dönem bitti"), t_dim());
                        } else {
                            badge(ui, ti("Başlamadı"), Color32::from_rgb(200, 120, 20));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if admin && confirm_btn(ui, &format!("gdel{}", g.id), "Sil") {
                                delete = Some(g.id);
                            }
                            ui.label(RichText::new(format!("%{:.0}", pct * 100.0)).size(20.0).strong().color(if pct >= 1.0 { Color32::from_rgb(37, 140, 80) } else { accent() }));
                        });
                    });
                    ui.label(RichText::new(format!("Dönem: {} – {}", date_tr(&g.period_start), date_tr(&g.period_end))).small().color(muted()));
                    if !g.note.is_empty() {
                        ui.label(&g.note);
                    }
                    ui.add_space(4.0);
                    for (name, p, t) in active {
                        ui.horizontal(|ui| {
                            ui.add_sized([120.0, 20.0], egui::Label::new(*name));
                            let f = (*p as f32 / *t as f32).min(1.0);
                            ui.add(egui::ProgressBar::new(f).desired_width(320.0).text(format!("{p} / {t}")).fill(if f >= 1.0 { Color32::from_rgb(37, 140, 80) } else { accent() }));
                        });
                    }
                });
                ui.add_space(4.0);
            }
        });
        if let Some(id) = delete {
            self.call(move |api| api.delete(&format!("/api/goals/{id}")), |app, _| app.load_goals());
        }
    }

    // ------------------------------------------------------------------ bildirimler
    pub fn ui_notifications(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            page_title(ui, "Bildirimler");
            if ui.button(ti("Yenile")).clicked() {
                self.load_notifs(true);
            }
        });
        ui.label(RichText::new(ti("Yeni görevler, toplantılar, hedefler ve ekip arkadaşlarınızın eklediği portföyler burada görünür.")).small().color(muted()));
        ui.separator();
        if self.notifs.is_empty() {
            ui.label(RichText::new(ti("Bildirim yok.")).color(muted()));
            return;
        }
        let list = self.notifs.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for n in &list {
                card_frame(ui.style()).inner_margin(8.0).fill(if n.read { card() } else { new_bg() }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        badge(ui, &n.kind, kind_color(&n.kind));
                        if !n.read {
                            ui.label(RichText::new(ti("YENİ")).small().strong().color(Color32::from_rgb(200, 60, 40)));
                        }
                        ui.label(&n.message);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(dt_tr(&n.created_at)).small().color(muted()));
                        });
                    });
                });
            }
        });
    }
}
