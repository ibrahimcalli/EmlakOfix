//! Ayarlar sayfası: Görünüm, Bağlantı, Hakkında/Güncelleme (herkes) + Çıktı, Yedekleme (yönetici).
use crate::i18n_ui::ti;
use crate::api::{self, Api};
use crate::app::*;
use crate::theme::{self, *};
use eframe::egui::{self, vec2, Color32, RichText, Sense, Stroke, Ui};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn swatch(ui: &mut Ui, color: Color32, selected: bool, tip: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    ui.painter().circle_filled(r.center(), 11.5, color);
    if selected {
        ui.painter().circle_stroke(r.center(), 13.5, Stroke::new(2.0_f32, t_strong()));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked()
}

pub fn section(ui: &mut Ui, title: &str, hint: &str, add: impl FnOnce(&mut Ui)) {
    card_frame(ui.style()).inner_margin(16.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).size(17.0).strong().color(t_strong()));
        if !hint.is_empty() {
            ui.label(RichText::new(hint).small().color(muted()));
        }
        ui.add_space(6.0);
        add(ui);
    });
    ui.add_space(10.0);
}

fn same_minor(a: &str, b: &str) -> bool {
    let k = |s: &str| s.split('.').take(2).collect::<Vec<_>>().join(".");
    k(a) == k(b)
}

