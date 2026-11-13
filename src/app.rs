//! Uygulama çekirdeği: durum, arka plan işleri, doku (resim) önbelleği, ana çerçeve.
use crate::i18n_ui::ti;
use crate::api::*;
use crate::common::*;
use crate::server;
use crate::theme::{self, *};
use eframe::egui::{self, vec2, Color32, RichText};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};


pub type Cb = Box<dyn FnOnce(&mut App) + Send>;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub mode: String, // "server" | "client" | ""
    pub server_addr: String,
    pub port: u16,
    pub last_user: String,
    pub data_dir: String,
    pub look: theme::Look,
    /// Ana bilgisayar kipinde şifreli (HTTPS) bağlantı
    pub https: bool,
    /// Güvenilir yayıncı (ed25519 açık anahtarı, base64); doluysa yalnızca imzalı güncelleme kurulur
    pub trusted_key: String,
    /// Bilinen sunucu sertifika parmak izleri ("adres:port" -> sha256)
    pub pins: std::collections::BTreeMap<String, String>,
}

pub static VERIFY_MARKER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis").map(|d| d.config_dir().join("ayarlar.json"))
}
pub fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("tr", "Ofis", "EmlakOfis")
        .map(|d| d.data_dir().join("veri"))
        .unwrap_or_else(|| PathBuf::from("emlak_veri"))
}
impl Config {
    pub fn load() -> Config {
        config_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        if let Some(p) = config_path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default());
        }
    }
    pub fn port_or_default(&self) -> u16 {
        if self.port == 0 { DEFAULT_PORT } else { self.port }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Welcome,
    Login,
    Main,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Home,
    Detail,
    Form,
    Tasks,
    Attendance,
    Goals,
    Notifications,
    Profile,
    Crm,
    Users,
    Locations,
    Settings,
    Dashboard,
    Buyers,
    Calendar,
    Announcements,
    Leaves,
    Map,
    Commissions,
    Audit,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum TexKey {
    Thumb(i64),
    Full(i64),
    User(i64),
    Logo,
    Local(String),
}

pub enum Tex {
    Loading,
    Failed,
    Ready(egui::TextureHandle),
}

pub struct Toast {
    pub text: String,
    pub at: Instant,
    pub error: bool,
}

#[derive(Default)]
pub struct Filters {
    pub q: String,
    pub ltype: String,
    pub status: String,
    pub district: String,
    pub neighborhood: String,
    pub mine: bool,
    pub stale: bool,
    pub list_mode: bool,
    pub group: bool,
}

#[derive(Default)]
pub struct FormState {
    pub p: crate::common::Portfolio,
    pub is_new: bool,
    pub price_s: String,
    pub gross_s: String,
    pub net_s: String,
    pub lat_s: String,
    pub lon_s: String,
    pub link_s: String,
    pub manual_neigh: bool,
    pub pending: Vec<PathBuf>,
    pub remove: Vec<i64>,
    pub cover_existing: Option<i64>,
    pub cover_pending: Option<usize>,
    pub saving: bool,
}

pub struct ShareState {
    pub text: String,
    pub phone: String,
}

pub struct TaskForm {
    pub show: bool,
    pub title: String,
    pub details: String,
    pub kind: String,
    pub selected: HashSet<i64>,
    pub use_due: bool,
    pub date: chrono::NaiveDate,
    pub hour: i64,
    pub minute: i64,
}

pub struct GoalForm {
    pub show: bool,
    pub title: String,
    pub user_id: i64,
    pub start: chrono::NaiveDate,
    pub end: chrono::NaiveDate,
    pub t: [i64; 4],
    pub note: String,
}

#[derive(Default)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub first: String,
    pub last: String,
    pub admin: bool,
}

pub struct App {
    pub ctx: egui::Context,
    pub cfg: Config,
    pub stage: Stage,
    pub api: Option<Api>,
    pub server_state: Option<std::sync::Arc<server::State>>,
    pub tx: Sender<Cb>,
    pub rx: Receiver<Cb>,
    pub toasts: Vec<Toast>,
    pub busy: usize,
    pub offline: bool,

    // Karşılama / giriş
    pub w_mode: u8,
    pub w_addr: String,
    pub w_port: String,
    pub found: Vec<String>,
    pub set_tab: u8,
    pub conn_msg: String,
    pub conn_busy: bool,
    pub test_res: Vec<crate::selftest::TestResult>,
    pub test_busy: bool,
    pub test_done: bool,
    pub server_ver: String,
    pub latency: Option<u128>,
    pub fonts: Vec<(String, String)>,
    pub addr_edit: String,
    pub searching: bool,
    pub connect_msg: String,
    pub needs_setup: bool,
    pub l_user: String,
    pub l_pass: String,
    pub su_first: String,
    pub su_last: String,
    pub su_pass2: String,

