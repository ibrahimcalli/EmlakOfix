//! Gerçek harita altlığı: OpenStreetMap (sokak) ve Esri World Imagery (uydu) karoları.
//! Karolar arka plan iş parçacıklarında indirilir, diske önbelleğe alınır (çevrimdışı çalışmaya devam eder).
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Source {
    Street,
    Satellite,
}

impl Source {
    pub fn dir(self) -> &'static str {
        match self {
            Source::Street => "sokak",
            Source::Satellite => "uydu",
        }
    }
    pub fn attribution(self) -> &'static str {
        match self {
            Source::Street => "© OpenStreetMap katkıda bulunanlar",
            Source::Satellite => "Görüntü © Esri, Maxar, Earthstar Geographics",
        }
    }
}

/// Web Mercator: (boylam, enlem, yakınlaştırma) → karo koordinatı (kesirli)
pub fn lonlat_to_tile(lon: f64, lat: f64, z: u32) -> (f64, f64) {
    let n = 2f64.powi(z as i32);
    let lat = lat.clamp(-85.0511, 85.0511).to_radians();
    let x = (lon + 180.0) / 360.0 * n;
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// Karo koordinatı (kesirli) → (boylam, enlem)
pub fn tile_to_lonlat(x: f64, y: f64, z: f64) -> (f64, f64) {
    let n = 2f64.powf(z);
    let lon = x / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees();
    (lon, lat)
}

pub fn url(src: Source, z: u32, x: u32, y: u32) -> String {
    match src {
        Source::Street => format!("https://tile.openstreetmap.org/{z}/{x}/{y}.png"),
        Source::Satellite => format!("https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}"),
    }
}

/// Disk önbelleği: <klasör>/<kaynak>/<z>/<x>_<y>.tile
#[derive(Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: PathBuf) -> Cache {
        Cache { root }
    }
    pub fn default_root() -> PathBuf {
        directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis").map(|d| d.cache_dir().join("karolar")).unwrap_or_else(|| std::env::temp_dir().join("emlakofis_karolar"))
    }
    fn path(&self, s: Source, z: u32, x: u32, y: u32) -> PathBuf {
        self.root.join(s.dir()).join(z.to_string()).join(format!("{x}_{y}.tile"))
    }
    pub fn read(&self, s: Source, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        std::fs::read(self.path(s, z, x, y)).ok().filter(|b| !b.is_empty())
    }
    pub fn write(&self, s: Source, z: u32, x: u32, y: u32, data: &[u8]) {
        let p = self.path(s, z, x, y);
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, data);
    }
    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
    pub fn size_bytes(&self) -> u64 {
        fn walk(p: &std::path::Path) -> u64 {
            std::fs::read_dir(p).map(|d| d.flatten().map(|e| match e.metadata() { Ok(m) if m.is_dir() => walk(&e.path()), Ok(m) => m.len(), _ => 0 }).sum()).unwrap_or(0)
        }
        walk(&self.root)
    }
}

type Key = (Source, u32, u32, u32);

pub struct Store {
    cache: Cache,
    tex: HashMap<Key, egui::TextureHandle>,
    pending: HashSet<Key>,
    failed: HashMap<Key, std::time::Instant>,
    tx: Sender<(Key, Option<Vec<u8>>)>,
    rx: Receiver<(Key, Option<Vec<u8>>)>,
    pub net_errors: u32,
}

const MAX_INFLIGHT: usize = 8;

impl Default for Store {
    fn default() -> Self {
        let (tx, rx) = channel();
        Store { cache: Cache::new(Cache::default_root()), tex: HashMap::new(), pending: HashSet::new(), failed: HashMap::new(), tx, rx, net_errors: 0 }
    }
}

impl Store {
    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    /// Gelen karoları dokuya çevirir. Yeni karo geldiyse true.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut any = false;
        while let Ok((key, data)) = self.rx.try_recv() {
            self.pending.remove(&key);
            match data.and_then(|b| image::load_from_memory(&b).ok()) {
                Some(img) => {
                    let rgba = img.to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                    let name = format!("tile{:?}{}/{}/{}", key.0, key.1, key.2, key.3);
                    self.tex.insert(key, ctx.load_texture(name, ci, egui::TextureOptions::LINEAR));
                    any = true;
                }
                None => {
                    self.net_errors += 1;
                    self.failed.insert(key, std::time::Instant::now());
                }
            }
        }
        if self.tex.len() > 600 {
            self.tex.clear(); // bellek sınırı; diskten yeniden okunur
        }
        any
    }

    pub fn get(&mut self, ctx: &egui::Context, s: Source, z: u32, x: u32, y: u32) -> Option<egui::TextureId> {
        let key = (s, z, x, y);
        if let Some(t) = self.tex.get(&key) {
            return Some(t.id());
        }
        if self.pending.contains(&key) || self.pending.len() >= MAX_INFLIGHT {
            return None;
        }
        if let Some(t) = self.failed.get(&key) {
            if t.elapsed().as_secs() < 30 {
                return None;
            }
            self.failed.remove(&key);
        }
        self.pending.insert(key);
        let (tx, cache, ctx2) = (self.tx.clone(), self.cache.clone(), ctx.clone());
        std::thread::spawn(move || {
            let data = cache.read(s, z, x, y).or_else(|| {
                let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(5)).timeout(std::time::Duration::from_secs(10)).user_agent("EmlakOfis/1.2 (ofis ici masaustu program)").build();
                let r = agent.get(&url(s, z, x, y)).call().ok()?;
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut std::io::Read::take(r.into_reader(), 2 * 1024 * 1024), &mut buf).ok()?;
                if buf.len() < 100 {
                    return None;
                }
                cache.write(s, z, x, y, &buf);
                Some(buf)
            });
            let _ = tx.send(((s, z, x, y), data));
            ctx2.request_repaint();
        });
        None
    }
}
