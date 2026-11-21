// Tum site ayarlari: tanimlar (admin formu bu listeden otomatik uretilir) + okuma/yazma.
import { all, run } from './env.js';
import { toInt, truthy } from './util.js';

export const FONT_OPTIONS = [
  'system-ui', 'Georgia, serif', 'Inter', 'Poppins', 'Montserrat', 'Roboto', 'Open Sans', 'Lato', 'Nunito', 'Playfair Display', 'Merriweather', 'Source Sans 3',
];
const SOCIAL = ['facebook', 'instagram', 'youtube', 'linkedin', 'twitter', 'pinterest', 'tiktok'];

// type: text | textarea | number | bool | color | select | image | json | code ; group: admin sekmesi
export const SETTING_DEFS = [
  // ---- Genel
  { key: 'site_name', group: 'Genel', label: 'Site adı', type: 'text', def: 'Sahibinden Fethiye' },
  { key: 'site_tagline', group: 'Genel', label: 'Slogan', type: 'text', def: 'Fethiye Gayrimenkul & Danışmanlık' },
  { key: 'logo_url', group: 'Genel', label: 'Logo (adres)', type: 'image', def: '/logo.png' },
  { key: 'favicon_url', group: 'Genel', label: 'Favicon (adres)', type: 'image', def: '/favicon.png' },
  { key: 'default_lang', group: 'Genel', label: 'Varsayılan dil', type: 'select', options: ['tr', 'en', 'ru'], def: 'tr' },
  { key: 'enabled_langs', group: 'Genel', label: 'Açık diller (virgülle)', type: 'text', def: 'tr,en,ru' },
  { key: 'seo_title_suffix', group: 'Genel', label: 'Başlık eki (SEO)', type: 'text', def: ' | Sahibinden Fethiye' },
  { key: 'seo_description', group: 'Genel', label: 'Varsayılan açıklama (SEO)', type: 'textarea', def: 'Fethiye satılık ve kiralık villa, daire, arazi ilanları.' },
  { key: 'analytics_id', group: 'Genel', label: 'Google Analytics kimliği (G-XXXX)', type: 'text', def: '' },
  { key: 'maintenance_mode', group: 'Genel', label: 'Bakım modu (ziyaretçiye kapalı)', type: 'bool', def: '0' },
  { key: 'maintenance_text', group: 'Genel', label: 'Bakım modu metni', type: 'textarea', def: 'Sitemiz kısa süreliğine bakımda. Lütfen daha sonra tekrar deneyin.' },
  // ---- İletişim
  { key: 'phone', group: 'İletişim', label: 'Telefon', type: 'text', def: '+90 541 885 58 66' },
  { key: 'whatsapp', group: 'İletişim', label: 'WhatsApp numarası (905xxxxxxxxx)', type: 'text', def: '905418855866' },
  { key: 'email', group: 'İletişim', label: 'E-posta', type: 'text', def: '' },
  { key: 'address', group: 'İletişim', label: 'Adres', type: 'textarea', def: 'Fethiye / Muğla' },
  { key: 'map_lat', group: 'İletişim', label: 'Ofis harita enlem', type: 'number', def: '36.6214' },
  { key: 'map_lon', group: 'İletişim', label: 'Ofis harita boylam', type: 'number', def: '29.1164' },
  { key: 'contact_notify_email', group: 'İletişim', label: 'Form bildirimi e-postası (kayıt amaçlı)', type: 'text', def: '' },
  ...SOCIAL.map((s) => ({ key: `social_${s}`, group: 'İletişim', label: `${s[0].toUpperCase()}${s.slice(1)} bağlantısı`, type: 'text', def: '' })),
  // ---- Renkler ve yazı tipleri
  { key: 'theme_mode', group: 'Görünüm', label: 'Tema kipi', type: 'select', options: ['light', 'dark', 'auto'], def: 'light' },
  { key: 'color_primary', group: 'Görünüm', label: 'Birincil renk', type: 'color', def: '#1b2a41' },
  { key: 'color_accent', group: 'Görünüm', label: 'Vurgu rengi', type: 'color', def: '#d4af37' },
  { key: 'color_bg', group: 'Görünüm', label: 'Arka plan', type: 'color', def: '#f4f5f7' },
  { key: 'color_surface', group: 'Görünüm', label: 'Kart/yüzey', type: 'color', def: '#ffffff' },
  { key: 'color_text', group: 'Görünüm', label: 'Yazı rengi', type: 'color', def: '#1f2937' },
  { key: 'color_muted', group: 'Görünüm', label: 'Soluk yazı', type: 'color', def: '#6b7280' },
  { key: 'color_border', group: 'Görünüm', label: 'Çerçeve', type: 'color', def: '#e3e6ea' },
  { key: 'color_header_bg', group: 'Görünüm', label: 'Üst bölüm arka planı', type: 'color', def: '#ffffff' },
  { key: 'color_header_text', group: 'Görünüm', label: 'Üst bölüm yazısı', type: 'color', def: '#1b2a41' },
  { key: 'color_footer_bg', group: 'Görünüm', label: 'Alt bilgi arka planı', type: 'color', def: '#1b2a41' },
  { key: 'color_footer_text', group: 'Görünüm', label: 'Alt bilgi yazısı', type: 'color', def: '#dbe2ee' },
  { key: 'font_heading', group: 'Görünüm', label: 'Başlık yazı tipi', type: 'select', options: FONT_OPTIONS, def: 'Montserrat' },
  { key: 'font_body', group: 'Görünüm', label: 'Gövde yazı tipi', type: 'select', options: FONT_OPTIONS, def: 'Inter' },
  { key: 'google_fonts', group: 'Görünüm', label: 'Google Fonts ile yükle', type: 'bool', def: '1' },
  { key: 'font_scale', group: 'Görünüm', label: 'Yazı boyutu (%)', type: 'number', min: 85, max: 125, def: '100' },
  { key: 'radius', group: 'Görünüm', label: 'Köşe yuvarlaklığı (px)', type: 'number', min: 0, max: 28, def: '10' },
  { key: 'card_shadow', group: 'Görünüm', label: 'Kart gölgesi', type: 'select', options: ['none', 'soft', 'strong'], def: 'soft' },
  { key: 'container_width', group: 'Görünüm', label: 'İçerik genişliği (px)', type: 'number', min: 960, max: 1700, def: '1280' },
  { key: 'header_style', group: 'Görünüm', label: 'Üst bölüm biçimi', type: 'select', options: ['classic', 'centered', 'compact'], def: 'classic' },
  { key: 'sticky_header', group: 'Görünüm', label: 'Üst bölüm sabit kalsın', type: 'bool', def: '1' },
  { key: 'show_topbar', group: 'Görünüm', label: 'Üst ince şerit (telefon/sosyal)', type: 'bool', def: '1' },
  // ---- Ana sayfa
  { key: 'home_columns', group: 'Ana sayfa', label: 'İlan sütun sayısı (2/3/4)', type: 'select', options: ['2', '3', '4'], def: '3' },
  { key: 'home_per_page', group: 'Ana sayfa', label: 'Sayfa başına ilan sayısı', type: 'number', min: 2, max: 60, def: '12' },
  { key: 'home_default_view', group: 'Ana sayfa', label: 'Varsayılan görünüm', type: 'select', options: ['thumb', 'list'], def: 'thumb' },
  { key: 'home_show_view_toggle', group: 'Ana sayfa', label: 'Thumb/Liste anahtarı görünsün', type: 'bool', def: '1' },
  { key: 'listing_sort', group: 'Ana sayfa', label: 'İlan sıralaması', type: 'select', options: ['newest', 'price_asc', 'price_desc', 'featured'], def: 'featured' },
  { key: 'show_filter_bar', group: 'Ana sayfa', label: 'Filtre çubuğu', type: 'bool', def: '1' },
  { key: 'home_intro_html', group: 'Ana sayfa', label: 'Vitrin üstü metin (HTML)', type: 'textarea', def: '' },
  { key: 'home_blog_count', group: 'Ana sayfa', label: 'Ana sayfada son yazı sayısı (0 = yok)', type: 'number', min: 0, max: 12, def: '3' },
  { key: 'banner_autoplay', group: 'Ana sayfa', label: 'Banner otomatik geçsin', type: 'bool', def: '1' },
  { key: 'banner_interval', group: 'Ana sayfa', label: 'Banner geçiş süresi (sn)', type: 'number', min: 2, max: 30, def: '6' },
  { key: 'banner_height', group: 'Ana sayfa', label: 'Banner yüksekliği (px)', type: 'number', min: 160, max: 720, def: '420' },
  // ---- Kenar çubuğu
  { key: 'sidebar_position', group: 'Kenar çubuğu', label: 'Konum', type: 'select', options: ['right', 'left', 'hidden'], def: 'right' },
  { key: 'sidebar_widgets', group: 'Kenar çubuğu', label: 'Parçalar ve sırası (JSON: search,pages,categories,news,blogposts,tags,contact)', type: 'json', def: '["search","pages","categories","news","blogposts","tags","contact"]' },
  { key: 'sidebar_on_listings', group: 'Kenar çubuğu', label: 'İlan sayfalarında kenar çubuğu', type: 'bool', def: '1' },
  { key: 'sidebar_on_blog', group: 'Kenar çubuğu', label: 'Blog sayfalarında kenar çubuğu', type: 'bool', def: '1' },
  // ---- İlan
  { key: 'map_default', group: 'İlan', label: 'İlan haritası varsayılan altlık (uydu / sokak)', type: 'select', options: ['uydu', 'sokak'], def: 'uydu' },
  { key: 'hide_exact_location', group: 'İlan', label: 'Tam konumu gizle (yaklaşık daire)', type: 'bool', def: '1' },
  { key: 'fx_show', group: 'İlan', label: 'Döviz karşılığı göster', type: 'bool', def: '0' },
  { key: 'fx_rate_eur', group: 'İlan', label: '1 € kaç ₺', type: 'number', def: '0' },
  { key: 'fx_rate_usd', group: 'İlan', label: '1 $ kaç ₺', type: 'number', def: '0' },
  { key: 'fx_rate_gbp', group: 'İlan', label: '1 £ kaç ₺', type: 'number', def: '0' },
  { key: 'show_similar', group: 'İlan', label: 'Benzer ilanlar', type: 'bool', def: '1' },
  { key: 'show_sold_listings', group: 'İlan', label: 'Satıldı/Kiralandı ilanlar da listelensin', type: 'bool', def: '1' },
  // ---- Blog
  { key: 'blog_per_page', group: 'Blog', label: 'Blog sayfa başına yazı', type: 'number', min: 3, max: 48, def: '9' },
  { key: 'blog_title', group: 'Blog', label: 'Blog başlığı', type: 'text', def: 'Blog' },
  // ---- Alt bilgi / gelişmiş
  { key: 'footer_text', group: 'Gelişmiş', label: 'Alt bilgi metni', type: 'textarea', def: '© {yıl} Sahibinden Fethiye — Tüm hakları saklıdır.' },
  { key: 'custom_css', group: 'Gelişmiş', label: 'Özel CSS', type: 'code', def: '' },
  { key: 'custom_head', group: 'Gelişmiş', label: 'Özel <head> HTML', type: 'code', def: '' },
  { key: 'custom_footer_html', group: 'Gelişmiş', label: 'Sayfa sonu HTML/JS', type: 'code', def: '' },
  { key: 'robots_extra', group: 'Gelişmiş', label: 'robots.txt eki', type: 'textarea', def: '' },
  { key: 'session_hours', group: 'Gelişmiş', label: 'Yönetici oturum süresi (saat)', type: 'number', min: 1, max: 720, def: '24' },
];
export const SETTING_GROUPS = [...new Set(SETTING_DEFS.map((d) => d.group))];
const BY_KEY = Object.fromEntries(SETTING_DEFS.map((d) => [d.key, d]));
export const defaultSettings = () => Object.fromEntries(SETTING_DEFS.map((d) => [d.key, d.def]));

