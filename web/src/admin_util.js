// Panel icin util.js yeniden disa aktarimi + kucuk ek yardimcilar.
export * from './util.js';
import { randomHex } from './crypto.js';
export const randomHexSafe = (bytes = 8) => randomHex(bytes);

export const WIDGET_NAMES = { search: 'Ara kutusu', pages: 'Sayfalar menüsü', categories: 'Kategoriler', news: 'Haberler', blogposts: 'Son yazılar', tags: 'Etiketler', contact: 'İletişim' };

// Telefonu WhatsApp (wa.me) biciminde rakamlara cevirir (TR varsayilan).
export function waNumber(phone) {
  let d = String(phone || '').replace(/\D+/g, '');
  if (!d) return '';
  if (d.startsWith('00')) d = d.slice(2);
  else if (d.startsWith('0')) d = '90' + d.slice(1);
  else if (d.length === 10 && d.startsWith('5')) d = '90' + d;
  return d;
}
export const isoToLocalInput = (iso) => String(iso || '').slice(0, 16);
// Benzersiz slug uretir (tablo, kolon). exceptId ayni kaydi saymaz.
export async function uniqueSlug(env, table, base, exceptId = 0, fallback = 'kayit') {
  const { first } = await import('./env.js');
  const { slugify } = await import('./util.js');
  const root = slugify(base) || fallback;
  let slug = root;
  for (let i = 2; i < 200; i++) {
    const r = await first(env, `SELECT id FROM ${table} WHERE slug=? AND id<>?`, slug, exceptId);
    if (!r) return slug;
    slug = `${root}-${i}`;
  }
  return `${root}-${Date.now()}`;
}
export const RESERVED_SLUGS = new Set(['admin', 'api', 'media', 'img', 'blog', 'yazi', 'ilan', 'en', 'ru', 'tr', 'sitemap.xml', 'robots.txt', 'logo.png', 'favicon.png', 'favicon.ico', 'static', 'assets', 'admin.css', 'admin.js', 'iletisim-formu', 'ara']);
