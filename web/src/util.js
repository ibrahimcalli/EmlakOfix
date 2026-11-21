// Genel yardimcilar: kacis, html sablon etiketi, slug, cerez, bicimlendirme.
const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
export function esc(v) {
  return String(v ?? '').replace(/[&<>"']/g, (ch) => ESC[ch]);
}

class Raw {
  constructor(s) { this.s = String(s); }
  toString() { return this.s; }
}
export const raw = (s) => new Raw(s ?? '');
export const isRaw = (v) => v instanceof Raw;
function renderVal(v) {
  if (v instanceof Raw) return v.s;
  if (Array.isArray(v)) return v.map(renderVal).join('');
  if (v === null || v === undefined || v === false) return '';
  return esc(v);
}
// Otomatik kacisli sablon etiketi. Ham HTML icin raw() veya ic ice html`` kullanin.
export function html(strings, ...vals) {
  let out = strings[0];
  for (let i = 0; i < vals.length; i++) out += renderVal(vals[i]) + strings[i + 1];
  return new Raw(out);
}

const TR_MAP = { ç: 'c', Ç: 'c', ğ: 'g', Ğ: 'g', ı: 'i', İ: 'i', I: 'i', ö: 'o', Ö: 'o', ş: 's', Ş: 's', ü: 'u', Ü: 'u', â: 'a', Â: 'a', î: 'i', û: 'u' };
export function slugify(s, max = 80) {
  let t = String(s ?? '').replace(/[çÇğĞıİIöÖşŞüÜâÂîû]/g, (c) => TR_MAP[c] || c).toLowerCase();
  t = t.normalize('NFKD').replace(/[̀-ͯ]/g, '');
  t = t.replace(/&/g, ' ve ').replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  if (t.length > max) t = t.slice(0, max).replace(/-+$/g, '');
  return t;
}

export const nowIso = () => new Date().toISOString();
export const nowSec = () => Math.floor(Date.now() / 1000);

export function parseJson(s, fallback) {
  try {
    const v = JSON.parse(s);
    return v === null || v === undefined ? fallback : v;
  } catch { return fallback; }
}
export const clamp = (n, a, b) => Math.min(b, Math.max(a, n));
export function toInt(v, d = 0) {
  const n = parseInt(String(v ?? '').trim(), 10);
  return Number.isFinite(n) ? n : d;
}
export function toNum(v, d = 0) {
  if (typeof v === 'number') return Number.isFinite(v) ? v : d;
  const s = String(v ?? '').trim().replace(/\s/g, '');
  if (!s) return d;
  let t = s;
  if (/,/.test(t) && /\./.test(t)) t = t.replace(/\./g, '').replace(',', '.');
  else if (/,/.test(t)) t = t.replace(',', '.');
  else if (/^\d{1,3}(\.\d{3})+$/.test(t)) t = t.replace(/\./g, '');
  const n = parseFloat(t);
  return Number.isFinite(n) ? n : d;
}
export const truthy = (v) => v === true || v === 1 || v === '1' || v === 'true' || v === 'on' || v === 'evet';

// Kullanici girdisi baglantilari icin: yalniz http(s), mailto, tel, goreli yollar.
export function safeUrl(u) {
  const s = String(u ?? '').trim();
  if (!s) return '';
  if (/^(https?:\/\/|mailto:|tel:|\/(?!\/)|#)/i.test(s)) return s.replace(/[\u0000-\u001f\u007f]/g, '');
  if (/^[a-z][a-z0-9+.-]*:/i.test(s)) return '';
  return /^[\w\-./?=&%#+]+$/.test(s) ? '/' + s.replace(/^\/+/, '') : '';
}

export function parseCookies(header) {
  const out = {};
  for (const part of String(header || '').split(';')) {
    const i = part.indexOf('=');
    if (i < 0) continue;
    const k = part.slice(0, i).trim();
    if (!k) continue;
    try { out[k] = decodeURIComponent(part.slice(i + 1).trim()); } catch { out[k] = part.slice(i + 1).trim(); }
  }
  return out;
}
export function cookie(name, value, o = {}) {
  let s = `${name}=${encodeURIComponent(value)}; Path=${o.path || '/'}`;
  if (o.maxAge !== undefined) s += `; Max-Age=${o.maxAge}`;
  if (o.httpOnly) s += '; HttpOnly';
  if (o.secure) s += '; Secure';
  s += `; SameSite=${o.sameSite || 'Lax'}`;
  return s;
}

const LOCALES = { tr: 'tr-TR', en: 'en-GB', ru: 'ru-RU' };
export function fmtInt(n, lang = 'tr') {
  const v = Math.round(Number(n) || 0);
  if (lang === 'tr') return String(v).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  if (lang === 'ru') return String(v).replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
  return String(v).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}
export const CURRENCY_SYMBOLS = { TRY: '₺', TL: '₺', EUR: '€', USD: '$', GBP: '£', RUB: '₽' };
export function fmtPrice(price, currency = 'TRY', lang = 'tr') {
  const sym = CURRENCY_SYMBOLS[String(currency || 'TRY').toUpperCase()] || currency;
  return `${fmtInt(price, lang)} ${sym}`;
}
export function fmtDate(iso, lang = 'tr') {
  if (!iso) return '';
  const d = new Date(iso);
  if (isNaN(d)) return '';
  try { return new Intl.DateTimeFormat(LOCALES[lang] || 'tr-TR', { dateStyle: 'long', timeZone: 'Europe/Istanbul' }).format(d); } catch { return iso.slice(0, 10); }
}
export function fmtDateTime(iso) {
  if (!iso) return '';
  const d = new Date(iso);
  if (isNaN(d)) return '';
  try { return new Intl.DateTimeFormat('tr-TR', { dateStyle: 'short', timeStyle: 'short', timeZone: 'Europe/Istanbul' }).format(d); } catch { return iso; }
}

export function stripTags(s) {
  return String(s ?? '').replace(/<[^>]*>/g, ' ').replace(/&nbsp;/g, ' ').replace(/\s+/g, ' ').trim();
}
export function truncate(s, n) {
  const t = String(s ?? '');
  return t.length <= n ? t : t.slice(0, n - 1).replace(/\s+\S*$/, '') + '…';
}
export function pick(obj, keys) {
  const o = {};
  for (const k of keys) if (k in obj) o[k] = obj[k];
  return o;
}
export function chunk(arr, n) {
  const out = [];
  for (let i = 0; i < arr.length; i += n) out.push(arr.slice(i, i + n));
  return out;
}
export const placeholders = (n) => Array.from({ length: n }, () => '?').join(',');

export function htmlResponse(body, status = 200, headers = {}) {
  return new Response(String(body), { status, headers: { 'content-type': 'text/html; charset=utf-8', ...headers } });
}
export function jsonResponse(obj, status = 200, headers = {}) {
  return new Response(JSON.stringify(obj), { status, headers: { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', ...headers } });
}
export function textResponse(body, status = 200, type = 'text/plain; charset=utf-8', headers = {}) {
  return new Response(String(body), { status, headers: { 'content-type': type, ...headers } });
}
export function redirect(location, status = 302, cookies = []) {
  const h = new Headers({ location });
  for (const c of cookies) h.append('set-cookie', c);
  return new Response(null, { status, headers: h });
}

// D1 (Cloudflare) LIKE desen siniri ~50 bayt: daha uzun desen "LIKE or GLOB pattern too complex" hatasi verir.
// Aranan metni kacisla; desen (%..%) en fazla 46 bayt olacak sekilde kisaltir.
export function likePattern(q, maxBytes = 46) {
  const enc = new TextEncoder();
  let out = '';
  for (const ch of String(q)) {
    const piece = /[\\%_]/.test(ch) ? '\\' + ch : ch;
    if (enc.encode('%' + out + piece + '%').length > maxBytes) break;
    out += piece;
  }
  return '%' + out + '%';
}
