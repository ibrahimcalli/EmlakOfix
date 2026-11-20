//! Karşılama (sunucu/istemci seçimi) ve giriş ekranları.
use crate::api;
use crate::app::*;
use crate::theme::*;
use crate::i18n_ui::ti;
use crate::common::*;
use eframe::egui::{self, Color32, RichText};

impl App {
    pub fn ui_welcome(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            self.draw_backdrop(ui);
            ui.add_space((ui.available_height() * 0.08).min(60.0));
            ui.vertical_centered(|ui| {
                ui.set_max_width(560.0);
                if let Some(t) = self.logo_handle() {
                    let h = 84.0;
                    ui.image((t.id(), egui::vec2(h * t.size_vec2().x / t.size_vec2().y, h)));
                }
                ui.label(RichText::new(APP_NAME).size(34.0).strong().color(Color32::from_rgb(240, 244, 252)));
                ui.label(RichText::new(ti("Portföy • CRM • Mesai • Görev yönetimi")).color(Color32::from_rgb(212, 175, 55)));
                ui.add_space(14.0);
                card_frame(ui.style()).inner_margin(22.0).show(ui, |ui| {
                    ui.set_width(520.0);
                    ui.label(RichText::new(ti("Bu bilgisayar nasıl çalışsın?")).size(18.0).strong());
                    ui.label(
                        RichText::new(ti("Ofisteki bilgisayarların verileri ortak kullanabilmesi için bilgisayarlardan BİRİ ana bilgisayar (sunucu) olur, diğerleri ona bağlanır."))
                            .color(muted()),
                    );
                    ui.add_space(10.0);
                    ui.radio_value(&mut self.w_mode, 0, RichText::new(ti("Bu bilgisayar ANA BİLGİSAYAR olsun (veriler burada tutulur)")).strong());
                    if self.w_mode == 0 {
                        ui.indent("srv", |ui| {
                            ui.horizontal(|ui| {
                                ui.label(ti("Bağlantı noktası (port):"));
                                ui.add(egui::TextEdit::singleline(&mut self.w_port).desired_width(70.0));
                            });
                            let dir = if self.cfg.data_dir.is_empty() { default_data_dir() } else { std::path::PathBuf::from(&self.cfg.data_dir) };
                            ui.label(RichText::new(format!("Veri klasörü: {}", dir.display())).small().color(muted()));
                            ui.label(RichText::new(ti("Bu bilgisayar ofis saatlerinde açık kalmalıdır.")).small().color(muted()));
                        });
                    }
                    ui.add_space(6.0);
                    ui.radio_value(&mut self.w_mode, 1, RichText::new(ti("Bu bilgisayar ofisteki ana bilgisayara BAĞLANSIN")).strong());
                    if self.w_mode == 1 {
                        ui.indent("cli", |ui| {
                            ui.horizontal(|ui| {
                                ui.label(ti("Ana bilgisayar adresi:"));
                                ui.add(egui::TextEdit::singleline(&mut self.w_addr).hint_text(ti("192.168.1.20:7878")).desired_width(190.0));
                                let label = if self.searching { "Aranıyor…" } else { "Ağda otomatik ara" };
                                if ui.add_enabled(!self.searching, egui::Button::new(label)).clicked() {
                                    self.searching = true;
                                    self.found.clear();
                                    self.spawn(api::discover, |app, v| {
                                        app.searching = false;
                                        if v.is_empty() {
                                            app.connect_msg = "Ağda ana bilgisayar bulunamadı. Adresi elle yazın (ana bilgisayardaki \"Bağlantı adresi\").".into();
                                        } else {
                                            app.connect_msg.clear();
                                            if app.w_addr.trim().is_empty() {
                                                app.w_addr = v[0].clone();
                                            }
                                            app.found = v;
                                        }
                                    });
                                }
                            });
                            let found = self.found.clone();
                            for f in found {
                                if ui.link(format!("Bulundu: {f}  (seç)")).clicked() {
                                    self.w_addr = f;
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    let need = (self.w_mode == 0 && !self.cfg.host_ok) || (self.w_mode == 1 && !self.cfg.client_ok && !self.cfg.host_ok);
                    if need && !crate::sign::host_key(&self.cfg.trusted_key).is_empty() {
                        ui.label(RichText::new(ti("Kurulum kodu (Ayarlar > Kurulum ve lisans kodları bölümünden yönetici üretir):")).strong());
                        ui.add(egui::TextEdit::singleline(&mut self.w_code).hint_text("EO-…").desired_width(f32::INFINITY));
                        ui.label(RichText::new(if self.w_mode == 0 { ti("Ana bilgisayar için YÖNETİCİ kodu gerekir. Kodsuz kurulum yapılamaz.") } else { ti("Bağlanmak için KULLANICI (veya yönetici) kodu gerekir. Kodsuz kurulum yapılamaz.") }).small().color(muted()));
                    }
                    ui.add_space(4.0);
                    if !self.connect_msg.is_empty() {
                        ui.colored_label(Color32::from_rgb(190, 40, 40), &self.connect_msg);
                    }
                    if ui.add_sized([200.0, 34.0], egui::Button::new(RichText::new(ti("Devam")).size(16.0).strong())).clicked() {
                        self.apply_welcome();
                    }
                });
            });
        });
    }

    fn apply_welcome(&mut self) {
        let port: u16 = match self.w_port.trim().parse::<u16>() {
            Ok(p) if p >= 1024 => p,
            _ => {
                self.connect_msg = "Port 1024 ile 65535 arasında bir sayı olmalı (öneri: 7878).".into();
                return;
            }
        };
        if self.w_mode == 1 {
            if self.w_addr.trim().is_empty() {
                self.connect_msg = "Ana bilgisayarın adresini yazın.".into();
                return;
            }
            if let Err(e) = self.activate_code(false) {
                self.connect_msg = e;
                return;
            }
            self.cfg.mode = "client".into();
            self.cfg.server_addr = self.w_addr.trim().to_string();
        } else {
            if let Err(e) = self.activate_code(true) {
                self.connect_msg = e;
                return;
            }
            self.cfg.mode = "server".into();
            self.cfg.port = port;
        }
        self.w_code.clear();
        self.cfg.save();
        self.connect();
    }

    /// Kodu doğrular ve etkinleştirir. host=true: ana bilgisayar (yalnızca yönetici kodu).
    pub fn activate_code(&mut self, host: bool) -> Result<(), String> {
        let key = crate::sign::host_key(&self.cfg.trusted_key);
        let already = if host { self.cfg.host_ok } else { self.cfg.client_ok || self.cfg.host_ok };
        if key.is_empty() || already {
            return Ok(());
        }
        let info = crate::sign::verify_code(&key, &self.w_code)?;
        if host && info.kind != 'H' {
            return Err("Bu bir KULLANICI kodu; ana bilgisayar için YÖNETİCİ kodu gerekir.".into());
        }
        if host { self.cfg.host_ok = true } else { self.cfg.client_ok = true }
        self.cfg.lic_kind = info.kind.to_string();
        self.cfg.lic_until = if info.license_until == crate::sign::NO_LIMIT { String::new() } else { info.license_until };
        Ok(())
    }

    /// Lisans süresi dolunca gösterilen kilit ekranı: yeni kod girilince açılır.
    pub fn ui_license_locked(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(520.0);
                ui.label(RichText::new(ti("Lisans süresi doldu")).size(26.0).strong());
                ui.label(RichText::new(format!("{} {}", ti("Bu kurulumun lisans bitiş tarihi:"), self.cfg.lic_until)).color(muted()));
                ui.add_space(8.0);
                ui.label(ti("Devam etmek için yöneticiden yeni bir kod alıp aşağıya yapıştırın. Verileriniz silinmez; kod girilince kaldığınız yerden devam edersiniz."));
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut self.lic_code).hint_text("EO-…").desired_width(f32::INFINITY));
                if !self.lic_msg.is_empty() {
                    ui.colored_label(Color32::from_rgb(190, 40, 40), &self.lic_msg);
                }
                if ui.add_sized([200.0, 32.0], egui::Button::new(ti("Kodu uygula"))).clicked() {
                    let key = crate::sign::host_key(&self.cfg.trusted_key);
                    match crate::sign::verify_code(&key, &self.lic_code) {
                        Ok(i) if i.kind == 'H' || self.cfg.mode != "server" => {
                            self.cfg.lic_until = if i.license_until == crate::sign::NO_LIMIT { String::new() } else { i.license_until };
                            self.cfg.lic_kind = i.kind.to_string();
                            self.cfg.save();
                            self.lic_code.clear();
                            self.lic_msg.clear();
                        }
                        Ok(_) => self.lic_msg = "Ana bilgisayar için YÖNETİCİ kodu gerekir.".into(),
                        Err(e) => self.lic_msg = e,
                    }
                }
            });
        });
    }

    pub fn ui_login(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            self.draw_backdrop(ui);
            let full = ui.max_rect();
            let wide = full.width() >= 980.0;
            let (brand_rect, card_rect) = if wide {
                let bw = (full.width() * 0.44).clamp(430.0, 640.0);
                (
                    Some(egui::Rect::from_min_max(full.min + egui::vec2(56.0, 0.0), egui::pos2(full.min.x + bw, full.max.y))),
                    egui::Rect::from_min_max(egui::pos2(full.min.x + bw, full.min.y), full.max),
                )
            } else {
                (None, full)
            };
            if let Some(br) = brand_rect {
                let mut c = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::top_down(egui::Align::Min)));
                c.add_space((br.height() * 0.5 - 250.0).max(24.0));
                self.brand_panel(&mut c);
            }
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(card_rect).layout(egui::Layout::top_down(egui::Align::Center)));
            c.add_space((card_rect.height() * 0.60 - 120.0).max(20.0));
            if !wide {
                if let Some(t) = self.logo_handle() {
                    let h = 80.0;
                    c.image((t.id(), egui::vec2(h * t.size_vec2().x / t.size_vec2().y, h)));
                }
                c.label(RichText::new(self.settings.office_name.clone()).size(28.0).strong().color(Color32::from_rgb(240, 244, 252)));
                c.add_space(8.0);
            }
            self.login_card(&mut c);
            c.add_space(10.0);
            if !wide {
                c.hyperlink_to(RichText::new(crate::ui_brand::SITE.trim_start_matches("https://")).color(Color32::from_rgb(212, 175, 55)), crate::ui_brand::SITE);
                c.label(RichText::new(format!("☎ {}   ✉ {}", crate::ui_brand::PHONE, crate::ui_brand::MAIL)).small().color(Color32::from_rgb(200, 210, 228)));
            }
            let foot = Color32::from_rgb(190, 202, 224);
            if self.server_state.is_some() {
                let ips = crate::server::local_ips();
                c.label(
                    RichText::new(format!(
                        "Bu bilgisayar ana bilgisayardır. Diğer bilgisayarlar şu adresle bağlanır: {}",
                        if ips.is_empty() { format!("(IP bulunamadı):{}", self.cfg.port_or_default()) } else { format!("{}:{}", ips[0], self.cfg.port_or_default()) }
                    ))
                    .small()
                    .color(foot),
                );
            } else if !self.cfg.server_addr.is_empty() {
                c.horizontal(|ui| {
                    ui.add_space((card_rect.width() - 400.0).max(0.0) / 2.0);
                    ui.label(RichText::new(format!("{} {}", ti("Ana bilgisayar:"), self.cfg.server_addr)).small().color(foot));
                    if ui.small_button(ti("değiştir")).clicked() {
                        self.stage = Stage::Welcome;
                    }
                });
            }
            let r = egui::Rect::from_min_max(egui::pos2(full.min.x, full.max.y - 24.0), full.max);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format!("{} v{} · © {}", APP_NAME, crate::updater::current_version(), self.settings.office_name), egui::FontId::proportional(12.0), Color32::from_rgba_unmultiplied(220, 228, 244, 150));
        });
    }

    fn login_card(&mut self, ui: &mut egui::Ui) {
        // Giriş kartı seçili temadan bağımsızdır: arka plan resminin üzerinde her zaman koyu, yarı saydam kart.
        let fill = Color32::from_rgba_unmultiplied(12, 22, 40, 226);
        let (t_strong, muted) = (Color32::from_rgb(242, 245, 251), Color32::from_rgb(165, 177, 200));
        {
            let st = ui.style_mut();
            st.visuals = egui::Visuals::dark();
            st.visuals.override_text_color = Some(Color32::from_rgb(226, 231, 242));
            st.visuals.extreme_bg_color = Color32::from_rgb(24, 33, 52);
            st.visuals.widgets.inactive.bg_fill = Color32::from_rgb(58, 66, 84);
            st.visuals.widgets.hovered.bg_fill = Color32::from_rgb(74, 84, 106);
        }
        egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(212, 175, 55, 140)))
            .corner_radius(14)
            .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 2, color: Color32::from_black_alpha(110) })
            .inner_margin(26.0)
            .show(ui, |ui| {
              ui.set_width(380.0);
              ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                if self.offline {
                    ui.colored_label(Color32::from_rgb(220, 70, 70), RichText::new(ti("Ana bilgisayara ulaşılamıyor.")).strong());
                    ui.label(RichText::new(ti("Ana bilgisayarın açık ve programın çalışıyor olduğundan, aynı ağda olduğunuzdan emin olun.")).color(muted));
                    ui.horizontal(|ui| {
                        if ui.button(ti("Yeniden dene")).clicked() {
                            self.ping();
                        }
                        if self.server_state.is_none() && ui.button(ti("Bağlantı ayarları")).clicked() {
                            self.stage = Stage::Welcome;
                        }
                    });
                    if !self.connect_msg.is_empty() {
                        ui.label(RichText::new(&self.connect_msg).small().color(muted));
                    }
                } else if self.needs_setup {
                    self.ui_first_setup(ui);
                } else {
                    ui.label(RichText::new(ti("Hoş geldiniz")).size(24.0).strong().color(t_strong));
                    ui.label(RichText::new(ti("Devam etmek için giriş yapın")).color(muted));
                    ui.add_space(10.0);
                    ui.label(RichText::new(ti("Kullanıcı adı")).strong());
                    let r1 = ui.add_sized([ui.available_width(), 32.0], egui::TextEdit::singleline(&mut self.l_user).margin(egui::vec2(8.0, 6.0)));
                    ui.add_space(4.0);
                    ui.label(RichText::new(ti("Şifre")).strong());
                    let r2 = ui.add_sized([ui.available_width(), 32.0], egui::TextEdit::singleline(&mut self.l_pass).password(true).margin(egui::vec2(8.0, 6.0)));
                    if self.l_user.is_empty() && !r2.has_focus() {
                        r1.request_focus();
                    }
                    let mut r3_enter = false;
                    if self.need_code {
                        ui.add_space(4.0);
                        ui.label(RichText::new(ti("Doğrulama kodu")).strong());
                        let r3 = ui.add_sized([ui.available_width(), 32.0], egui::TextEdit::singleline(&mut self.l_code).hint_text(ti("123456 veya kurtarma kodu")).margin(egui::vec2(8.0, 6.0)));
                        r3.request_focus();
                        r3_enter = r3.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    }
                    let enter = r3_enter || (r1.lost_focus() || r2.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.add_space(10.0);
                    if !self.connect_msg.is_empty() {
                        ui.colored_label(Color32::from_rgb(220, 70, 70), &self.connect_msg);
                    }
                    if ui.add_sized([ui.available_width(), 38.0], egui::Button::new(RichText::new(ti("Giriş yap")).size(16.0).strong().color(on_btn())).fill(btn())).clicked() || enter {
                        self.do_login();
                    }
                }
              });
            });
    }

    fn ui_first_setup(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(ti("İlk kurulum")).size(20.0).strong());
        ui.label(RichText::new(ti("Henüz hiç kullanıcı yok. Yönetici (admin) hesabını oluşturun; diğer çalışanları sonra Çalışanlar ekranından eklersiniz.")).color(muted()));
        ui.add_space(6.0);
        ui.label(ti("Yönetici kullanıcı adı (en az 3 harf)"));
        ui.add(egui::TextEdit::singleline(&mut self.l_user).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(ti("Ad"));
                ui.add(egui::TextEdit::singleline(&mut self.su_first).desired_width(170.0));
            });
            ui.vertical(|ui| {
                ui.label(ti("Soyad"));
                ui.add(egui::TextEdit::singleline(&mut self.su_last).desired_width(170.0));
            });
        });
        ui.label(ti("Şifre (en az 6 karakter)"));
        ui.add(egui::TextEdit::singleline(&mut self.l_pass).password(true).desired_width(f32::INFINITY));
        ui.label(ti("Şifre (tekrar)"));
        ui.add(egui::TextEdit::singleline(&mut self.su_pass2).password(true).desired_width(f32::INFINITY));
        ui.add_space(8.0);
        if !self.connect_msg.is_empty() {
            ui.colored_label(Color32::from_rgb(190, 40, 40), &self.connect_msg);
        }
        if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new(ti("Hesabı oluştur ve giriş yap")).strong())).clicked() {
            if self.l_pass != self.su_pass2 {
                self.connect_msg = "Şifreler aynı değil.".into();
                return;
            }
            let body = serde_json::json!({"username": self.l_user.trim(), "password": self.l_pass, "first_name": self.su_first.trim(), "last_name": self.su_last.trim()});
            self.connect_msg.clear();
            self.bg(
                move |api| api.post("/api/setup", &body),
                |app, r| match r {
                    Ok(_) => {
                        app.needs_setup = false;
                        app.su_pass2.clear();
                        app.do_login();
                    }
                    Err(e) => app.connect_msg = e,
                },
            );
        }
    }
}