    // Oturum
    pub me: UserInfo,
    pub users: Vec<UserInfo>,
    pub locations: Vec<District>,
    pub settings: OutputSettings,
    pub view: View,
    pub rev: u64,
    pub last_poll: Instant,
    pub polling: bool,
    pub unread: i64,
    pub tex: HashMap<TexKey, Tex>,
    pub inflight: usize,

    // Ana ekran
    pub portfolios: Vec<PortfolioSummary>,
    pub filt: Filters,
    pub show_n: usize,
    pub list_seq: u64,

    // Detay
    pub detail_id: i64,
    pub ext: crate::ui_ext::ExtState,
    pub detail: Option<crate::common::Portfolio>,
    pub photo_idx: usize,
    pub share: Option<ShareState>,
    pub confirm_delete: bool,
    pub agent_me: bool,
    pub exporting: bool,
    pub last_export: Option<PathBuf>,
    pub doc_lang: crate::i18n_doc::Lang,
    pub thumb_w: f32,
    pub bg_tex: Option<egui::TextureHandle>,
    pub bg_key: String,
    pub bg_ver: u32,
    pub bg_msg: String,
    pub logo_tex: Option<egui::TextureHandle>,

    pub form: FormState,

    // Çalışma
    pub tasks: Vec<TaskInfo>,
    pub task_filter: u8,
    pub task_user: i64,
    pub tform: TaskForm,
    pub att: Vec<AttInfo>,
    pub att_month: chrono::NaiveDate,
    pub att_user: i64,
    pub att_edit: Option<AttInfo>,
    pub goals: Vec<GoalInfo>,
    pub gform: GoalForm,
    pub notifs: Vec<NotifInfo>,

    // Yönetim
    pub crm: Vec<CrmContact>,
    pub crm_q: String,
    pub crm_edit: Option<CrmContact>,
    pub new_user: NewUser,
    pub edit_user: Option<UserInfo>,
    pub reset_pw: Option<(i64, String)>,
    pub loc_sel: i64,
    pub new_district: String,
    pub new_neigh: String,
    pub set_edit: OutputSettings,
    pub server_info: Option<ServerInfo>,

    // Profil
    pub prof: UserInfo,
    pub pw_old: String,
    pub pw_new: String,
    pub pw_new2: String,
}

pub fn first_of_month(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
}
pub fn last_of_month(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    let (y, m) = if d.month() == 12 { (d.year() + 1, 1) } else { (d.year(), d.month() + 1) };
    chrono::NaiveDate::from_ymd_opt(y, m, 1).map(|n| n.pred_opt().unwrap_or(n)).unwrap_or(d)
}

