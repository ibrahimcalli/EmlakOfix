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
    /// Çevrimdışı harita klasörü (kurulum dizini/cevrimdisi_haritalar); önce buraya bakılır
    offline: Option<PathBuf>,
}

/// Çevrimdışı harita klasörü: program klasörü yazılabilirse onun yanında, değilse kullanıcı veri klasöründe.
pub fn offline_root() -> PathBuf {
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("cevrimdisi_haritalar"))) {
        if std::fs::create_dir_all(&d).is_ok() {
            let t = d.join(".yazma_denemesi");
            if std::fs::write(&t, b"1").is_ok() {
                let _ = std::fs::remove_file(&t);
                return d;
            }
        }
    }
    directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis").map(|d| d.data_dir().join("cevrimdisi_haritalar")).unwrap_or_else(|| std::env::temp_dir().join("emlakofis_cevrimdisi"))
}

impl Cache {
    pub fn new(root: PathBuf) -> Cache {
        Cache { root, offline: None }
    }
    pub fn with_offline(root: PathBuf, offline: PathBuf) -> Cache {
        Cache { root, offline: Some(offline) }
    }
    pub fn offline_dir(&self) -> Option<&PathBuf> {
        self.offline.as_ref()
    }
    pub fn default_root() -> PathBuf {
        directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis").map(|d| d.cache_dir().join("karolar")).unwrap_or_else(|| std::env::temp_dir().join("emlakofis_karolar"))
    }
    fn path(&self, s: Source, z: u32, x: u32, y: u32) -> PathBuf {
        self.root.join(s.dir()).join(z.to_string()).join(format!("{x}_{y}.tile"))
    }
    pub fn read(&self, s: Source, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        if let Some(o) = &self.offline {
            if let Some(b) = std::fs::read(Cache::new(o.clone()).path(s, z, x, y)).ok().filter(|b| !b.is_empty()) {
                return Some(b);
            }
        }
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
    /// true: ağa hiç çıkmadan yalnızca diskteki (çevrimdışı) karoları göster
    pub offline_only: bool,
    cache: Cache,
    tex: HashMap<Key, egui::TextureHandle>,
    pending: HashSet<Key>,
    failed: HashMap<Key, std::time::Instant>,
    tx: Sender<(Key, Result<Vec<u8>, String>)>,
    rx: Receiver<(Key, Result<Vec<u8>, String>)>,
    /// Üst üste başarısız indirme sayısı (başarılı karo gelince sıfırlanır)
    pub net_errors: u32,
    pub last_err: String,
}

const MAX_INFLIGHT: usize = 4;

fn agent() -> &'static ureq::Agent {
    static A: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(20))
            .user_agent(concat!("EmlakOfis/", env!("CARGO_PKG_VERSION"), " (+https://emlakfethiye.com.tr; ibrahimcallifethiye@gmail.com)"))
            .build()
    })
}

/// Karo indirir; geçici hatalarda (429/5xx/ağ) kısa beklemeyle bir kez daha dener.
fn download(u: &str) -> Result<Vec<u8>, String> {
    let mut last = String::new();
    for attempt in 0..2 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(900));
        }
        match agent().get(u).set("Referer", "https://emlakfethiye.com.tr/").call() {
            Ok(r) => {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut std::io::Read::take(r.into_reader(), 2 * 1024 * 1024), &mut buf).map_err(|e| e.to_string())?;
                if buf.len() < 100 {
                    return Err("boş yanıt".into());
                }
                return Ok(buf);
            }
            Err(ureq::Error::Status(c, _)) => {
                last = format!("sunucu HTTP {c} döndürdü");
                if c != 429 && c < 500 {
                    break;
                }
            }
            Err(e) => last = format!("bağlantı hatası: {e}"),
        }
    }
    Err(last)
}

