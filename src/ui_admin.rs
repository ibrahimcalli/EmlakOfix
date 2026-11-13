//! Profil, CRM, çalışanlar, ilçe/mahalle yönetimi ve ayarlar ekranları.
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::common::*;
use crate::ui_portfolio::combo_pick;
use eframe::egui::{self, vec2, Align2, Color32, FontId, RichText, Sense, Ui};

fn hex_to_rgb(h: &str) -> [u8; 3] {
    let h = h.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&h[0..2], 16), u8::from_str_radix(&h[2..4], 16), u8::from_str_radix(&h[4..6], 16)) {
            return [r, g, b];
        }
    }
    [31, 78, 121]
}
fn rgb_to_hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn csv_cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\"").replace('\n', " "))
}

impl App {
    /// Yuvarlak profil resmi (yoksa baş harfler).
    pub fn avatar(&mut self, ui: &mut Ui, uid: i64, has_photo: bool, name: &str, size: f32) {
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        let painter = ui.painter_at(rect);
        let tex = if has_photo { self.tex(TexKey::User(uid)) } else { None };
        match tex {
            Some(t) => {
                ui.put(rect, egui::Image::from_texture((t.id(), vec2(size, size))).corner_radius(size / 2.0));
            }
            None => {
                painter.circle_filled(rect.center(), size / 2.0, accent());
                let ini: String = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
                painter.text(rect.center(), Align2::CENTER_CENTER, ini, FontId::proportional(size * 0.4), Color32::WHITE);
            }
        }
    }