impl App {
    pub fn ui_settings(&mut self, ui: &mut Ui) {
        page_title(ui, "Ayarlar");
        let admin = self.me.is_admin();
        if !admin && self.set_tab > 2 {
            self.set_tab = 0;
        }
        ui.horizontal_wrapped(|ui| {
            let mut tabs = vec![(0u8, "Görünüm"), (1, "Bağlantı"), (2, "Hakkında ve güncelleme")];
            if admin {
                tabs.push((3, "Çıktı ayarları"));
                tabs.push((4, "Yedekleme ve sunucu"));
                tabs.push((5, "Testler"));
            }
            for (i, n) in tabs {
                if ui.selectable_label(self.set_tab == i, RichText::new(format!("  {}  ", ti(n))).size(15.5)).clicked() {
                    self.set_tab = i;
                    if i == 4 {
                        self.load_server_info();
                    }
                }
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(900.0);
            match self.set_tab {
                0 => self.tab_look(ui),
                1 => self.tab_conn(ui),
                2 => {
                    self.tab_about(ui);
                    self.ext_update_section(ui);
                }
                3 => self.settings_output(ui),
                5 => self.tab_tests(ui),
                _ => {
                    self.server_panel(ui);
                    self.ext_backup_section(ui);
                }
            }
        });
    }

    fn commit_look(&mut self, look: theme::Look) {
        if look == self.cfg.look {
            return;
        }
        let font_changed = look.font != self.cfg.look.font;
        self.cfg.look = look;
        crate::i18n_ui::set_lang(crate::i18n_ui::UiLang::from_code(&self.cfg.look.lang));
        if font_changed {
            theme::apply(&self.ctx, &self.cfg.look);
        } else {
            theme::apply_fast(&self.ctx, &self.cfg.look);
        }
        self.cfg.save();
    }

    // ------------------------------------------------------------ Görünüm
    fn tab_look(&mut self, ui: &mut Ui) {
        let mut look = self.cfg.look.clone();

        section(ui, ti("Tema"), ti("Açık tema gündüz, koyu tema loş ortamlar ve göz yorgunluğu için uygundur."), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut look.dark, false, RichText::new(ti("   ☀  Açık tema   ")).size(15.5));
                ui.selectable_value(&mut look.dark, true, RichText::new(ti("   🌙  Koyu tema   ")).size(15.5));
            });
            ui.add_space(4.0);
            ui.label(RichText::new(ti("Görünüm paketi")).strong());
            ui.horizontal_wrapped(|ui| {
                for (k, name) in theme::SKINS {
                    if ui.selectable_label(look.skin == k, RichText::new(ti(name)).size(15.0)).clicked() {
                        look.skin = k.to_string();
                    }
                }
            });
            ui.label(RichText::new(ti("Altın & Lacivert ve Gece paketleri koyu çalışır; menü ve ortadaki alanlar daha karanlık, grafikler daha okunaklıdır.")).small().color(muted()));
        });

        self.bg_section(ui, &mut look);

        section(ui, ti("Renkler"), ti("Ana renk başlıkları, seçili menüyü ve bağlantıları; buton rengi ana düğmeleri (Kaydet, Ekle…) boyar."), |ui| {
            ui.label(RichText::new(ti("Ana renk")).strong());
            ui.horizontal_wrapped(|ui| {
                for (name, hex) in theme::PRESETS {
                    if swatch(ui, theme::parse_hex(hex).unwrap(), look.accent.eq_ignore_ascii_case(hex), name) {
                        look.accent = hex.to_string();
                    }
                }
                let mut rgb = theme::parse_hex(&look.accent).map(|c| [c.r(), c.g(), c.b()]).unwrap_or([31, 78, 121]);
                if ui.color_edit_button_srgb(&mut rgb).on_hover_text(ti("Özel renk seç")).changed() {
                    look.accent = theme::to_hex(Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                }
                ui.label(RichText::new(look.accent.clone()).small().color(muted()));
            });
            ui.add_space(8.0);
            ui.label(RichText::new(ti("Buton rengi")).strong());
            ui.horizontal_wrapped(|ui| {
                let same = look.button.is_empty();
                if ui.selectable_label(same, ti("Ana renkle aynı")).clicked() {
                    look.button.clear();
                }
                for (name, hex) in theme::PRESETS {
                    if swatch(ui, theme::parse_hex(hex).unwrap(), !same && look.button.eq_ignore_ascii_case(hex), name) {
                        look.button = hex.to_string();
                    }
                }
                let cur = theme::parse_hex(if same { &look.accent } else { &look.button }).map(|c| [c.r(), c.g(), c.b()]).unwrap_or([31, 78, 121]);
                let mut rgb = cur;
                if ui.color_edit_button_srgb(&mut rgb).on_hover_text(ti("Özel renk seç")).changed() {
                    look.button = theme::to_hex(Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                }
            });
        });

        section(ui, ti("Yazı tipi ve boyutu"), ti("Yazılar okunaksızsa boyutu büyütün. Yazı tipi yalnızca bu bilgisayarı etkiler."), |ui| {
            ui.horizontal(|ui| {
                ui.label(ti("Yazı tipi:"));
                let cur_name = self.fonts.iter().find(|(_, k)| *k == look.font).map(|(n, _)| n.clone()).unwrap_or_else(|| "Varsayılan".into());
                egui::ComboBox::from_id_salt("font_pick").width(280.0).selected_text(cur_name).show_ui(ui, |ui| {
                    for (name, key) in self.fonts.clone() {
                        ui.selectable_value(&mut look.font, key, name);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(ti("Yazı boyutu:"));
                for (name, v) in theme::SCALES {
                    if ui.selectable_label(look.scale == v, name).clicked() {
                        look.scale = v;
                    }
                }
                let mut sc = look.scale as f32;
                if ui.add(egui::Slider::new(&mut sc, 85.0..=150.0).suffix(" %").integer()).changed() {
                    look.scale = sc as u32;
                }
            });
            ui.label(RichText::new(ti("Örnek: Çalışkan öğrenci şimdi yağmurda üşüdü — ĞÜŞİÖÇ ığüşöç 0123456789")).color(t_mid()));
        });

        section(ui, ti("Dil"), "", |ui| {
            ui.horizontal(|ui| {
                ui.label(ti("Program dili:"));
                let cur = crate::i18n_ui::UiLang::from_code(&look.lang);
                egui::ComboBox::from_id_salt("lang_pick").width(200.0).selected_text(cur.label()).show_ui(ui, |ui| {
                    for l in crate::i18n_ui::UiLang::ALL {
                        ui.selectable_value(&mut look.lang, l.code().to_string(), l.label());
                    }
                });
            });
            ui.label(RichText::new(ti("Program dili seçilir. Tarih, para ve belge çıktısı dili ayrıca belirlenir.")).small().color(muted()));
        });

        section(ui, ti("Önizleme"), "", |ui| {
            ui.horizontal_wrapped(|ui| {
                let _ = ui.add(egui::Button::new(RichText::new(ti("Kaydet")).strong().color(on_btn())).fill(btn()));
                let _ = ui.button(ti("Vazgeç"));
                badge(ui, ti("Yayında"), Color32::from_rgb(34, 139, 70));
                badge(ui, ti("Satılık"), accent());
                ui.label(RichText::new(ti("Ana renkli başlık")).size(17.0).color(accent()));
                let _ = ui.link(ti("Bağlantı örneği"));
            });
        });

        if ui.button(ti("Varsayılan görünüme dön")).clicked() {
            look = theme::Look::default();
        }
        self.commit_look(look);
    }

    // ------------------------------------------------------------ Bağlantı
    fn run_test(&mut self, addr: String) {
        if self.conn_busy {
            return;
        }
        self.conn_busy = true;
        self.conn_msg = "Test ediliyor…".into();
        self.latency = None;
        self.spawn(
            move || {
                let t = std::time::Instant::now();
                let r = Api::new(&addr).ping();
                (r, t.elapsed().as_millis())
            },
            |app, (r, ms)| {
                app.conn_busy = false;
                match r {
                    Ok(p) if p.app == "emlak-ofis" => {
                        app.latency = Some(ms);
                        app.server_ver = p.version.clone();
                        app.conn_msg = if same_minor(&p.version, VERSION) {
                            "Bağlantı başarılı.".into()
                        } else {
                            format!("Bağlantı var ama sürümler farklı (bu bilgisayar {VERSION}, ana bilgisayar {}). Tüm bilgisayarları aynı sürüme güncelleyin.", p.version)
                        };
                    }
                    Ok(_) => app.conn_msg = "Bu adreste Emlak Ofis sunucusu yok.".into(),
                    Err(e) => app.conn_msg = e,
                }
            },
        );
    }

    fn tab_conn(&mut self, ui: &mut Ui) {
        let host = self.server_state.is_some();
        let port = self.cfg.port_or_default();

        section(ui, ti("Bu bilgisayarın rolü"), "", |ui| {
            if host {
                ui.label(RichText::new(ti("ANA BİLGİSAYAR (sunucu)")).strong().color(accent()));
                ui.label(ti("Veriler bu bilgisayarda tutulur; diğer bilgisayarlar buraya bağlanır. Ofis saatlerinde açık kalmalıdır."));
            } else {
                ui.label(RichText::new(ti("İSTEMCİ")).strong().color(accent()));
                ui.label(format!("Ana bilgisayara bağlı: {}", self.cfg.server_addr));
            }
            ui.horizontal(|ui| {
                let (c, t) = if self.offline { (Color32::from_rgb(200, 60, 60), "Şu an bağlantı yok") } else { (Color32::from_rgb(40, 160, 85), "Şu an bağlı") };
                ui.label(RichText::new(ti("●")).color(c));
                ui.label(RichText::new(t).color(c).strong());
                if let Some(ms) = self.latency {
                    ui.label(RichText::new(format!("(yanıt süresi {ms} ms)")).small().color(muted()));
                }
            });
        });

        section(ui, ti("Şifreli bağlantı (HTTPS)"), ti("Ofis ağındaki trafiği (şifreler, müşteri bilgileri) dinlemeye karşı şifreler."), |ui| {
            if host {
                let mut on = self.cfg.https;
                if ui.checkbox(&mut on, ti("Bağlantıyı şifrele (HTTPS) — programı yeniden başlatınca etkinleşir")).changed() {
                    self.cfg.https = on;
                    self.cfg.save();
                }
                match self.server_state.as_ref().and_then(|s| s.tls_fp.clone()) {
                    Some(fp) => {
                        ui.label(RichText::new(ti("Şu an: ŞİFRELİ")).strong().color(Color32::from_rgb(40, 160, 85)));
                        ui.label(RichText::new(format!("Sertifika parmak izi: {}", crate::tls::fp_pretty(&fp))).small().color(muted()));
                        ui.label(RichText::new(ti("Diğer bilgisayarlar ilk bağlanışta bu parmak izini kaydeder; sonradan değişirse bağlantı reddedilir.")).small().color(muted()));
                    }
                    None => {
                        ui.label(RichText::new(ti("Şu an: şifresiz (HTTP). Yalnızca güvendiğiniz ofis ağında kullanın.")).color(Color32::from_rgb(200, 130, 20)));
                    }
                }
            } else {
                let h = Api::new(&self.cfg.server_addr).host();
                match crate::api::get_pins().get(&h).cloned() {
                    Some(fp) => {
                        ui.label(RichText::new(ti("Bu ana bilgisayarla bağlantı ŞİFRELİ; sertifika kayıtlı.")).strong().color(Color32::from_rgb(40, 160, 85)));
                        ui.label(RichText::new(format!("Kayıtlı parmak izi: {}", crate::tls::fp_pretty(&fp))).small().color(muted()));
                        if ui.button(ti("Sertifika güvenini sıfırla")).on_hover_text(ti("Ana bilgisayar yeniden kurulduysa veya sertifikası yenilendiyse kullanın.")).clicked() {
                            crate::api::forget_pin(&h);
                            self.cfg.pins = crate::api::get_pins();
                            self.cfg.save();
                            self.toast(ti("Sertifika güveni sıfırlandı; bir sonraki bağlantıda yeniden kaydedilir."));
                        }
                    }
                    None => {
                        ui.label(RichText::new(ti("Bu ana bilgisayarla bağlantı şifresiz (HTTP) veya henüz kurulmadı. Ana bilgisayarda HTTPS açılırsa otomatik şifreli bağlanılır.")).color(muted()));
                    }
                }
            }
        });

        if host {
            let ips = crate::server::local_ips();
            section(ui, ti("Diğer bilgisayarların bağlanacağı adres"), ti("Kablolu ve kablosuz (Wi-Fi) bilgisayarlar aynı ağdaysa aynı adresi kullanır."), |ui| {
                if ips.is_empty() {
                    ui.label(RichText::new(ti("Ağ adresi bulunamadı — bu bilgisayarın ağa bağlı olduğundan emin olun.")).color(muted()));
                }
                for ip in &ips {
                    let a = format!("{ip}:{port}");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&a).size(18.0).strong().color(t_strong()));
                        if ui.small_button(ti("Kopyala")).clicked() {
                            ui.ctx().copy_text(a.clone());
                            self.toast(ti("Adres kopyalandı."));
                        }
                    });
                }
                ui.add_space(6.0);
                ui.label(RichText::new(ti("Güvenlik duvarı izni (bir kez yapılır)")).strong());
                let cmds = [
                    ("Linux", format!("sudo ufw allow {port}/tcp && sudo ufw allow 47862/udp")),
                    ("Windows (Yönetici komut satırı)", format!("netsh advfirewall firewall add rule name=\"Emlak Ofis\" dir=in action=allow protocol=TCP localport={port}")),
                ];
                for (os, c) in cmds {
                    ui.label(RichText::new(os).small().color(muted()));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&c).monospace().size(12.5));
                        if ui.small_button(ti("Kopyala")).clicked() {
                            ui.ctx().copy_text(c.clone());
                            self.toast(ti("Komut kopyalandı."));
                        }
                    });
                }
            });
        } else {
            section(ui, ti("Ana bilgisayara bağlantı"), ti("Adres değiştirirseniz yeniden giriş yapmanız gerekir."), |ui| {
                ui.horizontal(|ui| {
                    ui.label(ti("Adres:"));
                    ui.add(egui::TextEdit::singleline(&mut self.addr_edit).hint_text(ti("192.168.1.20:7878")).desired_width(220.0));
                    if ui.add_enabled(!self.conn_busy, egui::Button::new("Bağlantıyı test et")).clicked() {
                        let a = self.addr_edit.trim().to_string();
                        self.run_test(a);
                    }
                });
                ui.horizontal(|ui| {
                    let label = if self.searching { "Ağ taranıyor… (en çok ~15 sn)" } else { "Ağda ana bilgisayarı ara (kablolu + Wi-Fi)" };
                    if ui.add_enabled(!self.searching, egui::Button::new(label)).clicked() {
                        self.searching = true;
                        self.found.clear();
                        self.conn_msg = String::new();
                        self.spawn(move || api::discover_all(port), |app, v| {
                            app.searching = false;
                            app.conn_msg = if v.is_empty() {
                                "Ana bilgisayar bulunamadı. Aynı ağda mısınız? Güvenlik duvarı iznini ve adresi kontrol edin.".into()
                            } else {
                                "Ana bilgisayar bulundu; aşağıdan seçin.".into()
                            };
                            app.found = v;
                        });
                    }
                });
                for f in self.found.clone() {
                    if ui.link(format!("Bulundu: {f}  (bunu kullan)")).clicked() {
                        self.addr_edit = f;
                    }
                }
                if !self.conn_msg.is_empty() {
                    let ok = self.conn_msg.starts_with("Bağlantı başarılı") || self.conn_msg.contains("bulundu;");
                    ui.label(RichText::new(self.conn_msg.clone()).color(if ok { Color32::from_rgb(40, 150, 80) } else { Color32::from_rgb(200, 90, 60) }));
                }
                ui.add_space(6.0);
                let changed = self.addr_edit.trim() != self.cfg.server_addr && !self.addr_edit.trim().is_empty();
                if ui.add_enabled(changed, egui::Button::new(RichText::new(ti("Kaydet ve yeniden bağlan")).strong().color(on_btn())).fill(btn())).clicked() {
                    self.cfg.server_addr = self.addr_edit.trim().to_string();
                    self.cfg.save();
                    self.tex.clear();
                    self.connect();
                }
            });
        }

        section(ui, ti("Kablosuz (Wi-Fi) ve karışık ağ kullanımı"), "", |ui| {
            for t in [
                "Kablolu ve Wi-Fi bilgisayarlar aynı modem/ağa bağlıysa birlikte çalışır; Windows ve Linux bilgisayarlar karışık kullanılabilir.",
                "Windows bilgisayar, Linux ana bilgisayara aynı şekilde bağlanır; ek program gerekmez. Yalnızca ana bilgisayarda güvenlik duvarı izni gerekir.",
                "Wi-Fi'de \"misafir ağı\" veya \"istemci yalıtımı (AP isolation)\" açıksa cihazlar birbirini göremez; ofis ağına bağlanın ya da yalıtımı kapatın.",
                "Ana bilgisayarı mümkünse kabloyla bağlayın ve modemde ona sabit IP (DHCP rezervasyonu) verin; böylece adres değişmez.",
                "Wi-Fi anlık kopsa bile program otomatik yeniden dener; kaydedilmiş veriler kaybolmaz.",
                "Programı internete açmayın (modemde port yönlendirme yapmayın). Ofis dışından erişim için VPN kullanın.",
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.label(ti("•"));
                    ui.label(t);
                });
            }
        });
    }

    // ------------------------------------------------------------ Hakkında / güncelleme
    fn tab_about(&mut self, ui: &mut Ui) {
        section(ui, ti("Sürüm"), "", |ui| {
            egui::Grid::new("about").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
                ui.label(ti("Bu bilgisayardaki program"));
                ui.label(RichText::new(format!("{VERSION}")).strong());
                ui.end_row();
                ui.label(ti("Ana bilgisayardaki program"));
                let sv = if self.server_ver.is_empty() { "—".to_string() } else { self.server_ver.clone() };
                ui.label(RichText::new(sv).strong());
                ui.end_row();
                ui.label(ti("Ayar dosyası"));
                ui.label(config_path().map(|p| p.display().to_string()).unwrap_or_default());
                ui.end_row();
                if self.server_state.is_some() {
                    ui.label(ti("Veri klasörü"));
                    let dir = if self.cfg.data_dir.is_empty() { default_data_dir() } else { std::path::PathBuf::from(&self.cfg.data_dir) };
                    ui.label(dir.display().to_string());
                    ui.end_row();
                }
            });
            if !self.server_ver.is_empty() && !same_minor(&self.server_ver, VERSION) {
                ui.add_space(6.0);
                ui.label(RichText::new(ti("Dikkat: Bu bilgisayarla ana bilgisayarın program sürümleri farklı. Tüm bilgisayarları aynı sürüme güncelleyin.")).color(Color32::from_rgb(200, 100, 40)).strong());
            }
        });

        section(ui, ti("Programı güncelleme (özet)"), ti("Ayrıntılı anlatım KURULUM.docx dosyasındaki \"Güncelleme\" bölümündedir. Güncelleme kutusu bu sayfanın altındadır."), |ui| {
            for (i, t) in [
                "Yönetici ana bilgisayarda: aşağıdaki \"Güncelleme paketi seç (.zip)\" ile paketi seçip \"Kur ve yeniden başlat\" der. Program kapanır, kurar ve kendiliğinden açılır.",
                "Yeni sürüm açılmazsa ya da hata verirse program eski sürüme kendiliğinden döner.",
                "Yönetici kurunca paket ana bilgisayara da yüklenir; diğer bilgisayarlar açılışta \"Yeni sürüm hazır\" bildirimi görür ve eski sürümle çalışmaya devam edebilir.",
                "Diğer bilgisayarlarda aynı zip seçilir ya da \"Ana bilgisayardan indir ve kur\" düğmesi kullanılır.",
                "Veriler ayrı klasörde durduğu için güncellemede korunur.",
            ]
            .iter()
            .enumerate()
            {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{}.", i + 1)).strong());
                    ui.label(*t);
                });
            }
        });

        section(ui, ti("Hakkında"), "", |ui| {
            ui.label(ti("Emlak Ofis — portföy, CRM, mesai, görev ve hedef yönetimi. Yalnızca ofis içi kullanım içindir."));
            ui.label(RichText::new(ti("Rust • egui • SQLite. İlçe/mahalle verisi: turkey-neighbourhoods (MIT lisansı).")).small().color(muted()));
        });
    }
}

