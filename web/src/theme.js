// Tema: ayarlari CSS degiskenlerine cevirir, Google Fonts baglantisi uretir.
import { FONT_OPTIONS } from './settings.js';
import { esc, toInt } from './util.js';

const HEX = /^#[0-9a-fA-F]{6}$/;
const col = (v, d) => (HEX.test(String(v || '').trim()) ? String(v).trim() : d);
const SERIF = new Set(['Georgia, serif', 'Playfair Display', 'Merriweather']);

function lum(hex) {
  const n = parseInt(hex.slice(1), 16);
  const f = (c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * f((n >> 16) & 255) + 0.7152 * f((n >> 8) & 255) + 0.0722 * f(n & 255);
}
// Verilen zemin uzerinde okunur yazi rengi
export const onColor = (bg) => (lum(bg) > 0.45 ? '#111827' : '#ffffff');

function fontName(v, d) { return FONT_OPTIONS.includes(v) ? v : d; }
export function fontStack(name) {
  if (name === 'system-ui') return "system-ui,-apple-system,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif";
  if (name === 'Georgia, serif') return 'Georgia,Cambria,"Times New Roman",serif';
  return `'${name}',${SERIF.has(name) ? 'Georgia,serif' : "system-ui,-apple-system,'Segoe UI',Arial,sans-serif"}`;
}
const isGoogle = (n) => n !== 'system-ui' && n !== 'Georgia, serif';

// <link> etiketleri (google_fonts=1 ise)
export function fontLinks(s) {
  if (String(s.google_fonts) !== '1') return '';
  const names = [...new Set([fontName(s.font_heading, 'Montserrat'), fontName(s.font_body, 'Inter')].filter(isGoogle))];
  if (!names.length) return '';
  const fam = names.map((n) => `family=${n.replace(/ /g, '+')}:wght@400;500;600;700`).join('&');
  const href = `https://fonts.googleapis.com/css2?${fam}&display=swap`;
  return `<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin><link rel="stylesheet" href="${esc(href)}">`;
}

const SHADOWS = { none: 'none', soft: '0 2px 10px rgba(16,24,40,.08)', strong: '0 8px 28px rgba(16,24,40,.20)' };

export function themeCss(s) {
  const primary = col(s.color_primary, '#1b2a41');
  const accent = col(s.color_accent, '#d4af37');
  const light = {
    bg: col(s.color_bg, '#f4f5f7'), surface: col(s.color_surface, '#ffffff'), text: col(s.color_text, '#1f2937'), muted: col(s.color_muted, '#6b7280'),
    border: col(s.color_border, '#e3e6ea'), hdrBg: col(s.color_header_bg, '#ffffff'), hdrText: col(s.color_header_text, '#1b2a41'),
    ftrBg: col(s.color_footer_bg, '#1b2a41'), ftrText: col(s.color_footer_text, '#dbe2ee'),
  };
  const lightVars = `--bg:${light.bg};--surface:${light.surface};--text:${light.text};--muted:${light.muted};--border:${light.border};--primary:${primary};--on-primary:${onColor(primary)};--accent:${accent};--on-accent:${onColor(accent)};--link:${primary};--hdr-bg:${light.hdrBg};--hdr-text:${light.hdrText};--ftr-bg:${light.ftrBg};--ftr-text:${light.ftrText};--chip:rgba(0,0,0,.05);color-scheme:light;`;
  const darkVars = `--bg:#0f141b;--surface:#18202b;--text:#e6e9ee;--muted:#9aa4b2;--border:#2a3442;--primary:${accent};--on-primary:${onColor(accent)};--accent:${accent};--on-accent:${onColor(accent)};--link:${accent};--hdr-bg:#121922;--hdr-text:#e6e9ee;--ftr-bg:#0a0e13;--ftr-text:#b8c2d0;--chip:rgba(255,255,255,.08);color-scheme:dark;`;
  const mode = ['light', 'dark', 'auto'].includes(s.theme_mode) ? s.theme_mode : 'light';
  const scale = Math.min(125, Math.max(85, toInt(s.font_scale, 100)));
  const radius = Math.min(28, Math.max(0, toInt(s.radius, 10)));
  const width = Math.min(1700, Math.max(960, toInt(s.container_width, 1280)));
  const shadow = SHADOWS[s.card_shadow] || SHADOWS.soft;
  const common = `--radius:${radius}px;--shadow:${shadow};--container:${width}px;--font-h:${fontStack(fontName(s.font_heading, 'Montserrat'))};--font-b:${fontStack(fontName(s.font_body, 'Inter'))};`;
  let css = `:root{${common}${mode === 'dark' ? darkVars : lightVars}}\nhtml{font-size:${scale}%}`;
  if (mode === 'auto') css += `\n@media (prefers-color-scheme:dark){:root{${darkVars}}}`;
  return css;
}

// Ad/yazi rengi: etiket rozeti icin okunur yazi rengi
export function badgeStyle(color) {
  const c = HEX.test(String(color || '')) ? color : '#d4af37';
  return `background:${c};color:${onColor(c)}`;
}
