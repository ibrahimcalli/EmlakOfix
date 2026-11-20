//! Profilim > İki aşamalı doğrulama kartı.
use crate::app::*;
use crate::i18n_ui::ti;
use crate::theme::*;
use chrono::Datelike;
use eframe::egui::{self, RichText, Ui};

impl App {
    pub fn totp_card(&mut self, ui: &mut Ui) {
        if !self.totp.loaded {
            self.totp.loaded = true;
            self.call(|api| api.get::<serde_json::Value>("/api/me/totp"), |app, v| {
                app.totp.enabled = v["enabled"].as_bool().unwrap_or(false);
                app.totp.left = v["recovery_left"].as_u64().unwrap_or(0) as usize;
            });
        }
        card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(ti("İki aşamalı doğrulama")).size(17.0).strong());
            ui.label(RichText::new(ti("Şifreye ek olarak telefondaki doğrulama uygulamasından (Google Authenticator, Microsoft Authenticator, Aegis…) 6 haneli kod istenir. Ofis dışından erişimde önerilir.")).small().color(muted()));
            ui.add_space(6.0);
            if self.totp.enabled {
                ui.colored_label(egui::Color32::from_rgb(34, 139, 70), ti("✔ Açık"));
                if !self.totp.codes.is_empty() {
                    egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(212, 175, 55, 28)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(212, 175, 55))).corner_radius(8).inner_margin(10.0).show(ui, |ui| {
                        ui.label(RichText::new(ti("Kurtarma kodlarınız — yalnızca şimdi gösterilir")).strong());
                        ui.label(RichText::new(ti("Telefonunuzu kaybederseniz giriş ekranındaki kod kutusuna bunlardan birini yazarak girebilirsiniz. Her kod bir kez kullanılır. Yazdırıp güvenli bir yerde saklayın; kimseyle paylaşmayın.")).small());
                        ui.add_space(4.0);
                        for pair in self.totp.codes.chunks(2) {
                            ui.label(RichText::new(pair.join("      ")).monospace().size(16.0).strong());
                        }
                        ui.horizontal(|ui| {
                            if ui.button(ti("Hepsini kopyala")).clicked() {
                                ui.ctx().copy_text(self.totp.codes.join("\n"));
                            }
                            if ui.button(ti("Kaydettim, gizle")).clicked() {
                                self.totp.codes.clear();
                            }
                        });
                    });
                    ui.add_space(4.0);
                } else {
                    ui.label(RichText::new(format!("{} {}", ti("Kalan kurtarma kodu:"), self.totp.left)).small().color(muted()));
                }
                ui.horizontal(|ui| {
                    if ui.button(ti("Yeni kurtarma kodları")).on_hover_text(ti("Şifrenizi ve güncel kodu yazın; eski kodlar geçersiz olur.")).clicked() {
                        let body = serde_json::json!({"password": self.totp.pw, "code": self.totp.code});
                        self.call(move |api| api.send::<_, serde_json::Value>("POST", "/api/me/totp/recovery", &body), |app, v| {
                            app.totp.codes = v["codes"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                            app.totp.left = app.totp.codes.len();
                            app.totp.pw.clear();
                            app.totp.code.clear();
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.totp.pw).password(true).hint_text(ti("Şifreniz")).desired_width(150.0));
                    ui.add(egui::TextEdit::singleline(&mut self.totp.code).hint_text(ti("Kod")).desired_width(90.0));
                    if ui.button(ti("Kapat")).clicked() {
                        let body = serde_json::json!({"password": self.totp.pw, "code": self.totp.code});
                        self.call(move |api| api.send::<_, serde_json::Value>("POST", "/api/me/totp/disable", &body), |app, _| {
                            app.totp = TotpUi { loaded: true, ..Default::default() };
                            app.toast(ti("İki aşamalı doğrulama kapatıldı."));
                        });
                    }
                });
            } else if self.totp.secret.is_empty() {
                if ui.button(ti("Kurulumu başlat")).clicked() {
                    self.call(|api| api.send::<_, serde_json::Value>("POST", "/api/me/totp/setup", &serde_json::json!({})), |app, v| {
                        app.totp.secret = v["secret"].as_str().unwrap_or("").to_string();
                        app.totp.uri = v["uri"].as_str().unwrap_or("").to_string();
                    });
                }
            } else {
                ui.colored_label(egui::Color32::from_rgb(220, 150, 40), ti("Dikkat: aşağıdaki anahtar giriş kodu DEĞİLDİR; önce telefondaki doğrulama uygulamasına eklenmelidir. Kodu uygulama üretir ve her 30 saniyede değişir."));
                if let Ok(qr) = qrcode::QrCode::new(self.totp.uri.as_bytes()) {
                    let w = qr.width();
                    let cell = 5.0;
                    let q = 3.0;
                    let size = egui::vec2((w as f32 + 2.0 * q) * cell, (w as f32 + 2.0 * q) * cell);
                    let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter().rect_filled(r, 4.0, egui::Color32::WHITE);
                    let cols = qr.to_colors();
                    for y in 0..w {
                        for x in 0..w {
                            if cols[y * w + x] == qrcode::Color::Dark {
                                let p = r.min + egui::vec2((x as f32 + q) * cell, (y as f32 + q) * cell);
                                ui.painter().rect_filled(egui::Rect::from_min_size(p, egui::vec2(cell + 0.3, cell + 0.3)), 0.0, egui::Color32::BLACK);
                            }
                        }
                    }
                    ui.label(RichText::new(ti("Telefondaki uygulamada \"QR kodu tara\" ile okutun.")).small().color(muted()));
                    ui.label(RichText::new(ti("Okutamıyorsanız anahtarı elle girin:")).small().color(muted()));
                }
                ui.label(ti("1) Doğrulama uygulamasında \"Kurulum anahtarı gir\" (hesap: bu kullanıcı, tür: zamana dayalı) seçip şu anahtarı yazın:"));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.totp.secret.as_str()).monospace().strong().size(16.0));
                    if ui.small_button(ti("Kopyala")).clicked() {
                        ui.ctx().copy_text(self.totp.secret.clone());
                    }
                });
                ui.label(ti("2) Uygulamanın gösterdiği 6 haneli kodu girip etkinleştirin:"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.totp.code).hint_text("123456").desired_width(100.0));
                    if ui.button(ti("Etkinleştir")).clicked() {
                        let body = serde_json::json!({"code": self.totp.code});
                        self.call(move |api| api.send::<_, serde_json::Value>("POST", "/api/me/totp/enable", &body), |app, v| {
                            let codes = v["codes"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default();
                            app.totp = TotpUi { loaded: true, enabled: true, left: codes.len(), codes, ..Default::default() };
                            app.toast(ti("İki aşamalı doğrulama açıldı."));
                        });
                    }
                });
            }
        });
    }

    /// Profilim > Açık oturumlar (uzaktan çıkış)
    pub fn sessions_card(&mut self, ui: &mut Ui) {
        if !self.sess_loaded {
            self.sess_loaded = true;
            self.call(|api| api.get::<Vec<serde_json::Value>>("/api/me/sessions"), |app, v| app.sessions = v);
        }
        card_frame(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(ti("Açık oturumlar")).size(17.0).strong());
                if ui.small_button(ti("Yenile")).clicked() {
                    self.sess_loaded = false;
                }
            });
            ui.label(RichText::new(ti("Hesabınızla giriş yapılmış cihazlar. Tanımadığınız bir oturum görürseniz kapatın ve şifrenizi değiştirin.")).small().color(muted()));
            ui.add_space(4.0);
            let list = self.sessions.clone();
            let mut kill: Option<String> = None;
            for s in &list {
                let cur = s["current"].as_bool().unwrap_or(false);
                ui.horizontal(|ui| {
                    let dev = s["device"].as_str().unwrap_or("");
                    let idle = s["idle_secs"].as_u64().unwrap_or(0);
                    let when = if idle < 90 { ti("şimdi etkin").to_string() } else if idle < 5400 { format!("{} {}", idle / 60, ti("dk önce")) } else { format!("{} {}", idle / 3600, ti("sa önce")) };
                    ui.label(RichText::new(format!("{}  •  {}  •  {}  •  {} {}", if dev.is_empty() { ti("Bilinmeyen cihaz") } else { dev }, s["ip"].as_str().unwrap_or("?"), when, ti("giriş:"), s["created"].as_str().unwrap_or("")).as_str()));
                    if cur {
                        ui.colored_label(accent(), ti("(bu oturum)"));
                    } else if ui.small_button(ti("Kapat")).clicked() {
                        kill = Some(s["id"].as_str().unwrap_or("").to_string());
                    }
                });
            }
            if let Some(id) = kill {
                self.call(move |api| api.delete(&format!("/api/me/sessions/{id}")), |app, _| {
                    app.sess_loaded = false;
                    app.toast(ti("Oturum kapatıldı."));
                });
            }
            if list.len() > 1 && ui.button(ti("Diğer tüm oturumları kapat")).clicked() {
                self.call(|api| api.send::<_, serde_json::Value>("POST", "/api/me/sessions/others", &serde_json::json!({})), |app, _| {
                    app.sess_loaded = false;
                    app.toast(ti("Diğer oturumlar kapatıldı."));
                });
            }
        });
    }

    /// Aylık ofis raporu (yönetici / müdür): ay seç, DOCX veya PDF kaydet.
    pub fn report_ui(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label(RichText::new(ti("Aylık ofis raporu")).strong());
        ui.label(RichText::new(ti("Seçilen ayın yeni portföy, kapanan iş, komisyon, randevu ve çalışan özetini; yukarıdaki logo/antet ayarlarına göre belge olarak hazırlar.")).small().color(muted()));
        if self.report_ym.is_empty() {
            let d = chrono::Local::now().date_naive();
            let prev = d.with_day(1).and_then(|x| x.pred_opt()).unwrap_or(d);
            self.report_ym = prev.format("%Y-%m").to_string();
        }
        ui.horizontal(|ui| {
            ui.label(ti("Ay (YYYY-AA):"));
            ui.add(egui::TextEdit::singleline(&mut self.report_ym).desired_width(80.0));
            for (label, ext) in [(ti("DOCX kaydet…"), "docx"), (ti("PDF kaydet…"), "pdf")] {
                if ui.button(label).clicked() {
                    let ym = self.report_ym.trim().to_string();
                    let s = self.set_edit.clone();
                    self.call(
                        move |api| {
                            let v: serde_json::Value = api.get(&format!("/api/reports/monthly?ym={ym}"))?;
                            let logo = if s.show_logo { api.get_bytes("/api/settings/logo").ok() } else { None };
                            let name = format!("Aylik_Rapor_{ym}.{ext}");
                            let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter(ext, &[ext]).save_file() else { return Ok(String::new()) };
                            if ext == "docx" { crate::monthly::write_docx(&v, &s, logo.as_deref(), &path)?; } else { crate::monthly::write_pdf(&v, &s, logo.as_deref(), &path)?; }
                            Ok(path.to_string_lossy().to_string())
                        },
                        |app, p| if !p.is_empty() { app.toast(format!("{} {}", ti("Rapor kaydedildi:"), p)) },
                    );
                }
            }
        });
    }
}