impl App {
    /// Ayarlar > Testler: tüm otomatik testleri çalıştırır, sonuçları listeler; kopyalama / DOCX kaydı.
    pub fn tab_tests(&mut self, ui: &mut Ui) {
        use crate::selftest;
        section(ui, ti("Testler"), "Programın kendi kendine testleri: geçici bir sunucu başlatır, yetkileri, akışları, HTTPS'i, imzayı, yedeği, belge çıktısını ve haritayı dener. Gerçek verilerinize dokunmaz.", |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.test_busy, |ui| {
                    if ui.add(egui::Button::new(RichText::new("▶ Tüm testleri çalıştır").strong().color(on_btn())).fill(btn())).clicked() {
                        self.test_busy = true;
                        self.test_res.clear();
                        self.spawn(selftest::run_all, |app, r| {
                            app.test_busy = false;
                            app.test_done = true;
                            app.test_res = r;
                        });
                    }
                });
                if self.test_busy {
                    ui.spinner();
                    ui.label("Çalışıyor… (yaklaşık 10–20 sn)");
                }
            });
        });
        if !self.test_done {
            return;
        }
        let (ok, bad) = selftest::summary(&self.test_res);
        let total = self.test_res.len();
        let col = if bad == 0 { Color32::from_rgb(34, 139, 70) } else { Color32::from_rgb(190, 55, 55) };
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(if bad == 0 { format!("✔ Tüm testler geçti ({ok}/{total})") } else { format!("✘ {bad} test başarısız — {ok}/{total} geçti") }).size(17.0).strong().color(col));
            if ui.button("Sonuçları kopyala").on_hover_text("Tüm sonuçları metin olarak panoya kopyalar").clicked() {
                ui.ctx().copy_text(selftest::to_text(&self.test_res));
                self.toast("Test sonuçları panoya kopyalandı.");
            }
            if bad > 0 && ui.button("Yalnızca hataları kopyala").clicked() {
                let only: Vec<_> = self.test_res.iter().filter(|r| !r.ok).cloned().collect();
                ui.ctx().copy_text(selftest::to_text(&only));
                self.toast("Hatalı testler panoya kopyalandı.");
            }
            if ui.button("DOCX olarak kaydet…").clicked() {
                let res = self.test_res.clone();
                self.spawn(
                    move || -> Result<Option<String>, String> {
                        let name = format!("Emlak_Ofis_Test_Raporu_{}.docx", chrono::Local::now().format("%Y%m%d_%H%M"));
                        let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter("Word", &["docx"]).save_file() else { return Ok(None) };
                        selftest::save_docx(&res, &path)?;
                        Ok(Some(path.to_string_lossy().to_string()))
                    },
                    |app, r| match r {
                        Ok(Some(p)) => app.toast(format!("Rapor kaydedildi: {p}")),
                        Ok(None) => {}
                        Err(e) => app.err(e),
                    },
                );
            }
        });
        ui.add_space(4.0);
        let mut groups: Vec<String> = Vec::new();
        for r in &self.test_res {
            if !groups.contains(&r.group) {
                groups.push(r.group.clone());
            }
        }
        for g in groups {
            let items: Vec<&selftest::TestResult> = self.test_res.iter().filter(|r| r.group == g).collect();
            let gbad = items.iter().filter(|r| !r.ok).count();
            let head = format!("{} {}  ({}/{})", if gbad == 0 { "✔" } else { "✘" }, g, items.len() - gbad, items.len());
            egui::CollapsingHeader::new(RichText::new(head).strong().color(if gbad == 0 { Color32::from_rgb(34, 139, 70) } else { Color32::from_rgb(190, 55, 55) }))
                .id_salt(format!("tg_{g}"))
                .default_open(gbad > 0)
                .show(ui, |ui| {
                    for r in items {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(if r.ok { "✔" } else { "✘" }).color(if r.ok { Color32::from_rgb(34, 139, 70) } else { Color32::from_rgb(190, 55, 55) }));
                            ui.label(&r.name);
                            ui.label(RichText::new(format!("{} ms", r.ms)).small().color(muted()));
                        });
                        if !r.ok {
                            ui.label(RichText::new(format!("    → {}", r.detail)).color(Color32::from_rgb(190, 55, 55)).small());
                        }
                    }
                });
        }
    }
}