// Gecerlilik: tur ve sinirlara gore temizler; gecersizse varsayilana doner.
export function normalizeSetting(key, value) {
  const d = BY_KEY[key];
  if (!d) return null;
  let v = String(value ?? '');
  if (d.type === 'bool') return truthy(v) ? '1' : '0';
  if (d.type === 'number') {
    let n = parseFloat(v.replace(',', '.'));
    if (!Number.isFinite(n)) return d.def;
    if (d.min !== undefined) n = Math.max(d.min, n);
    if (d.max !== undefined) n = Math.min(d.max, n);
    return String(n);
  }
  if (d.type === 'color') return /^#[0-9a-fA-F]{6}$/.test(v.trim()) ? v.trim() : d.def;
  if (d.type === 'select') return d.options.includes(v) ? v : d.def;
  if (d.type === 'json') { try { JSON.parse(v); return v; } catch { return d.def; } }
  return v;
}

const CACHE = new WeakMap(); // env -> {at, data}
export async function getSettings(env, fresh = false) {
  const c = CACHE.get(env);
  if (!fresh && c && Date.now() - c.at < 5000) return c.data;
  const rows = await all(env, 'SELECT key,value FROM settings');
  const data = defaultSettings();
  for (const r of rows) if (r.key in data) data[r.key] = r.value;
  CACHE.set(env, { at: Date.now(), data });
  return data;
}
export async function setSetting(env, key, value) {
  const v = normalizeSetting(key, value);
  if (v === null) return false;
  await run(env, 'INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value', key, v);
  CACHE.delete(env);
  return true;
}
export async function setSettings(env, obj) {
  let n = 0;
  for (const [k, v] of Object.entries(obj)) if (await setSetting(env, k, v)) n++;
  return n;
}
export const sInt = (s, k, d = 0) => toInt(s[k], d);
export const sBool = (s, k) => truthy(s[k]);
export function sJson(s, k, fb) { try { return JSON.parse(s[k]); } catch { return fb; } }
// kv tablosu degil, settings icinde serbest anahtar (ornegin 'seeded', 'schema_ver')
export async function getRaw(env, key) {
  const r = await all(env, 'SELECT value FROM settings WHERE key=?', key);
  return r[0] ? r[0].value : null;
}
export async function setRaw(env, key, value) {
  await run(env, 'INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value', key, String(value));
  CACHE.delete(env);
}