    // ------------------------------------------------------------------ profil
    pub fn ui_profile(&mut self, ui: &mut Ui) {
        page_title(ui, "Profilim");
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(760.0);
            card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("Bilgilerim")).size(17.0).strong());
                ui.label(RichText::new(ti("Bu bilgiler, aldığınız PDF/DOCX tanıtım belgelerinin altında iletişim bilgisi olarak yer alır (yönetici hangi alanların görüneceğini Ayarlar'dan belirler).")).small().color(muted()));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (id, has, name) = (self.me.id, self.me.has_photo, self.me.full_name());
                    self.avatar(ui, id, has, &name, 110.0);
                    ui.vertical(|ui| {
                        if ui.button(ti("Fotoğraf seç…")).clicked() {
                            self.pick_files("Profil fotoğrafı", &["jpg", "jpeg", "png", "webp"], false, |app, files| {
                                if let Some(f) = files.into_iter().next() {
                                    let uid = app.me.id;
                                    app.call(
                                        move |api| {
                                            let b = std::fs::read(&f).map_err(|e| e.to_string())?;
                                            api.send_bytes("PUT", "/api/me/photo", &b)?;
                                            api.get::<UserInfo>("/api/me")
                                        },
                                        move |app, u| {
                                            app.tex.remove(&TexKey::User(uid));
                                            app.me = u.clone();
                                            app.prof = u;
                                            app.load_users();
                                            app.toast(ti("Profil fotoğrafı güncellendi."));
                                        },
                                    );
                                }
                            });
                        }
                        if self.me.has_photo && ui.button(ti("Fotoğrafı kaldır")).clicked() {
                            let uid = self.me.id;
                            self.call(
                                |api| {
                                    api.delete("/api/me/photo")?;
                                    api.get::<UserInfo>("/api/me")
                                },
                                move |app, u| {
                                    app.tex.remove(&TexKey::User(uid));
                                    app.me = u.clone();
                                    app.prof = u;
                                    app.load_users();
                                },
                            );
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Grid::new("prof").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Kullanıcı adı"));
                    ui.label(RichText::new(&self.me.username).strong());
                    ui.end_row();
                    ui.label(ti("Ad"));
                    ui.add(egui::TextEdit::singleline(&mut self.prof.first_name).desired_width(260.0));
                    ui.end_row();
                    ui.label(ti("Soyad"));
                    ui.add(egui::TextEdit::singleline(&mut self.prof.last_name).desired_width(260.0));
                    ui.end_row();
                    ui.label(ti("Telefon"));
                    ui.add(egui::TextEdit::singleline(&mut self.prof.phone).hint_text(ti("0532 123 45 67")).desired_width(260.0));
                    ui.end_row();
                    ui.label(ti("E-posta"));
                    ui.add(egui::TextEdit::singleline(&mut self.prof.email).desired_width(260.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new(ti("Bilgilerimi kaydet")).strong().color(on_btn())).fill(btn())).clicked() {
                    let body = self.prof.clone();
                    self.call(
                        move |api| api.put("/api/me", &body).and_then(|v| serde_json::from_value::<UserInfo>(v).map_err(|e| e.to_string())),
                        |app, u| {
                            app.me = u.clone();
                            app.prof = u;
                            app.load_users();
                            app.toast(ti("Bilgileriniz kaydedildi."));
                        },
                    );
                }
            });
            ui.add_space(10.0);
            card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("Şifre değiştir")).size(17.0).strong());
                egui::Grid::new("pw").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Mevcut şifre"));
                    ui.add(egui::TextEdit::singleline(&mut self.pw_old).password(true).desired_width(220.0));
                    ui.end_row();
                    ui.label(ti("Yeni şifre"));
                    ui.add(egui::TextEdit::singleline(&mut self.pw_new).password(true).desired_width(220.0));
                    ui.end_row();
                    ui.label(ti("Yeni şifre (tekrar)"));
                    ui.add(egui::TextEdit::singleline(&mut self.pw_new2).password(true).desired_width(220.0));
                    ui.end_row();
                });
                if ui.button(ti("Şifreyi değiştir")).clicked() {
                    if self.pw_new != self.pw_new2 {
                        self.err(ti("Yeni şifreler aynı değil."));
                    } else {
                        let body = serde_json::json!({"old": self.pw_old, "new": self.pw_new});
                        self.call(move |api| api.put("/api/me/password", &body), |app, _| {
                            app.pw_old.clear();
                            app.pw_new.clear();
                            app.pw_new2.clear();
                            app.toast(ti("Şifreniz değiştirildi."));
                        });
                    }
                }
            });
        });
    }

    // ------------------------------------------------------------------ CRM
    pub fn ui_crm(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            page_title(ui, "CRM — Mülk sahipleri");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(ti("Excel için CSV olarak kaydet")).clicked() {
                    let rows = self.crm.clone();
                    self.save_dialog("crm_kayitlari.csv", "csv", move |app, path| {
                        if let Some(path) = path {
                            let mut out = String::from("\u{feff}Ad Soyad;Telefon;E-posta;Not;İlgili portföyler;Kayıt tarihi;Ekleyen\r\n");
                            for c in &rows {
                                let pf: Vec<String> = c.portfolios.iter().map(|p| p.title.clone()).collect();
                                out.push_str(&[csv_cell(&c.full_name), csv_cell(&c.phone), csv_cell(&c.email), csv_cell(&c.note), csv_cell(&pf.join(" | ")), csv_cell(&date_tr(&c.created_at)), csv_cell(&c.created_by_name)].join(";"));
                                out.push_str("\r\n");
                            }
                            match std::fs::write(&path, out) {
                                Ok(_) => app.toast(format!("Kaydedildi: {}", path.display())),
                                Err(e) => app.err(format!("Yazılamadı: {e}")),
                            }
                        }
                    });
                }
                if ui.button(ti("Yenile")).clicked() {
                    self.load_crm();
                }
            });
        });
        ui.label(RichText::new(ti("Çalışanların portföy kaydederken girdiği mülk sahibi bilgileri burada otomatik listelenir. Bu ekranı yalnızca yönetici görür.")).small().color(muted()));
        ui.add(egui::TextEdit::singleline(&mut self.crm_q).hint_text(ti("Ara: ad, telefon, e-posta, not, portföy…")).desired_width(340.0));
        ui.separator();
        let q = self.crm_q.to_lowercase();
        let rows: Vec<CrmContact> = self
            .crm
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.full_name.to_lowercase().contains(&q)
                    || c.phone.contains(&q)
                    || c.email.to_lowercase().contains(&q)
                    || c.note.to_lowercase().contains(&q)
                    || c.portfolios.iter().any(|p| p.title.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        ui.label(RichText::new(format!("{} kayıt", rows.len())).small().color(muted()));
        let mut open: Option<i64> = None;
        let mut del: Option<i64> = None;
        let mut wa: Option<String> = None;
        let mut kvkk: Option<(i64, u8)> = None;
        let mut kvkk_val = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for c in &rows {
                card_frame(ui.style()).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if c.full_name.is_empty() { "(isimsiz)" } else { &c.full_name }).size(16.0).strong());
                        if !c.phone.is_empty() {
                            ui.label(format!("Tel: {}", c.phone));
                        }
                        if !c.email.is_empty() {
                            ui.label(format!("E-posta: {}", c.email));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if confirm_btn(ui, &format!("cdel{}", c.id), "Sil") {
                                del = Some(c.id);
                            }
                            if ui.button(ti("Düzenle")).clicked() {
                                self.crm_edit = Some(c.clone());
                            }
                            if !c.phone.is_empty() && ui.button(ti("WhatsApp")).clicked() {
                                wa = Some(c.phone.clone());
                            }
                        });
                    });
                    if !c.note.is_empty() {
                        ui.label(RichText::new(&c.note).color(t_mid()));
                    }
                    if !c.portfolios.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(ti("Portföyleri:")).small().color(muted()));
                            for p in &c.portfolios {
                                if ui.link(format!("{} ({})", p.title, p.status)).clicked() {
                                    open = Some(p.id);
                                }
                            }
                        });
                    }
                    ui.label(RichText::new(format!("Kayıt: {} • Ekleyen: {}", date_tr(&c.created_at), c.created_by_name)).small().color(muted()));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ti("KVKK:")).small().strong());
                        if c.anonymized {
                            badge(ui, ti("Anonimleştirildi"), Color32::from_rgb(120, 120, 130));
                        } else {
                            let mut consent = c.consent;
                            if ui.checkbox(&mut consent, ti("Açık rıza alındı")).changed() {
                                kvkk = Some((c.id, 1));
                                kvkk_val = consent;
                            }
                            if c.consent && !c.consent_date.is_empty() {
                                ui.label(RichText::new(format!("({})", date_tr(&c.consent_date))).small().color(muted()));
                            }
                            if ui.small_button(ti("Verileri dışa aktar")).clicked() {
                                kvkk = Some((c.id, 2));
                            }
                            if confirm_btn(ui, &format!("canon{}", c.id), "Anonimleştir") {
                                kvkk = Some((c.id, 3));
                            }
                        }
                    });
                });
                ui.add_space(3.0);
            }
        });
        if let Some((id, act)) = kvkk {
            match act {
                1 => self.call(move |api| api.put(&format!("/api/crm/{id}/consent"), &serde_json::json!({"consent": kvkk_val})), |app, _| app.load_crm()),
                2 => self.call(move |api| api.get::<serde_json::Value>(&format!("/api/crm/{id}/export")), move |app, v| {
                    let txt = serde_json::to_string_pretty(&v).unwrap_or_default();
                    app.save_dialog(&format!("mulk_sahibi_{id}.json"), "json", move |app, p| {
                        if let Some(p) = p {
                            match std::fs::write(&p, txt) {
                                Ok(_) => app.toast(format!("Kaydedildi: {}", p.display())),
                                Err(e) => app.err(format!("Yazılamadı: {e}")),
                            }
                        }
                    });
                }),
                _ => self.call(move |api| api.post(&format!("/api/crm/{id}/anonymize"), &serde_json::json!({})), |app, _| {
                    app.toast(ti("Kayıt anonimleştirildi."));
                    app.load_crm();
                }),
            }
        }
        if let Some(num) = wa {
            self.open_url(&format!("https://wa.me/{}", wa_number(&num)));
        }
        if let Some(id) = open {
            self.open_detail(id);
        }
        if let Some(id) = del {
            self.call(move |api| api.delete(&format!("/api/crm/{id}")), |app, _| app.load_crm());
        }
        self.crm_edit_window(ui.ctx());
    }

    fn crm_edit_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.crm_edit.take() else { return };
        let mut keep = true;
        let mut save = false;
        egui::Window::new("Mülk sahibi kaydını düzenle").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            egui::Grid::new("crm_edit").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(ti("Ad Soyad"));
                ui.add(egui::TextEdit::singleline(&mut c.full_name).desired_width(280.0));
                ui.end_row();
                ui.label(ti("Telefon"));
                ui.add(egui::TextEdit::singleline(&mut c.phone).desired_width(280.0));
                ui.end_row();
                ui.label(ti("E-posta"));
                ui.add(egui::TextEdit::singleline(&mut c.email).desired_width(280.0));
                ui.end_row();
                ui.label(ti("Not"));
                ui.add(egui::TextEdit::multiline(&mut c.note).desired_rows(3).desired_width(280.0));
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
            let id = c.id;
            self.call(move |api| api.put(&format!("/api/crm/{id}"), &c), |app, _| {
                app.toast(ti("Kayıt güncellendi."));
                app.load_crm();
            });
        } else if keep {
            self.crm_edit = Some(c);
        }
    }

    // ------------------------------------------------------------------ çalışanlar
    pub fn ui_users(&mut self, ui: &mut Ui) {
        page_title(ui, "Çalışanlar");
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            card_frame(ui.style()).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("Yeni çalışan ekle")).size(17.0).strong());
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.new_user.username).hint_text(ti("kullanıcı adı")).desired_width(130.0));
                    ui.add(egui::TextEdit::singleline(&mut self.new_user.password).hint_text(ti("şifre (min. 6)")).desired_width(130.0));
                    ui.add(egui::TextEdit::singleline(&mut self.new_user.first).hint_text(ti("ad")).desired_width(110.0));
                    ui.add(egui::TextEdit::singleline(&mut self.new_user.last).hint_text(ti("soyad")).desired_width(110.0));
                    ui.checkbox(&mut self.new_user.admin, ti("Yönetici"));
                    if ui.add(egui::Button::new(RichText::new(ti("Ekle")).color(on_btn())).fill(btn())).clicked() {
                        let n = &self.new_user;
                        let body = serde_json::json!({"username": n.username.trim(), "password": n.password, "first_name": n.first.trim(), "last_name": n.last.trim(), "role": if n.admin {"admin"} else {"employee"}});
                        self.call(move |api| api.post("/api/users", &body), |app, _| {
                            app.new_user = NewUser::default();
                            app.toast(ti("Çalışan eklendi. Kullanıcı adı ve şifresini kendisine iletin."));
                            app.load_users();
                        });
                    }
                });
            });
            ui.add_space(8.0);
            let users = self.users.clone();
            let mut reset: Option<(i64, String)> = None;
            let mut edit: Option<UserInfo> = None;
            egui::Grid::new("users_grid").striped(true).num_columns(6).spacing([18.0, 8.0]).show(ui, |ui| {
                for h in ["", "Kullanıcı adı", "Ad Soyad", "Telefon / E-posta", "Yetki", "İşlem"] {
                    ui.label(RichText::new(h).strong().color(muted()));
                }
                ui.end_row();
                for u in &users {
                    self.avatar(ui, u.id, u.has_photo, &u.full_name(), 30.0);
                    let c = if u.active { t_strong() } else { t_dim() };
                    ui.label(RichText::new(&u.username).color(c));
                    ui.label(RichText::new(u.full_name()).color(c));
                    ui.label(RichText::new(format!("{} {}", u.phone, u.email)).color(c));
                    ui.label(RichText::new(format!("{}{}", if u.is_admin() { "Yönetici" } else { "Çalışan" }, if u.active { "" } else { " (pasif)" })).color(c));
                    ui.horizontal(|ui| {
                        if ui.small_button(ti("Düzenle")).clicked() {
                            edit = Some(u.clone());
                        }
                        if ui.small_button(ti("Şifre sıfırla")).clicked() {
                            reset = Some((u.id, String::new()));
                        }
                    });
                    ui.end_row();
                }
            });
            if let Some(u) = edit {
                self.edit_user = Some(u);
            }
            if let Some(r) = reset {
                self.reset_pw = Some(r);
            }
            ui.add_space(6.0);
            ui.label(RichText::new(ti("Çalışanlar silinmez, \"pasif\" yapılır; böylece geçmiş portföy ve mesai kayıtları korunur. Pasif kullanıcı programa giremez.")).small().color(muted()));
        });
        self.user_windows(ui.ctx());
    }

    fn user_windows(&mut self, ctx: &egui::Context) {
        if let Some(mut u) = self.edit_user.take() {
            let mut keep = true;
            let mut save = false;
            egui::Window::new(format!("Çalışanı düzenle — {}", u.username)).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                egui::Grid::new("user_edit").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Ad"));
                    ui.add(egui::TextEdit::singleline(&mut u.first_name).desired_width(200.0));
                    ui.end_row();
                    ui.label(ti("Soyad"));
                    ui.add(egui::TextEdit::singleline(&mut u.last_name).desired_width(200.0));
                    ui.end_row();
                    ui.label(ti("Yetki"));
                    let mut r = if u.is_admin() { "Yönetici".to_string() } else if u.role == "manager" { "Ekip lideri".to_string() } else { "Çalışan".to_string() };
                    if combo_pick(ui, "user_role", &mut r, &["Çalışan".to_string(), "Ekip lideri".to_string(), "Yönetici".to_string()], 140.0) {
                        u.role = match r.as_str() { "Yönetici" => "admin", "Ekip lideri" => "manager", _ => "employee" }.into();
                    }
                    ui.end_row();
                    ui.label(ti("Durum"));
                    ui.checkbox(&mut u.active, ti("Aktif (programa girebilir)"));
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
                let id = u.id;
                self.call(move |api| api.put(&format!("/api/users/{id}"), &u), |app, _| {
                    app.toast(ti("Çalışan güncellendi."));
                    app.load_users();
                });
            } else if keep {
                self.edit_user = Some(u);
            }
        }
        if let Some((id, mut pw)) = self.reset_pw.take() {
            let mut keep = true;
            let mut save = false;
            let name = self.users.iter().find(|u| u.id == id).map(|u| u.full_name()).unwrap_or_default();
            egui::Window::new(format!("Şifre sıfırla — {name}")).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label(ti("Yeni şifre (en az 6 karakter):"));
                ui.add(egui::TextEdit::singleline(&mut pw).desired_width(220.0));
                ui.label(RichText::new(ti("Kullanıcı açık oturumlardan çıkarılır; yeni şifreyle girer.")).small().color(muted()));
                ui.horizontal(|ui| {
                    if ui.button(ti("Sıfırla")).clicked() {
                        save = true;
                        keep = false;
                    }
                    if ui.button(ti("Vazgeç")).clicked() {
                        keep = false;
                    }
                });
            });
            if save {
                let body = serde_json::json!({"password": pw});
                self.call(move |api| api.post(&format!("/api/users/{id}/reset_password"), &body), |app, _| app.toast(ti("Şifre sıfırlandı.")));
            } else if keep {
                self.reset_pw = Some((id, pw));
            }
        }
    }

    // ------------------------------------------------------------------ ilçe / mahalle
    pub fn ui_locations(&mut self, ui: &mut Ui) {
        page_title(ui, "İlçe / Mahalle listesi");
        ui.label(RichText::new(ti("Muğla'nın ilçe ve mahalleleri hazır gelir. Eksik veya yeni bir mahalle varsa buradan ekleyebilirsiniz. Silme işlemi mevcut portföyleri etkilemez.")).small().color(muted()));
        if self.loc_sel == 0 || !self.locations.iter().any(|d| d.id == self.loc_sel) {
            self.loc_sel = self.locations.first().map(|d| d.id).unwrap_or(0);
        }
        let locs = self.locations.clone();
        let mut del_d: Option<i64> = None;
        let mut del_n: Option<i64> = None;
        ui.separator();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(250.0);
                ui.label(RichText::new(ti("İlçeler")).strong());
                egui::ScrollArea::vertical().id_salt("loc_d").max_height(ui.available_height() - 70.0).show(ui, |ui| {
                    for d in &locs {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(self.loc_sel == d.id, format!("{} ({})", d.name, d.neighborhoods.len())).clicked() {
                                self.loc_sel = d.id;
                            }
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.new_district).hint_text(ti("yeni ilçe adı")).desired_width(150.0));
                    if ui.button(ti("Ekle")).clicked() && !self.new_district.trim().is_empty() {
                        let body = serde_json::json!({"name": self.new_district.trim()});
                        self.call(move |api| api.post("/api/locations/district", &body), |app, _| {
                            app.new_district.clear();
                            app.load_locations();
                        });
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                if let Some(d) = locs.iter().find(|d| d.id == self.loc_sel) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} — mahalleler ({})", d.name, d.neighborhoods.len())).size(17.0).strong());
                        if confirm_btn(ui, &format!("ddel{}", d.id), "İlçeyi sil") {
                            del_d = Some(d.id);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.new_neigh).hint_text(ti("yeni mahalle / köy adı")).desired_width(220.0));
                        if ui.button(ti("Mahalle ekle")).clicked() && !self.new_neigh.trim().is_empty() {
                            let body = serde_json::json!({"district_id": d.id, "name": self.new_neigh.trim()});
                            self.call(move |api| api.post("/api/locations/neighborhood", &body), |app, _| {
                                app.new_neigh.clear();
                                app.load_locations();
                            });
                        }
                    });
                    egui::ScrollArea::vertical().id_salt("loc_n").show(ui, |ui| {
                        egui::Grid::new("neigh_grid").num_columns(2).spacing([24.0, 4.0]).striped(true).show(ui, |ui| {
                            for n in &d.neighborhoods {
                                ui.label(&n.name);
                                if confirm_btn(ui, &format!("ndel{}", n.id), "Sil") {
                                    del_n = Some(n.id);
                                }
                                ui.end_row();
                            }
                        });
                    });
                }
            });
        });
        if let Some(id) = del_d {
            self.call(move |api| api.delete(&format!("/api/locations/district/{id}")), |app, _| app.load_locations());
        }
        if let Some(id) = del_n {
            self.call(move |api| api.delete(&format!("/api/locations/neighborhood/{id}")), |app, _| app.load_locations());
        }
    }

    // ------------------------------------------------------------------ ayarlar
    pub fn settings_output(&mut self, ui: &mut Ui) {
        {
            card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(ti("Çıktı (PDF / DOCX / WhatsApp) ayarları")).size(17.0).strong());
                ui.label(RichText::new(ti("Çalışanların aldığı tanıtım belgelerinde nelerin görüneceğini buradan belirlersiniz.")).small().color(muted()));
                ui.add_space(6.0);
                let s = &mut self.set_edit;
                egui::Grid::new("set_text").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    ui.label(ti("Ofis adı"));
                    ui.add(egui::TextEdit::singleline(&mut s.office_name).desired_width(360.0));
                    ui.end_row();
                    ui.label(ti("Slogan"));
                    ui.add(egui::TextEdit::singleline(&mut s.tagline).desired_width(360.0));
                    ui.end_row();
                    ui.label(ti("Ofis telefonu"));
                    ui.add(egui::TextEdit::singleline(&mut s.office_phone).desired_width(360.0));
                    ui.end_row();
                    ui.label(ti("Ofis adresi"));
                    ui.add(egui::TextEdit::singleline(&mut s.office_address).desired_width(360.0));
                    ui.end_row();
                    ui.label(ti("Alt bilgi notu"));
                    ui.add(egui::TextEdit::multiline(&mut s.footer_note).desired_rows(2).desired_width(360.0));
                    ui.end_row();
                    ui.label(ti("Vurgu rengi"));
                    ui.horizontal(|ui| {
                        let mut rgb = hex_to_rgb(&s.accent);
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            s.accent = rgb_to_hex(rgb);
                        }
                        ui.label(&s.accent);
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.label(RichText::new(ti("Belgede gösterilecekler")).strong());
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut s.show_logo, ti("Logo"));
                    ui.checkbox(&mut s.show_office_contact, ti("Ofis iletişim bilgileri"));
                    ui.checkbox(&mut s.show_listing_no, ti("Portföy no"));
                    ui.checkbox(&mut s.show_date, ti("Tarih"));
                    ui.checkbox(&mut s.show_price, ti("Fiyat"));
                    ui.checkbox(&mut s.show_address, ti("Açık adres"));
                    ui.checkbox(&mut s.show_map_link, ti("Harita bağlantısı"));
                    ui.checkbox(&mut s.show_details, ti("Özellik tablosu"));
                    ui.checkbox(&mut s.show_description, ti("Açıklama"));
                    ui.checkbox(&mut s.show_photos, ti("Fotoğraflar"));
                });
                ui.horizontal(|ui| {
                    ui.label(ti("En fazla fotoğraf sayısı:"));
                    ui.add(egui::DragValue::new(&mut s.max_photos).range(1..=24));
                });
                ui.label(RichText::new(ti("Belge altındaki çalışan (danışman) bilgileri")).strong());
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut s.show_agent_photo, ti("Profil fotoğrafı"));
                    ui.checkbox(&mut s.show_agent_name, ti("Ad Soyad"));
                    ui.checkbox(&mut s.show_agent_phone, ti("Telefon"));
                    ui.checkbox(&mut s.show_agent_email, ti("E-posta"));
                });
                ui.label(RichText::new(ti("Mülk sahibi (CRM) bilgileri hiçbir çıktıda yer almaz.")).small().color(muted()));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(ti("Logo:"));
                    if let Some(t) = self.tex(TexKey::Logo) {
                        let sz = t.size_vec2();
                        let k = (120.0 / sz.x).min(60.0 / sz.y).min(1.0);
                        ui.image((t.id(), sz * k));
                    } else {
                        ui.label(RichText::new(ti("yok")).color(muted()));
                    }
                    if ui.button(ti("Logo seç…")).clicked() {
                        self.pick_files("Logo", &["png", "jpg", "jpeg", "webp"], false, |app, files| {
                            if let Some(f) = files.into_iter().next() {
                                app.call(
                                    move |api| {
                                        let b = std::fs::read(&f).map_err(|e| e.to_string())?;
                                        api.send_bytes("PUT", "/api/settings/logo", &b)
                                    },
                                    |app, _| {
                                        app.tex.remove(&TexKey::Logo);
                                        app.toast(ti("Logo yüklendi."));
                                    },
                                );
                            }
                        });
                    }
                    if ui.button(ti("Logoyu kaldır")).clicked() {
                        self.call(|api| api.delete("/api/settings/logo"), |app, _| {
                            app.tex.remove(&TexKey::Logo);
                        });
                    }
                });
                ui.add_space(8.0);
                if ui.add(egui::Button::new(RichText::new(ti("Ayarları kaydet")).strong().color(on_btn())).fill(btn())).clicked() {
                    let body = self.set_edit.clone();
                    self.call(move |api| api.put("/api/settings", &body), |app, _| {
                        app.toast(ti("Ayarlar kaydedildi."));
                        app.load_settings();
                    });
                }
            });
        }
    }

    pub fn server_panel(&mut self, ui: &mut Ui) {
        card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(ti("Ana bilgisayar ve yedekleme")).size(17.0).strong());
            let Some(si) = self.server_info.clone() else {
                ui.label(RichText::new(ti("Yükleniyor…")).color(muted()));
                return;
            };
            ui.label(format!("Veri klasörü (ana bilgisayarda): {}", si.data_dir));
            ui.label(format!("Veritabanı boyutu: {:.1} MB", si.db_size_mb));
            ui.label(RichText::new(ti("Diğer bilgisayarlar şu adreslerden birine bağlanır:")).color(muted()));
            for a in &si.addresses {
                ui.label(RichText::new(a).strong().size(16.0));
            }
            if si.addresses.is_empty() {
                ui.label(RichText::new(ti("(Ağ adresi bulunamadı — ağa bağlı olduğunuzdan emin olun.)")).color(muted()));
            }
            ui.label(RichText::new(format!("Güvenlik duvarında {} numaralı TCP bağlantı noktasına (ve otomatik bulma için 47862 UDP) izin verilmelidir; ayrıntı KURULUM.docx dosyasındadır.", si.port)).small().color(muted()));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(ti("Şimdi yedek al")).clicked() {
                    self.call(|api| api.post("/api/backup", &serde_json::json!({})), |app, v| {
                        app.toast(format!("Yedek alındı: {}", v["path"].as_str().unwrap_or("")));
                        app.load_server_info();
                    });
                }
                ui.label(RichText::new(ti("Program açılırken ve her 6 saatte bir otomatik yedek alınır (son 30 yedek saklanır).")).small().color(muted()));
            });
            if !si.backups.is_empty() {
                ui.label(RichText::new(ti("Son yedekler:")).small().color(muted()));
                for b in si.backups.iter().take(6) {
                    ui.label(RichText::new(b).small());
                }
            }
        });
    }
}