impl Default for Store {
    fn default() -> Self {
        let (tx, rx) = channel();
        Store { offline_only: false, cache: Cache::with_offline(Cache::default_root(), offline_root()), tex: HashMap::new(), pending: HashSet::new(), failed: HashMap::new(), tx, rx, net_errors: 0, last_err: String::new() }
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
            let dec = data.and_then(|b| image::load_from_memory(&b).map_err(|e| format!("resim çözülemedi: {e}")));
            match dec {
                Ok(img) => {
                    self.net_errors = 0;
                    let rgba = img.to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                    let name = format!("tile{:?}{}/{}/{}", key.0, key.1, key.2, key.3);
                    self.tex.insert(key, ctx.load_texture(name, ci, egui::TextureOptions::LINEAR));
                    any = true;
                }
                Err(e) => {
                    if e != "çevrimdışı" {
                        self.net_errors += 1;
                        self.last_err = e;
                    }
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
            if t.elapsed().as_secs() < if self.offline_only { 3 } else { 30 } {
                return None;
            }
            self.failed.remove(&key);
        }
        self.pending.insert(key);
        let (tx, cache, ctx2, offline_only) = (self.tx.clone(), self.cache.clone(), ctx.clone(), self.offline_only);
        std::thread::spawn(move || {
            let data = match cache.read(s, z, x, y) {
                Some(b) => Ok(b),
                None if offline_only => Err("çevrimdışı".to_string()),
                None => download(&url(s, z, x, y)).map(|b| {
                    cache.write(s, z, x, y, &b);
                    b
                }),
            };
            let _ = tx.send(((s, z, x, y), data));
            ctx2.request_repaint();
        });
        None
    }
}

// ------------------------------------------------------------ çevrimdışı harita indirme
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// (batı, güney, doğu, kuzey) sınırlı alan için z aralığındaki tüm karolar.
pub fn tiles_for(bbox: (f64, f64, f64, f64), zmin: u32, zmax: u32) -> Vec<(u32, u32, u32)> {
    let mut v = Vec::new();
    for z in zmin..=zmax {
        let (x0, y1) = lonlat_to_tile(bbox.0, bbox.1, z);
        let (x1, y0) = lonlat_to_tile(bbox.2, bbox.3, z);
        let n = (1u64 << z) as i64;
        for x in (x0.floor() as i64).max(0)..=(x1.floor() as i64).min(n - 1) {
            for y in (y0.floor() as i64).max(0)..=(y1.floor() as i64).min(n - 1) {
                v.push((z, x as u32, y as u32));
            }
        }
    }
    v
}

pub struct Job {
    pub total: usize,
    pub done: AtomicUsize,
    pub failed: AtomicUsize,
    pub bytes: AtomicUsize,
    pub cancel: AtomicBool,
    pub finished: AtomicBool,
}

/// Karoları ağır olmadan (3 iş parçacığı, aralarda kısa bekleme) çevrimdışı klasöre indirir. Var olanlar atlanır.
pub fn start_download(dir: PathBuf, srcs: Vec<Source>, tiles: Vec<(u32, u32, u32)>, ctx: egui::Context) -> Arc<Job> {
    let list: Arc<Vec<(Source, u32, u32, u32)>> = Arc::new(srcs.iter().flat_map(|s| tiles.iter().map(move |t| (*s, t.0, t.1, t.2))).collect());
    let job = Arc::new(Job { total: list.len(), done: AtomicUsize::new(0), failed: AtomicUsize::new(0), bytes: AtomicUsize::new(0), cancel: AtomicBool::new(false), finished: AtomicBool::new(false) });
    let next = Arc::new(AtomicUsize::new(0));
    let alive = Arc::new(AtomicUsize::new(3));
    for _ in 0..3 {
        let (job, list, next, alive, dir, ctx) = (job.clone(), list.clone(), next.clone(), alive.clone(), dir.clone(), ctx.clone());
        std::thread::spawn(move || {
            let cache = Cache::new(dir);
            let mut consecutive_fail = 0;
            loop {
                if job.cancel.load(Ordering::Relaxed) || consecutive_fail > 25 {
                    break;
                }
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(&(s, z, x, y)) = list.get(i) else { break };
                if cache.read(s, z, x, y).is_some() {
                    job.done.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                match download(&url(s, z, x, y)) {
                    Ok(b) => {
                        job.bytes.fetch_add(b.len(), Ordering::Relaxed);
                        cache.write(s, z, x, y, &b);
                        job.done.fetch_add(1, Ordering::Relaxed);
                        consecutive_fail = 0;
                    }
                    Err(_) => {
                        job.failed.fetch_add(1, Ordering::Relaxed);
                        job.done.fetch_add(1, Ordering::Relaxed);
                        consecutive_fail += 1;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(70));
                if i % 20 == 0 {
                    ctx.request_repaint();
                }
            }
            if alive.fetch_sub(1, Ordering::SeqCst) == 1 {
                job.finished.store(true, Ordering::SeqCst);
                ctx.request_repaint();
            }
        });
    }
    job
}

/// Hazır bölgeler: (ad, (batı, güney, doğu, kuzey), en az z, en çok z)
pub const REGIONS: &[(&str, (f64, f64, f64, f64), u32, u32)] = &[
    ("Fethiye ve ilçeleri — genel görünüm", (28.75, 36.35, 29.60, 36.95), 10, 14),
    ("Fethiye merkez • Çalış • Karagözler", (29.07, 36.60, 29.17, 36.70), 15, 16),
    ("Hisarönü • Ovacık • Ölüdeniz • Kayaköy", (29.08, 36.52, 29.20, 36.62), 15, 16),
    ("Göcek • Dalaman yönü", (28.86, 36.72, 28.99, 36.82), 15, 16),
    ("Seydikemer • Eşen", (29.28, 36.58, 29.44, 36.72), 15, 16),
];
