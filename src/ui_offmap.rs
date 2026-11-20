//! Çevrimdışı harita indirme penceresi ve ilk kurulumda Fethiye paketi.
use crate::app::*;
use crate::i18n_ui::ti;
use crate::map_tiles as tiles;
use crate::theme::*;
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

const MAX_TILES: usize = 20000;

fn est_mb(n: usize, sat: bool, street: bool) -> f64 {
    n as f64 * (if sat { 26.0 } else { 0.0 } + if street { 14.0 } else { 0.0 }) / 1024.0
}

impl App {
    /// Giriş sonrası bir kez: Fethiye ve çevresi uydu paketini (genel + ilçe merkezleri) arka planda indirir.
    pub fn seed_offline_maps(&mut self, ctx: &egui::Context) {
        if self.cfg.offline_seeded {
            return;
        }
        if let Some(j) = self.ext.map.job.clone() {
            // başarıyla (çoğu karo inmiş) bittiyse bir daha denenmez; internet yoksa sonraki açılışta yeniden denenir
            if self.ext.map.seeding && j.finished.load(Ordering::Relaxed) {
                self.ext.map.seeding = false;
                if j.failed.load(Ordering::Relaxed) * 2 < j.total {
                    self.cfg.offline_seeded = true;
                    self.cfg.save();
                }
            }
            return;
        }
        if self.ext.map.seed_tried {
            return;
        }
        self.ext.map.seed_tried = true;
        self.ext.map.seeding = true;
        let mut all = Vec::new();
        for r in tiles::REGIONS {
            all.extend(tiles::tiles_for(r.1, r.2, r.3));
        }
        self.ext.map.job = Some(tiles::start_download(tiles::offline_root(), vec![tiles::Source::Satellite], all, ctx.clone()));
    }

    pub fn offline_window(&mut self, ctx: &egui::Context) {
        let mut open = self.ext.map.off_open;
        if !open {
            return;
        }
        egui::Window::new(ti("Çevrimdışı harita indir")).open(&mut open).collapsible(false).resizable(false).default_width(470.0).show(ctx, |ui| {
            let m = &mut self.ext.map;
            let dir = tiles::offline_root();
            ui.label(RichText::new(ti("İnternet olmadığında da haritayı açabilmek için seçtiğiniz bölgeyi bu bilgisayara indirin.")).small().color(muted()));
            ui.label(RichText::new(format!("{} {}", ti("Klasör:"), dir.display())).small().color(muted()));
            ui.add_space(6.0);
            ui.label(RichText::new(ti("Bölge")).strong());
            egui::ComboBox::from_id_salt("off_region").width(420.0).selected_text(if m.off_view { ti("Şu an ekranda görünen alan").to_string() } else { tiles::REGIONS[m.off_region.min(tiles::REGIONS.len() - 1)].0.to_string() }).show_ui(ui, |ui| {
                for (i, r) in tiles::REGIONS.iter().enumerate() {
                    if ui.selectable_label(!m.off_view && m.off_region == i, r.0).clicked() {
                        m.off_region = i;
                        m.off_view = false;
                        m.off_zmax = 0;
                    }
                }
                if ui.selectable_label(m.off_view, ti("Şu an ekranda görünen alan")).clicked() {
                    m.off_view = true;
                    m.off_zmax = (m.view_z + 3).min(17);
                }
            });
            let (bbox, zmin, rmax) = if m.off_view { (m.view_bbox, m.view_z.max(3), (m.view_z + 3).min(17)) } else { let r = &tiles::REGIONS[m.off_region.min(tiles::REGIONS.len() - 1)]; (r.1, r.2, r.3) };
            let mut zmax = if m.off_zmax == 0 { rmax } else { m.off_zmax };
            ui.horizontal(|ui| {
                ui.label(ti("En yüksek yakınlaştırma:"));
                ui.add(egui::Slider::new(&mut zmax, zmin..=17));
            });
            m.off_zmax = zmax;
            ui.horizontal(|ui| {
                ui.label(ti("Altlık:"));
                if ui.selectable_label(!m.off_street, ti("Uydu")).clicked() {
                    m.off_street = false;
                }
                if ui.selectable_label(m.off_street, ti("Sokak")).clicked() {
                    m.off_street = true;
                }
            });
            let list = tiles::tiles_for(bbox, zmin, zmax);
            let mb = est_mb(list.len(), !m.off_street, m.off_street);
            ui.label(format!("{} {} • ~{:.0} MB", list.len(), ti("karo"), mb));
            if list.len() > 6000 {
                ui.colored_label(egui::Color32::from_rgb(220, 150, 40), ti("Çok büyük bir alan. Harita sağlayıcılarına yük bindirmemek için daha küçük bölgeler ve daha düşük yakınlaştırma seçin."));
            }
            let busy = m.job.as_ref().map(|j| !j.finished.load(Ordering::Relaxed)).unwrap_or(false);
            ui.add_space(6.0);
            if !busy {
                let too_big = list.len() > MAX_TILES;
                if ui.add_enabled(!too_big && !list.is_empty(), egui::Button::new(RichText::new(ti("İndirmeyi başlat")).strong().color(on_btn())).fill(btn())).clicked() {
                    let src = if m.off_street { tiles::Source::Street } else { tiles::Source::Satellite };
                    m.job = Some(tiles::start_download(dir.clone(), vec![src], list, ctx.clone()));
                }
                if too_big {
                    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), format!("{} {MAX_TILES}", ti("Tek seferde en çok şu kadar karo indirilebilir:")));
                }
            }
            if let Some(j) = &m.job {
                let done = j.done.load(Ordering::Relaxed);
                let total = j.total.max(1);
                ui.add(egui::ProgressBar::new(done as f32 / total as f32).text(format!("{done} / {total}  •  {:.1} MB", j.bytes.load(Ordering::Relaxed) as f64 / 1_048_576.0)));
                let failed = j.failed.load(Ordering::Relaxed);
                if failed > 0 {
                    ui.label(RichText::new(format!("{} {failed}", ti("İndirilemeyen karo:"))).small().color(egui::Color32::from_rgb(200, 90, 60)));
                }
                if busy {
                    if ui.button(ti("İptal")).clicked() {
                        j.cancel.store(true, Ordering::Relaxed);
                    }
                } else {
                    ui.colored_label(egui::Color32::from_rgb(34, 139, 70), ti("Tamamlandı. \"Çevrimdışı\" modunda bu alan internetsiz açılır."));
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new(format!("{}: {:.0} MB", ti("Çevrimdışı klasör boyutu"), tiles::Cache::new(dir).size_bytes() as f64 / 1_048_576.0)).small().color(muted()));
            ui.label(RichText::new(ti("Haritalar OpenStreetMap ve Esri hizmetlerinden indirilir; yalnızca ofis içi kullanım için, makul miktarda indirin.")).small().color(muted()));
        });
        self.ext.map.off_open = open;
    }
}