pub fn decode_image(bytes: &[u8], max: u32) -> Result<egui::ColorImage, String> {
    use image::{DynamicImage, ImageDecoder, ImageReader};
    let reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orient = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    img.apply_orientation(orient);
    if img.width() > max || img.height() > max {
        img = img.thumbnail(max, max);
    }
    let rgba = img.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

pub fn status_color(s: &str) -> Color32 {
    match s {
        "Yayında" => Color32::from_rgb(34, 139, 70),
        "Taslak" => Color32::from_rgb(120, 120, 130),
        "Satıldı" => Color32::from_rgb(190, 55, 55),
        "Kiralandı" => Color32::from_rgb(45, 100, 190),
        _ => Color32::from_rgb(90, 90, 90),
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        let cfg = Config::load();
        crate::api::set_pins(cfg.pins.clone());
        crate::updater::set_trusted_key(&cfg.trusted_key);
        crate::i18n_ui::set_lang(crate::i18n_ui::UiLang::from_code(&cfg.look.lang));
        theme::apply(&ctx, &cfg.look);

        let (tx, rx) = channel();
        let today = chrono::Local::now().date_naive();
        let mut app = App {
            ctx,
            stage: Stage::Welcome,
            api: None,
            server_state: None,
            tx,
            rx,
            toasts: vec![],
            busy: 0,
            offline: false,
            w_mode: if cfg.mode == "client" { 1 } else { 0 },
            w_addr: cfg.server_addr.clone(),
            w_port: cfg.port_or_default().to_string(),
            found: vec![],
            searching: false,
            connect_msg: String::new(),
            needs_setup: false,
            l_user: cfg.last_user.clone(),
            l_pass: String::new(),
            su_first: String::new(),
            su_last: String::new(),
            su_pass2: String::new(),
            me: UserInfo::default(),
            users: vec![],
            locations: vec![],
            settings: OutputSettings::default(),
            view: View::Home,
            ext: crate::ui_ext::ExtState::new(),
            rev: 0,
            last_poll: Instant::now(),
            polling: false,
            unread: 0,
            tex: HashMap::new(),
            inflight: 0,
            portfolios: vec![],
            filt: Filters::default(),
            show_n: 60,
            list_seq: 0,
            detail_id: 0,
            detail: None,
            photo_idx: 0,
            share: None,
            confirm_delete: false,
            agent_me: false,
            exporting: false,
            last_export: None,
            doc_lang: crate::i18n_doc::Lang::Tr,
            thumb_w: 250.0,
            bg_tex: None,
            bg_key: String::new(),
            bg_ver: 0,
            bg_msg: String::new(),
            logo_tex: None,
            form: FormState::default(),
            tasks: vec![],
            task_filter: 0,
            task_user: 0,
            tform: TaskForm {
                show: false,
                title: String::new(),
                details: String::new(),
                kind: "Görev".into(),
                selected: HashSet::new(),
                use_due: false,
                date: today,
                hour: 10,
                minute: 0,
            },
            att: vec![],
            att_month: first_of_month(today),
            att_user: 0,
            att_edit: None,
            goals: vec![],
            gform: GoalForm { show: false, title: String::new(), user_id: 0, start: first_of_month(today), end: last_of_month(today), t: [5, 2, 1, 0], note: String::new() },
            notifs: vec![],
            crm: vec![],
            crm_q: String::new(),
            crm_edit: None,
            new_user: NewUser::default(),
            edit_user: None,
            reset_pw: None,
            loc_sel: 0,
            new_district: String::new(),
            new_neigh: String::new(),
            set_edit: OutputSettings::default(),
            server_info: None,
            prof: UserInfo::default(),
            pw_old: String::new(),
            pw_new: String::new(),
            pw_new2: String::new(),
            set_tab: 0,
            conn_msg: String::new(),
            conn_busy: false,
            test_res: Vec::new(),
            test_busy: false,
            test_done: false,
            server_ver: String::new(),
            latency: None,
            fonts: theme::available_fonts(),
            addr_edit: cfg.server_addr.clone(),
            cfg,
        };
        app.filt.grid_default();
        if let Some((ok, msg)) = crate::restore::take_notice() {
            if ok {
                app.toast(msg);
            } else {
                app.err(format!("Geri yükleme yapılamadı: {msg}"));
            }
        }
        if let Some(why) = crate::updater::take_rollback_notice() {
            app.err(format!("Güncelleme açılışta sorun çıkardığı için önceki sürüme dönüldü ({why})."));
        }
        if !app.cfg.mode.is_empty() {
            app.connect();
        }
        app
    }

    // ------------------------------------------------------------ arka plan işleri
    pub fn bg<T: Send + 'static>(&mut self, work: impl FnOnce(Api) -> T + Send + 'static, done: impl FnOnce(&mut App, T) + Send + 'static) {
        let Some(api) = self.api.clone() else { return };
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.busy += 1;
        std::thread::spawn(move || {
            let r = work(api);
            let _ = tx.send(Box::new(move |app: &mut App| {
                app.busy = app.busy.saturating_sub(1);
                done(app, r)
            }));
            ctx.request_repaint();
        });
    }

    pub fn call<T: Send + 'static>(&mut self, work: impl FnOnce(Api) -> Result<T, String> + Send + 'static, ok: impl FnOnce(&mut App, T) + Send + 'static) {
        self.bg(work, move |app, r| match r {
            Ok(v) => ok(app, v),
            Err(e) => app.err(e),
        });
    }

    pub fn toast(&mut self, t: impl Into<String>) {
        self.toasts.push(Toast { text: t.into(), at: Instant::now(), error: false });
    }
    pub fn err(&mut self, t: impl Into<String>) {
        let t: String = t.into();
        if t.contains("Oturum süresi doldu") && self.stage == Stage::Main {
            self.logout(false);
            self.connect_msg = "Oturum süresi doldu, lütfen yeniden giriş yapın.".into();
            return;
        }
        self.toasts.push(Toast { text: t, at: Instant::now(), error: true });
    }

    // ------------------------------------------------------------ bağlantı / oturum
    pub fn connect(&mut self) {
        self.connect_msg.clear();
        match self.cfg.mode.as_str() {
            "server" => {
                let dir = if self.cfg.data_dir.is_empty() { default_data_dir() } else { PathBuf::from(&self.cfg.data_dir) };
                let port = self.cfg.port_or_default();
                if self.server_state.is_none() {
                    match server::start_with(dir, port, self.cfg.https) {
                        Ok(st) => self.server_state = Some(st),
                        Err(e) => {
                            self.connect_msg = e;
                            self.stage = Stage::Welcome;
                            return;
                        }
                    }
                }
                let me = format!("127.0.0.1:{port}");
                match self.server_state.as_ref().and_then(|s| s.tls_fp.clone()) {
                    Some(fp) => crate::api::learn_pin(&me, &fp),
                    None => crate::api::forget_pin(&me),
                }
                self.api = Some(Api::new(&me));
            }
            _ => {
                self.api = Some(Api::new(&self.cfg.server_addr.clone()));
            }
        }
        self.stage = Stage::Login;
        self.ping();
    }

    pub fn ping(&mut self) {
        self.bg(|api| api.ping(), |app, r| match r {
            Ok(p) => {
                app.needs_setup = p.needs_setup;
                app.server_ver = p.version.clone();
                app.offline = false;
                // İlk HTTPS bağlantısı: sunucu sertifikasının parmak izini kaydet (TOFU)
                if let Some(a) = &app.api {
                    if a.is_https() {
                        if let Some(fp) = a.seen_fingerprint() {
                            if !crate::api::get_pins().contains_key(&a.host()) {
                                crate::api::learn_pin(&a.host(), &fp);
                                app.cfg.pins = crate::api::get_pins();
                                app.cfg.save();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                app.offline = true;
                app.connect_msg = e;
            }
        });
    }

    pub fn do_login(&mut self) {
        let (u, p) = (self.l_user.trim().to_string(), self.l_pass.clone());
        if u.is_empty() || p.is_empty() {
            self.connect_msg = "Kullanıcı adı ve şifre girin.".into();
            return;
        }
        self.connect_msg.clear();
        self.bg(move |api| api.login(&u, &p), |app, r| match r {
            Ok(rep) => app.after_login(rep.user),
            Err(e) => app.connect_msg = e,
        });
    }

    pub fn after_login(&mut self, user: UserInfo) {
        self.cfg.last_user = user.username.clone();
        self.cfg.save();
        self.me = user;
        self.l_pass.clear();
        self.stage = Stage::Main;
        self.view = View::Home;
        self.rev = 0;
        self.unread = 0;
        self.tex.clear();
        self.last_poll = Instant::now() - Duration::from_secs(10);
        self.load_users();
        self.load_locations();
        self.load_settings();
        self.load_portfolios();
        self.load_update_info();
        self.load_anns();
    }

    pub fn logout(&mut self, tell_server: bool) {
        if tell_server {
            self.bg(|api| api.post("/api/logout", &serde_json::json!({})), |_, _| {});
        }
        if let Some(api) = &self.api {
            api.set_token("");
        }
        self.stage = Stage::Login;
        self.me = UserInfo::default();
        self.portfolios.clear();
        self.detail = None;
        self.tex.clear();
        self.l_pass.clear();
        self.ping();
    }

    // ------------------------------------------------------------ yükleyiciler
    pub fn load_users(&mut self) {
        self.call(|api| api.get::<Vec<UserInfo>>("/api/users"), |app, v| app.users = v);
    }
    pub fn load_locations(&mut self) {
        self.call(|api| api.get::<Vec<District>>("/api/locations"), |app, v| app.locations = v);
    }
    pub fn load_settings(&mut self) {
        self.call(|api| api.get::<OutputSettings>("/api/settings"), |app, v| {
            app.settings = v.clone();
            app.set_edit = v;
        });
    }
    pub fn load_portfolios(&mut self) {
        self.list_seq += 1;
        let seq = self.list_seq;
        let f = &self.filt;
        let mut path = String::from("/api/portfolios?limit=3000");
        for (k, v) in [("q", &f.q), ("type", &f.ltype), ("status", &f.status), ("district", &f.district), ("neighborhood", &f.neighborhood)] {
            if !v.is_empty() {
                path.push_str(&format!("&{}={}", k, url_encode(v)));
            }
        }
        if f.mine {
            path.push_str("&mine=1");
        }
        if f.stale {
            path.push_str("&stale=1");
        }
        self.bg(move |api| api.get::<Vec<PortfolioSummary>>(&path), move |app, r| {
            if seq != app.list_seq {
                return;
            }
            match r {
                Ok(v) => app.portfolios = v,
                Err(e) => app.err(e),
            }
        });
    }
    pub fn load_detail(&mut self, id: i64) {
        self.call(move |api| api.get::<Portfolio>(&format!("/api/portfolios/{id}")), move |app, p| {
            if app.detail_id == id {
                if app.photo_idx >= p.photos.len() {
                    app.photo_idx = 0;
                }
                app.detail = Some(p);
            }
        });
    }
    pub fn load_tasks(&mut self) {
        let all = self.me.is_admin();
        self.call(move |api| api.get::<Vec<TaskInfo>>(&format!("/api/tasks?all={}", if all { 1 } else { 0 })), |app, v| app.tasks = v);
    }
    pub fn load_att(&mut self) {
        let month = self.att_month.format("%Y-%m").to_string();
        let uid = if self.me.is_admin() { self.att_user } else { self.me.id };
        self.call(move |api| api.get::<Vec<AttInfo>>(&format!("/api/attendance?month={month}&user_id={uid}")), |app, v| app.att = v);
    }
    pub fn load_goals(&mut self) {
        self.call(|api| api.get::<Vec<GoalInfo>>("/api/goals"), |app, v| app.goals = v);
    }
    pub fn load_crm(&mut self) {
        if self.me.is_admin() {
            self.call(|api| api.get::<Vec<CrmContact>>("/api/crm"), |app, v| app.crm = v);
        }
    }
    pub fn load_notifs(&mut self, mark_read: bool) {
        self.call(|api| api.get::<Vec<NotifInfo>>("/api/notifications"), move |app, v| {
            app.notifs = v;
            if mark_read {
                app.unread = 0;
                app.bg(|api| api.post("/api/notifications/read_all", &serde_json::json!({})), |_, _| {});
            }
        });
    }
    pub fn load_server_info(&mut self) {
        if self.me.is_admin() {
            self.call(|api| api.get::<ServerInfo>("/api/server_info"), |app, v| app.server_info = Some(v));
        }
    }

    pub fn goto(&mut self, v: View) {
        self.view = v;
        match v {
            View::Home => {
                self.load_portfolios();
                self.load_anns();
                self.load_update_info();
            }
            View::Tasks => {
                self.load_tasks();
                self.load_users();
            }
            View::Attendance => self.load_att(),
            View::Goals => {
                self.load_goals();
                self.load_users();
            }
            View::Notifications => self.load_notifs(true),
            View::Crm => self.load_crm(),
            View::Users => self.load_users(),
            View::Locations => self.load_locations(),
            View::Settings => {
                self.load_settings();
                self.load_server_info();
            }
            View::Profile => {
                self.prof = self.me.clone();
                self.pw_old.clear();
                self.pw_new.clear();
                self.pw_new2.clear();
            }
            View::Dashboard => {
                self.load_dashboard();
                self.load_anns();
            }
            View::Buyers => self.load_buyers(),
            View::Calendar => {
                self.load_appts();
                self.load_buyers();
                self.load_users();
            }
            View::Announcements => self.load_anns(),
            View::Leaves => {
                self.load_leaves();
                self.load_users();
            }
            View::Map => self.load_map(),
            View::Commissions => {
                self.load_comms();
                self.load_users();
            }
            View::Audit => self.load_audit(),
            View::Detail | View::Form => {}
        }
    }

    pub fn open_detail(&mut self, id: i64) {
        self.detail_id = id;
        self.detail = None;
        self.photo_idx = 0;
        self.share = None;
        self.confirm_delete = false;
        self.last_export = None;
        self.agent_me = true;
        self.view = View::Detail;
        self.ext.hist.clear();
        self.ext.notes.clear();
        self.ext.matches.clear();
        self.load_detail(id);
        self.load_detail_extras(id);
    }

    /// Sunucuda değişiklik olduğunda (başka biri kayıt eklediğinde vb.) açık ekranı yeniler.
    pub fn refresh_current(&mut self) {
        match self.view {
            View::Home => self.load_portfolios(),
            View::Detail => {
                let id = self.detail_id;
                self.load_detail(id);
            }
            View::Tasks => self.load_tasks(),
            View::Attendance => self.load_att(),
            View::Goals => self.load_goals(),
            View::Crm => self.load_crm(),
            View::Users => self.load_users(),
            View::Locations => self.load_locations(),
            View::Dashboard => self.load_dashboard(),
            View::Buyers => self.load_buyers(),
            View::Calendar => self.load_appts(),
            View::Announcements => self.load_anns(),
            View::Leaves => self.load_leaves(),
            View::Map => self.load_map(),
            View::Commissions => self.load_comms(),
            View::Audit => self.load_audit(),
            _ => {}
        }
        if matches!(self.view, View::Form) {
            self.load_locations();
        }
    }

    fn poll(&mut self) {
        if self.polling || self.last_poll.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.polling = true;
        self.bg(|api| api.get::<RevReply>("/api/rev"), |app, r| {
            app.polling = false;
            app.last_poll = Instant::now();
            match r {
                Ok(rv) => {
                    app.offline = false;
                    let first = app.rev == 0;
                    if rv.rev != app.rev {
                        app.rev = rv.rev;
                        if !first {
                            app.refresh_current();
                        }
                    }
                    if rv.unread != app.unread {
                        if first && rv.unread > 0 {
                            app.toast(format!("{} okunmamış bildiriminiz var.", rv.unread));
                        } else if rv.unread > app.unread {
                            app.call(|api| api.get::<Vec<NotifInfo>>("/api/notifications"), |app, v| {
                                if let Some(n) = v.iter().find(|n| !n.read) {
                                    app.toast(format!("{}: {}", n.kind, n.message));
                                }
                                app.notifs = v;
                            });
                        }
                        app.unread = rv.unread;
                    }
                }
                Err(e) => {
                    if e.contains("Oturum süresi doldu") {
                        app.err(e);
                    } else {
                        app.offline = true;
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------ doku önbelleği
    pub fn tex(&mut self, key: TexKey) -> Option<egui::TextureHandle> {
        match self.tex.get(&key) {
            Some(Tex::Ready(h)) => return Some(h.clone()),
            Some(_) => return None,
            None => {}
        }
        if self.inflight >= 6 {
            self.ctx.request_repaint_after(Duration::from_millis(80));
            return None;
        }
        self.inflight += 1;
        self.tex.insert(key.clone(), Tex::Loading);
        let k2 = key.clone();
        let (path, max) = match &key {
            TexKey::Thumb(id) => (format!("/api/photos/{id}?size=thumb"), 400),
            TexKey::Full(id) => (format!("/api/photos/{id}"), 1600),
            TexKey::User(id) => (format!("/api/users/{id}/photo"), 256),
            TexKey::Logo => ("/api/settings/logo".to_string(), 400),
            TexKey::Local(_) => (String::new(), 360),
        };
        let name = format!("{:?}", key);
        self.bg(
            move |api| {
                let bytes = match &k2 {
                    TexKey::Local(p) => std::fs::read(p).map_err(|e| e.to_string())?,
                    _ => api.get_bytes(&path)?,
                };
                decode_image(&bytes, max)
            },
            move |app, r| {
                app.inflight = app.inflight.saturating_sub(1);
                match r {
                    Ok(img) => {
                        let h = app.ctx.load_texture(name, img, egui::TextureOptions::LINEAR);
                        app.tex.insert(key, Tex::Ready(h));
                    }
                    Err(_) => {
                        app.tex.insert(key, Tex::Failed);
                    }
                }
            },
        );
        None
    }

    // ------------------------------------------------------------ ana çerçeve
    fn ui_main(&mut self, ctx: &egui::Context) {
        use egui::{Align2, CornerRadius, FontId, Margin, Sense, Stroke};
        let side_logo = self.logo_handle();
        egui::TopBottomPanel::top("top")
            .frame(egui::Frame::new().fill(card()).stroke(Stroke::new(1.0_f32, stroke())).inner_margin(Margin::symmetric(16, 9)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(t) = &side_logo {
                        let h = 38.0;
                        ui.image((t.id(), vec2(h * t.size_vec2().x / t.size_vec2().y, h)));
                    }
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(self.settings.office_name.clone()).size(18.0).strong().color(t_strong()));
                        let sub = if self.settings.tagline.trim().is_empty() { "Portföy • CRM • Mesai" } else { self.settings.tagline.trim() };
                        ui.label(RichText::new(sub).small().color(muted()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(ti("Çıkış")).clicked() {
                            self.logout(true);
                        }
                        let label = if self.unread > 0 { format!("🔔 {}", self.unread) } else { "Bildirimler".to_string() };
                        let b = if self.unread > 0 { egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(Color32::from_rgb(200, 60, 40)) } else { egui::Button::new(label) };
                        if ui.add(b).on_hover_text(ti("Bildirimler")).clicked() {
                            self.goto(View::Notifications);
                        }
                        ui.add_space(6.0);
                        let nm = self.me.full_name();
                        let w = ui.painter().layout_no_wrap(nm.clone(), FontId::proportional(15.0), t_strong()).size().x.max(70.0) + 6.0;
                        ui.allocate_ui_with_layout(vec2(w, 34.0), egui::Layout::top_down(egui::Align::Max), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(nm).strong().color(t_strong()));
                            ui.label(RichText::new(if self.me.is_admin() { "Yönetici" } else { "Çalışan" }).small().color(muted()));
                        });
                        if self.me.has_photo {
                            if let Some(t) = self.tex(TexKey::User(self.me.id)) {
                                ui.add(egui::Image::from_texture((t.id(), vec2(34.0, 34.0))).corner_radius(17.0));
                            }
                        } else {
                            let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 17.0, accent());
                            let ini: String = self.me.full_name().split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, ini, FontId::proportional(14.0), on_color(accent()));
                        }
                    });
                });
            });

        if self.offline {
            egui::TopBottomPanel::top("offline")
                .frame(egui::Frame::new().fill(Color32::from_rgb(176, 42, 42)).inner_margin(Margin::symmetric(16, 5)))
                .show(ctx, |ui| {
                    ui.label(RichText::new(ti("Ana bilgisayara ulaşılamıyor — Wi-Fi/ağ bağlantısını ve ana bilgisayarın açık olduğunu kontrol edin. Otomatik olarak yeniden deneniyor…")).color(Color32::WHITE));
                });
        }

        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::new().fill(card()).stroke(Stroke::new(1.0_f32, stroke())).inner_margin(Margin::symmetric(14, 4)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.toasts.retain(|t| t.at.elapsed() < Duration::from_secs(9));
                    if let Some(t) = self.toasts.last() {
                        let c = if t.error { Color32::from_rgb(200, 60, 60) } else { Color32::from_rgb(40, 150, 80) };
                        ui.label(RichText::new(&t.text).color(c));
                    } else {
                        ui.label(RichText::new(ti("Hazır")).color(muted()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (dot, txt) = if self.offline { (Color32::from_rgb(200, 60, 60), "Bağlantı yok") } else { (Color32::from_rgb(40, 160, 85), "Bağlı") };
                        ui.label(RichText::new(txt).color(dot).strong());
                        ui.label(RichText::new(ti("●")).color(dot));
                        if self.busy > 0 {
                            ui.spinner();
                        }
                        if self.server_state.is_some() {
                            let ips = server::local_ips();
                            ui.label(RichText::new(format!("Bu bilgisayar ana bilgisayar — adres: {}:{}", ips.first().cloned().unwrap_or("?".into()), self.cfg.port_or_default())).small().color(muted()));
                        }
                    });
                });
            });

        egui::SidePanel::left("nav")
            .exact_width(214.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(sidebar_bg()).inner_margin(Margin::symmetric(10, 14)))
            .show(ctx, |ui| {
              egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(6.0);
                let mut go: Option<View> = None;
                let mut new_pf = false;
                let cur = self.view;
                let mut item = |ui: &mut egui::Ui, label: &str, v: View, active: bool| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 33.0), Sense::click());
                    let sel = nav_sel();
                    if active {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), sel);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(16));
                    }
                    let col = if active { on_color(sel) } else { sidebar_text() };
                    ui.painter().text(rect.left_center() + vec2(14.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(15.5), col);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        go = Some(v);
                    }
                };
                let head = |ui: &mut egui::Ui, t: &str| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(t).size(11.5).color(sidebar_dim()).strong());
                    ui.add_space(2.0);
                };
                ui.spacing_mut().item_spacing.y = 3.0;
                head(ui, "PORTFÖY");
                item(ui, ti("Panel"), View::Dashboard, cur == View::Dashboard);
                item(ui, ti("Portföyler"), View::Home, matches!(cur, View::Home | View::Detail));
                {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                    let active = cur == View::Form;
                    if active {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), nav_sel());
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(16));
                    }
                    let col = if active { on_color(nav_sel()) } else { sidebar_text() };
                    ui.painter().text(rect.left_center() + vec2(14.0, 0.0), Align2::LEFT_CENTER, "+  Yeni Portföy", FontId::proportional(15.5), col);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        new_pf = true;
                    }
                }
                item(ui, ti("Harita"), View::Map, cur == View::Map);
                item(ui, ti("Alıcılar"), View::Buyers, cur == View::Buyers);
                head(ui, "ÇALIŞMA");
                item(ui, ti("Takvim"), View::Calendar, cur == View::Calendar);
                item(ui, ti("Duyurular"), View::Announcements, cur == View::Announcements);
                item(ui, if self.me.is_admin() { "Görevler" } else { "Görevlerim" }, View::Tasks, cur == View::Tasks);
                item(ui, ti("Mesai"), View::Attendance, cur == View::Attendance);
                item(ui, ti("Hedefler"), View::Goals, cur == View::Goals);
                item(ui, ti("İzin / Puantaj"), View::Leaves, cur == View::Leaves);
                head(ui, "HESAP");
                item(ui, ti("Profilim"), View::Profile, cur == View::Profile);
                item(ui, ti("Ayarlar"), View::Settings, cur == View::Settings);
                if self.me.is_admin() {
                    head(ui, "YÖNETİM");
                    item(ui, ti("CRM (Mülk Sahipleri)"), View::Crm, cur == View::Crm);
                    item(ui, ti("Çalışanlar"), View::Users, cur == View::Users);
                    item(ui, ti("İlçe / Mahalleler"), View::Locations, cur == View::Locations);
                    item(ui, ti("Komisyonlar"), View::Commissions, cur == View::Commissions);
                    item(ui, ti("Denetim kaydı"), View::Audit, cur == View::Audit);
                }
                ui.add_space(8.0);
                ui.label(RichText::new(format!("{}  v{}", APP_NAME, env!("CARGO_PKG_VERSION"))).small().color(sidebar_dim()));
                if new_pf {
                    self.start_new_portfolio();
                } else if let Some(v) = go {
                    self.goto(v);
                }
              });
            });

        egui::CentralPanel::default().frame(egui::Frame::new().fill(panel()).inner_margin(egui::Margin::same(18))).show(ctx, |ui| match self.view {
            View::Home => self.ui_home(ui),
            View::Detail => self.ui_detail(ui),
            View::Form => self.ui_form(ui),
            View::Tasks => self.ui_tasks(ui),
            View::Attendance => self.ui_attendance(ui),
            View::Goals => self.ui_goals(ui),
            View::Notifications => self.ui_notifications(ui),
            View::Profile => self.ui_profile(ui),
            View::Crm => self.ui_crm(ui),
            View::Users => self.ui_users(ui),
            View::Locations => self.ui_locations(ui),
            View::Settings => self.ui_settings(ui),
            View::Dashboard => self.ui_dashboard(ui),
            View::Buyers => self.ui_buyers(ui),
            View::Calendar => self.ui_calendar(ui),
            View::Announcements => self.ui_announcements(ui),
            View::Leaves => self.ui_leaves(ui),
            View::Map => self.ui_map(ui),
            View::Commissions => self.ui_commissions(ui),
            View::Audit => self.ui_audit(ui),
        });
    }
}

