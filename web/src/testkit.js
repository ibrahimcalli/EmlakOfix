// Canli sistem testleri icin ortak araclar (Worker icinde ve `node --test` ile ayni gövde calisir).
// Kural: test verisi TEST_ID_MIN ustu ilan numaralari ve "__test__" onekiyle yazilir; runner sonunda cleanup() calisir.
import { all, run } from './env.js';
export const TP = '__test__';
export const TEST_ID_MIN = 900000000;

export class TestFail extends Error {}
export const t = {
  ok(cond, msg = 'koşul sağlanmadı') { if (!cond) throw new TestFail(msg); },
  eq(a, b, msg = '') { if (a !== b && JSON.stringify(a) !== JSON.stringify(b)) throw new TestFail(`${msg} beklenen=${JSON.stringify(b)} gelen=${JSON.stringify(a)}`.trim()); },
  has(text, part, msg = '') { if (!String(text).includes(part)) throw new TestFail(`${msg} "${part}" bulunamadı`.trim()); },
  hasNot(text, part, msg = '') { if (String(text).includes(part)) throw new TestFail(`${msg} "${part}" bulunmamalıydı`.trim()); },
  status(res, code, msg = '') { if (res.status !== code) throw new TestFail(`${msg} HTTP beklenen=${code} gelen=${res.status}`.trim()); },
};

// Worker'a istek gonderir (yönlendirmeyi takip ETMEZ).
export async function call(env, path, init = {}) {
  const { route } = await import('./index.js');
  const base = env.SITE_URL || 'https://test.example';
  const req = new Request(new URL(path, base), { redirect: 'manual', ...init });
  return route(req, env, { waitUntil() {}, passThroughOnException() {} });
}
export const siteAuth = (env, extra = {}) => ({ authorization: `Bearer ${env.SITE_TOKEN}`, ...extra });
export const jsonInit = (method, obj, headers = {}) => ({ method, headers: { 'content-type': 'application/json', ...headers }, body: JSON.stringify(obj) });
export const textOf = async (res) => await res.text();
export const jsonOf = async (res) => await res.json();

let seq = 0;
export const testId = () => TEST_ID_MIN + 1000 + (seq++ % 5000);
export function listingPayload(over = {}) {
  const id = over.id || testId();
  return { id, title: `${TP} Satılık Deniz Manzaralı Villa ${id}`, title_en: `${TP} Villa for sale ${id}`, title_ru: `${TP} Вилла ${id}`, listing_type: 'Satılık', category: 'Villa', district: 'Fethiye', neighborhood: 'Çalış', address: 'Test sokak', lat: 36.65, lon: 29.12, price: 12500000, currency: 'TRY', gross_m2: 220, net_m2: 180, rooms: '4+1', building_age: '2', floor: 'Müstakil', total_floors: '2', heating: 'Klima', deed_status: 'Kat mülkiyeti', furnished: 1, swap_ok: 0, credit_ok: 1, desc_tr: `${TP} Türkçe açıklama <script>alert(1)</script>`, desc_en: `${TP} English description`, desc_ru: `${TP} Русское описание`, tags: ['Villa', 'Fırsat'], photo_count: 0, agent_name: 'Test Danışman', agent_phone: '+905550000000', prog_status: 'Yayında', published_at: new Date().toISOString(), ...over };
}
export async function putListing(env, over = {}) {
  const p = listingPayload(over);
  const res = await call(env, `/api/sync/listing/${p.id}`, jsonInit('PUT', p, siteAuth(env)));
  return { p, res };
}
export const JPEG_1PX = Uint8Array.from(atob('/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAMCAgICAgMCAgIDAwMDBAYEBAQEBAgGBgUGCQgKCgkICQkKDA8MCgsOCwkJDRENDg8QEBEQCgwSExIQEw8QEBD/yQALCAABAAEBAREA/8wABgAQEAX/2gAIAQEAAD8A0s8g/9k='), (c) => c.charCodeAt(0));

// Test verisini temizler (ilanlar, yazilar, kategoriler, sayfalar, etiketler, menu, banner, mesaj, yonetici, R2 anahtarlari).
export async function cleanup(env) {
  const like = `${TP}%`;
  await run(env, 'DELETE FROM listings WHERE id>=? OR title LIKE ? OR slug LIKE ?', TEST_ID_MIN, like, like);
  await run(env, 'DELETE FROM posts WHERE slug LIKE ? OR title LIKE ?', like, like);
  await run(env, 'DELETE FROM blog_categories WHERE slug LIKE ? OR name LIKE ?', like, like);
  await run(env, 'DELETE FROM pages WHERE slug LIKE ? OR title LIKE ?', like, like);
  await run(env, 'DELETE FROM tags WHERE name LIKE ?', like);
  await run(env, 'DELETE FROM menu_items WHERE label LIKE ?', like);
  await run(env, 'DELETE FROM banners WHERE title LIKE ?', like);
  await run(env, 'DELETE FROM messages WHERE name LIKE ?', like);
  await run(env, 'DELETE FROM sessions WHERE admin_id IN (SELECT id FROM admins WHERE username LIKE ?)', like);
  await run(env, 'DELETE FROM admins WHERE username LIKE ?', like);
  await run(env, 'DELETE FROM activity WHERE who LIKE ? OR what LIKE ?', like, like);
  for (const prefix of ['l/9000', 'm/__test__', 'm/zz-test-']) {
    let cursor;
    do {
      const r = await env.MEDIA.list({ prefix, cursor, limit: 500 });
      if (r.objects.length) await env.MEDIA.delete(r.objects.map((o) => o.key));
      cursor = r.truncated ? r.cursor : undefined;
    } while (cursor);
  }
}
export async function snapshotSettings(env) { return await all(env, 'SELECT key,value FROM settings'); }
export async function restoreSettings(env, snap) {
  await run(env, 'DELETE FROM settings');
  for (const r of snap) await run(env, 'INSERT INTO settings(key,value) VALUES(?,?)', r.key, r.value);
}