impl Filters {
    pub fn grid_default(&mut self) {
        self.list_mode = false;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.ext.verify_pending {
            if let Some(m) = VERIFY_MARKER.get() {
                if self.ext.born.elapsed() > crate::updater::VERIFY_AFTER {
                    let _ = std::fs::write(m, crate::updater::current_version());
                    self.ext.verify_pending = false;
                    self.toast(format!("Güncelleme tamamlandı: v{}", crate::updater::current_version()));
                }
            } else {
                self.ext.verify_pending = false;
            }
        }
        while let Ok(cb) = self.rx.try_recv() {
            cb(self);
        }
        match self.stage {
            Stage::Welcome => self.ui_welcome(ctx),
            Stage::Login => self.ui_login(ctx),
            Stage::Main => {
                self.poll();
                self.ui_main(ctx);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

// ---------------------------------------------------------------- ortak yardımcılar
impl App {
    /// API gerektirmeyen arka plan işi (ör. ağda arama, dosya seçme penceresi).
    pub fn spawn<T: Send + 'static>(&mut self, work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(&mut App, T) + Send + 'static) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let r = work();
            let _ = tx.send(Box::new(move |app: &mut App| done(app, r)));
            ctx.request_repaint();
        });
    }

    pub fn pick_files(&mut self, title: &str, exts: &'static [&'static str], multiple: bool, done: impl FnOnce(&mut App, Vec<PathBuf>) + Send + 'static) {
        let title = title.to_string();
        self.spawn(
            move || {
                let d = rfd::FileDialog::new().set_title(&title).add_filter("Dosyalar", exts);
                if multiple { d.pick_files().unwrap_or_default() } else { d.pick_file().into_iter().collect() }
            },
            done,
        );
    }

    pub fn save_dialog(&mut self, name: &str, ext: &'static str, done: impl FnOnce(&mut App, Option<PathBuf>) + Send + 'static) {
        let name = name.to_string();
        self.spawn(
            move || {
                let mut d = rfd::FileDialog::new().set_file_name(&name).add_filter(ext, &[ext]);
                if let Some(docs) = directories::UserDirs::new().and_then(|u| u.document_dir().map(|p| p.to_path_buf())) {
                    d = d.set_directory(docs);
                }
                d.save_file()
            },
            done,
        );
    }

    pub fn open_url(&mut self, url: &str) {
        if let Err(e) = open::that(url) {
            self.err(format!("Tarayıcı açılamadı: {e}"));
        }
    }

    pub fn open_path(&mut self, p: &std::path::Path) {
        if let Err(e) = open::that(p) {
            self.err(format!("Açılamadı: {e}"));
        }
    }
}

pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(format!(" {text} ")).small().strong().color(Color32::WHITE).background_color(color));
}

/// İki aşamalı onay düğmesi: ilk tıklama "Emin misiniz?" gösterir, ikincisi true döner.
pub fn confirm_btn(ui: &mut egui::Ui, key: &str, label: &str) -> bool {
    let id = ui.make_persistent_id(("confirm", key));
    let armed = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if armed {
        let b = egui::Button::new(RichText::new(ti("Emin misiniz? Sil")).color(Color32::WHITE)).fill(Color32::from_rgb(190, 40, 40));
        if ui.add(b).clicked() {
            ui.data_mut(|d| d.insert_temp(id, false));
            return true;
        }
        if ui.small_button(ti("Vazgeç")).clicked() {
            ui.data_mut(|d| d.insert_temp(id, false));
        }
    } else if ui.button(label).clicked() {
        ui.data_mut(|d| d.insert_temp(id, true));
    }
    false
}

pub fn page_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).size(24.0).strong().color(accent()));
    ui.add_space(4.0);
}

/// "2026-11-07 17:58:00" -> "07.11.2026 17:58"
pub fn dt_tr(s: &str) -> String {
    match chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")) {
        Ok(d) => d.format("%d.%m.%Y %H:%M").to_string(),
        Err(_) => date_tr(s),
    }
}

/// "2026-11-07..." -> "07.11.2026"
pub fn date_tr(s: &str) -> String {
    match chrono::NaiveDate::parse_from_str(s.get(..10).unwrap_or(""), "%Y-%m-%d") {
        Ok(d) => d.format("%d.%m.%Y").to_string(),
        Err(_) => s.to_string(),
    }
}

/// Türkçe yazılmış sayıyı çözer: "1.250.000", "85,5", "120".
pub fn parse_num(s: &str, money: bool) -> f64 {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return 0.0;
    }
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else if money || t.matches('.').count() > 1 {
        t.replace('.', "")
    } else {
        t
    };
    t.parse::<f64>().unwrap_or(0.0)
}

pub fn num_to_text(v: f64) -> String {
    if v <= 0.0 {
        String::new()
    } else if (v - v.round()).abs() < 0.0001 {
        format!("{}", v.round() as i64)
    } else {
        format!("{}", v).replace('.', ",")
    }
}
