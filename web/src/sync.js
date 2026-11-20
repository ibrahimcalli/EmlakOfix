// Program -> Site eşitleme API'si (/api/sync/*). Yetki: Authorization: Bearer <SITE_TOKEN> (sabit zamanlı karşılaştırma).
import { all, first, run, scalar, batch, logActivity } from './env.js';
import { jsonResponse, slugify, nowIso, parseJson, toNum, toInt, truthy, stripTags, truncate } from './util.js';
import { safeEqual } from './crypto.js';
import { sanitizeHtml } from './sanitize.js';

export const SYNC_VERSION = '1.0';
export const MAX_JSON = 1024 * 1024;          // ilan/yazı/etiket JSON gövdesi: 1 MB
export const MAX_POST_JSON = 2 * 1024 * 1024; // yazı gövdesi: 2 MB
export const MAX_IMAGE = 8 * 1024 * 1024;     // fotoğraf / medya: 8 MB
export const MAX_PHOTOS = 200;
const CACHE_IMG = 'public, max-age=2592000';

const bad = (msg, status = 400) => jsonResponse({ ok: false, error: msg }, status);

// ---------------------------------------------------------------- yardımcılar
async function readBody(request, max) {
  const cl = Number(request.headers.get('content-length') || 0);
  if (cl > max) return { tooBig: true };
  if (!request.body) return { buf: new Uint8Array(0) };
  const reader = request.body.getReader();
  const parts = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > max) { try { await reader.cancel(); } catch { /* yoksay */ } return { tooBig: true }; }
    parts.push(value);
  }
  const buf = new Uint8Array(total);
  let o = 0;
  for (const p of parts) { buf.set(p, o); o += p.byteLength; }
  return { buf };
}
async function readJson(request, max = MAX_JSON) {
  const b = await readBody(request, max);
  if (b.tooBig) return { res: bad('Gövde çok büyük.', 413) };
  let v;
  try { v = JSON.parse(new TextDecoder().decode(b.buf)); } catch { return { res: bad('Geçersiz JSON.') }; }
  if (!v || typeof v !== 'object' || Array.isArray(v)) return { res: bad('JSON nesnesi bekleniyordu.') };
  return { body: v };
}
// Sihirli bayt ile görsel türü: jpg | png | webp | gif | null
export function sniffImage(b) {
  if (!b || b.length < 12) return null;
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'jpg';
  if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47 && b[4] === 0x0d && b[5] === 0x0a && b[6] === 0x1a && b[7] === 0x0a) return 'png';
  if (b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46 && b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50) return 'webp';
  if (b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x38 && (b[4] === 0x37 || b[4] === 0x39) && b[5] === 0x61) return 'gif';
  return null;
}
export const MIME_BY_EXT = { jpg: 'image/jpeg', png: 'image/png', webp: 'image/webp', gif: 'image/gif' };

const CTRL = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g;
function str(v, max) {
  if (v === null || v === undefined || typeof v === 'object') return '';
  return String(v).replace(CTRL, '').trim().slice(0, max);
}
// Medya dosya adı: küçük harf, Türkçe karakter çevrilir, yalnız a-z 0-9 _ - . ; uzantı türe göre.
export function cleanMediaName(name, type) {
  let n = String(name ?? '').split(/[\\/]/).pop() || '';
  const m = /\.([A-Za-z0-9]{2,5})$/.exec(n);
  let ext = m ? m[1].toLowerCase() : '';
  if (ext === 'jpeg') ext = 'jpg';
  if (m) n = n.slice(0, -m[0].length);
  let base = n.split('_').map((p) => slugify(p, 80)).join('_').replace(/^\.+/, '').slice(0, 80);
  if (!base.replace(/_/g, '')) base = 'gorsel-' + Date.now().toString(36);
  if (type) ext = type; // içerik türü uzantıyı belirler
  if (!MIME_BY_EXT[ext]) ext = 'jpg';
  return `${base}.${ext}`;
}
async function deletePrefix(env, prefix) {
  let cursor;
  let n = 0;
  do {
    const r = await env.MEDIA.list({ prefix, cursor, limit: 500 });
    if (r.objects.length) { await env.MEDIA.delete(r.objects.map((o) => o.key)); n += r.objects.length; }
    cursor = r.truncated ? r.cursor : undefined;
  } while (cursor);
  return n;
}
const intArr = (v) => (Array.isArray(v) ? v.map((x) => toInt(x, -1)).filter((x) => x >= 0 && x < MAX_PHOTOS) : []);
function normTags(v) {
  let arr = Array.isArray(v) ? v : typeof v === 'string' ? v.split(/[;,]/) : [];
  const seen = new Set();
  const out = [];
  for (const x of arr) {
    const s = str(x, 40);
    if (!s || s.includes(';') || seen.has(s.toLowerCase())) continue;
    seen.add(s.toLowerCase());
    out.push(s);
    if (out.length >= 30) break;
  }
  return out;
}
const colorOk = (c) => (/^#[0-9a-fA-F]{6}$/.test(String(c || '').trim()) ? String(c).trim() : '#d4af37');

// ---------------------------------------------------------------- ilan
const L_TEXT = { title: 200, title_en: 200, title_ru: 200, listing_type: 40, category: 60, district: 80, neighborhood: 100, address: 300, rooms: 30, bathrooms: 30, building_age: 30, floor: 40, total_floors: 30, heating: 60, deed_status: 80, desc_tr: 30000, desc_en: 30000, desc_ru: 30000, agent_name: 100, agent_phone: 40, price_note: 200 };
const L_NUM = ['lat', 'lon', 'price', 'gross_m2', 'net_m2'];
const L_BOOL = ['furnished', 'swap_ok', 'credit_ok', 'hide_exact_location'];

async function uniqueListingSlug(env, base, id) {
  let slug = base;
  for (let i = 0; i < 6; i++) {
    const row = await first(env, 'SELECT id FROM listings WHERE slug=? AND id<>?', slug, id);
    if (!row) return slug;
    slug = i === 0 ? `${base}-${id}` : `${base}-${id}-${Math.random().toString(36).slice(2, 6)}`;
  }
  return `${base}-${id}-${Date.now().toString(36)}`;
}

async function putListing(env, request, id) {
  const { body, res } = await readJson(request);
  if (res) return res;
  const title = str(body.title, L_TEXT.title);
  if (!title) return bad('Başlık boş olamaz.');
  const ex = await first(env, 'SELECT id,slug,overrides,photo_order,photo_count FROM listings WHERE id=?', id);
  const ovRaw = ex ? parseJson(ex.overrides, {}) : {};
  const ov = ovRaw && typeof ovRaw === 'object' && !Array.isArray(ovRaw) ? ovRaw : {};
  const locked = (k) => Object.prototype.hasOwnProperty.call(ov, k);
  const cols = {}; // sütun -> değer (yalnız gövdede gelenler)
  cols.title = title;
  for (const [k, max] of Object.entries(L_TEXT)) if (k !== 'title' && k in body) cols[k] = str(body[k], max).replace(/^$/, '');
  // metin alanlarında satır sonlarını koru
  for (const k of ['desc_tr', 'desc_en', 'desc_ru']) if (k in body) cols[k] = (typeof body[k] === 'string' ? body[k].replace(CTRL, '').trim() : '').slice(0, L_TEXT[k]);
  for (const k of L_NUM) if (k in body) {
    let n = toNum(body[k], 0);
    if (k === 'lat' && Math.abs(n) > 90) n = 0;
    if (k === 'lon' && Math.abs(n) > 180) n = 0;
    if (k !== 'lat' && k !== 'lon' && (n < 0 || n > 1e13)) n = 0;
    cols[k] = n;
  }
  for (const k of L_BOOL) if (k in body) cols[k] = truthy(body[k]) ? 1 : 0;
  if ('currency' in body) { let c = str(body.currency, 5).toUpperCase(); if (c === 'TL') c = 'TRY'; cols.currency = /^[A-Z]{3}$/.test(c) ? c : 'TRY'; }
  if ('prog_status' in body) cols.prog_status = str(body.prog_status, 30) || 'Yayında';
  if ('published_at' in body) { const p = str(body.published_at, 40); cols.published_at = p && !isNaN(new Date(p)) ? p : ''; }
  if ('tags' in body) cols.tags = JSON.stringify(normTags(body.tags));
  if ('photo_count' in body) cols.photo_count = Math.max(0, Math.min(MAX_PHOTOS, toInt(body.photo_count, 0)));
  if ('photo_count' in body || 'photo_order' in body) {
    const pc = cols.photo_count ?? (ex ? ex.photo_count : 0);
    let order = intArr(body.photo_order);
    if (!order.length) order = Array.from({ length: pc }, (_, i) => i);
    cols.photo_order = JSON.stringify([...new Set(order)]);
  }
  // slug
  let slug = null;
  if (!ex) {
    const given = slugify(str(body.slug, 100), 80);
    slug = given || (slugify(title, 70) ? `${slugify(title, 70)}-${id}` : `ilan-${id}`);
  } else if ('slug' in body && !locked('slug')) {
    const given = slugify(str(body.slug, 100), 80);
    if (given && given !== ex.slug) slug = given;
  }
  if (slug !== null) cols.slug = await uniqueListingSlug(env, slug, id);
  // admin panelinde elle değiştirilen alanlara dokunma
  if (ex) for (const k of Object.keys(cols)) if (locked(k)) delete cols[k];
  if (ex && cols.published_at === '') delete cols.published_at; // boş tarih mevcut yayın tarihini silmesin
  cols.updated_at = nowIso();
  if (!ex && !cols.published_at) cols.published_at = cols.updated_at;
  const names = Object.keys(cols);
  const vals = names.map((k) => cols[k]);
  const doWrite = async () => {
    if (ex) await run(env, `UPDATE listings SET ${names.map((k) => `${k}=?`).join(',')} WHERE id=?`, ...vals, id);
    else await run(env, `INSERT INTO listings(id,${names.join(',')}) VALUES(?,${names.map(() => '?').join(',')})`, id, ...vals);
  };
  try { await doWrite(); } catch (e) {
    if (!/UNIQUE/i.test(String(e && e.message)) || !cols.slug) throw e;
    cols.slug = `${cols.slug}-${Math.random().toString(36).slice(2, 7)}`;
    vals[names.indexOf('slug')] = cols.slug;
    await doWrite();
  }
  if ('tags' in cols) {
    const stmts = JSON.parse(cols.tags).map((n) => ["INSERT OR IGNORE INTO tags(name,color) VALUES(?,'#d4af37')", n]);
    if (stmts.length) await batch(env, stmts);
  }
  const row = await first(env, 'SELECT slug,photo_count FROM listings WHERE id=?', id);
  return jsonResponse({ ok: true, id, slug: row.slug, created: !ex, photo_count: row.photo_count });
}

async function deleteListing(env, id) {
  const ex = await first(env, 'SELECT id FROM listings WHERE id=?', id);
  const n = await deletePrefix(env, `l/${id}/`);
  if (ex) await run(env, 'DELETE FROM listings WHERE id=?', id);
  if (ex) await logActivity(env, 'program', `İlan #${id} siteden silindi`);
  return jsonResponse({ ok: true, deleted: !!ex, photos_deleted: n });
}

// ---------------------------------------------------------------- fotoğraf
async function putPhoto(env, request, id, n) {
  const b = await readBody(request, MAX_IMAGE);
  if (b.tooBig) return bad('Fotoğraf 8 MB sınırını aşıyor.', 413);
  if (!b.buf.length) return bad('Fotoğraf gövdesi boş.');
  const type = sniffImage(b.buf);
  if (!type || type === 'gif') return bad('Yalnız JPEG, PNG veya WebP fotoğraf kabul edilir.', 415);
  const key = `l/${id}/${n}.jpg`;
  await env.MEDIA.put(key, b.buf, { httpMetadata: { contentType: MIME_BY_EXT[type], cacheControl: CACHE_IMG } });
  const ex = await first(env, 'SELECT photo_order,overrides FROM listings WHERE id=?', id);
  if (ex) {
    const ov = parseJson(ex.overrides, {}) || {};
    const order = intArr(parseJson(ex.photo_order, []));
    if (!order.includes(n) && !Object.prototype.hasOwnProperty.call(ov, 'photo_order')) {
      order.push(n);
      const sets = ['photo_order=?'];
      const args = [JSON.stringify(order)];
      if (!Object.prototype.hasOwnProperty.call(ov, 'photo_count')) { sets.push('photo_count=?'); args.push(order.length); }
      await run(env, `UPDATE listings SET ${sets.join(',')} WHERE id=?`, ...args, id);
    }
  }
  return jsonResponse({ ok: true, key, size: b.buf.length, type: MIME_BY_EXT[type] });
}
async function deletePhoto(env, id, n) {
  await env.MEDIA.delete(`l/${id}/${n}.jpg`);
  const ex = await first(env, 'SELECT photo_order,photo_count,overrides FROM listings WHERE id=?', id);
  if (ex) {
    const ov = parseJson(ex.overrides, {}) || {};
    const order = intArr(parseJson(ex.photo_order, [])).filter((x) => x !== n);
    const sets = ['photo_order=?'];
    const args = [JSON.stringify(order)];
    if (!Object.prototype.hasOwnProperty.call(ov, 'photo_count')) { sets.push('photo_count=?'); args.push(order.length); }
    await run(env, `UPDATE listings SET ${sets.join(',')} WHERE id=?`, ...args, id);
  }
  return jsonResponse({ ok: true });
}

// ---------------------------------------------------------------- etiketler
async function putTags(env, request) {
  const { body, res } = await readJson(request);
  if (res) return res;
  if (!Array.isArray(body.tags)) return bad('"tags" dizisi bekleniyordu.');
  const seen = new Set();
  const stmts = [];
  for (const t of body.tags.slice(0, 300)) {
    if (!t || typeof t !== 'object') continue;
    const name = str(t.name, 40);
    if (!name || name.includes(';') || seen.has(name)) continue;
    seen.add(name);
    stmts.push(['INSERT INTO tags(name,name_en,name_ru,color) VALUES(?,?,?,?) ON CONFLICT(name) DO UPDATE SET name_en=excluded.name_en, name_ru=excluded.name_ru, color=excluded.color', name, str(t.name_en, 40), str(t.name_ru, 40), colorOk(t.color)]);
  }
  if (stmts.length) await batch(env, stmts);
  return jsonResponse({ ok: true, count: stmts.length });
}

// ---------------------------------------------------------------- yazı
async function categoryId(env, nameIn) {
  const name = str(nameIn, 80) || 'Genel';
  const slug = slugify(name, 80) || 'genel';
  let row = await first(env, 'SELECT id FROM blog_categories WHERE name=? OR slug=?', name, slug);
  if (row) return row.id;
  const so = Number(await scalar(env, 'SELECT COALESCE(MAX(sort_key),0)+1 FROM blog_categories')) || 1;
  await run(env, "INSERT OR IGNORE INTO blog_categories(slug,name,kind,sort_key) VALUES(?,?, 'blog', ?)", slug, name, so);
  row = await first(env, 'SELECT id FROM blog_categories WHERE slug=?', slug);
  return row ? row.id : 0;
}
// image_name: yükleme sırasında uzantı içerik türüne göre değişmiş olabilir; R2'de gerçekte var olan adı bul.
async function resolveMediaName(env, raw) {
  const clean = cleanMediaName(raw);
  try {
    if (await env.MEDIA.head(`m/${clean}`)) return clean;
    const base = clean.replace(/\.[a-z0-9]+$/, '');
    for (const ext of ['jpg', 'png', 'webp', 'gif']) if (await env.MEDIA.head(`m/${base}.${ext}`)) return `${base}.${ext}`;
  } catch { /* R2 okunamazsa temiz adı kullan */ }
  return clean;
}
async function putPost(env, request, origin) {
  const { body, res } = await readJson(request, MAX_POST_JSON);
  if (res) return res;
  const title = str(body.title, 200);
  if (!title) return bad('Yazı başlığı boş olamaz.');
  const slugIn = slugify(str(body.slug, 120), 100);
  const slug = slugIn || slugify(title, 100) || `yazi-${Date.now().toString(36)}`;
  const ex = await first(env, 'SELECT id FROM posts WHERE slug=?', slug);
  const status = str(body.status, 12).toLowerCase() === 'taslak' ? 'taslak' : 'yayinda';
  const lv = body.lang_variants && typeof body.lang_variants === 'object' ? body.lang_variants : {};
  const variant = (lang) => (lv[lang] && typeof lv[lang] === 'object' ? lv[lang] : {});
  const cleanBody = (s) => sanitizeHtml(typeof s === 'string' ? s.slice(0, 600000) : '');
  const bodyHtml = cleanBody(body.body);
  let excerpt = str(body.excerpt, 600);
  if (!excerpt) excerpt = truncate(stripTags(bodyHtml), 200);
  const cat = await categoryId(env, body.category);
  const cols = { category_id: cat, title, excerpt, body: bodyHtml, status, author: str(body.author, 100), source: 'program', updated_at: nowIso() };
  const en = variant('en'), ru = variant('ru');
  if ('title_en' in body || en.title !== undefined) cols.title_en = str(en.title ?? body.title_en, 200);
  if ('title_ru' in body || ru.title !== undefined) cols.title_ru = str(ru.title ?? body.title_ru, 200);
  if ('body_en' in body || en.body !== undefined) cols.body_en = cleanBody(en.body ?? body.body_en);
  if ('body_ru' in body || ru.body !== undefined) cols.body_ru = cleanBody(ru.body ?? body.body_ru);
  const imgName = str(body.image_name, 120);
  if (imgName) cols.image = '/media/' + (await resolveMediaName(env, imgName));
  const names = Object.keys(cols);
  const vals = names.map((k) => cols[k]);
  let id;
  if (ex) {
    await run(env, `UPDATE posts SET ${names.map((k) => `${k}=?`).join(',')} WHERE id=?`, ...vals, ex.id);
    id = ex.id;
  } else {
    const names2 = [...names, 'slug', 'published_at'];
    await run(env, `INSERT INTO posts(${names2.join(',')}) VALUES(${names2.map(() => '?').join(',')})`, ...vals, slug, nowIso());
    id = (await first(env, 'SELECT id FROM posts WHERE slug=?', slug)).id;
  }
  await logActivity(env, 'program', `Yazı gönderildi: ${slug}`);
  return jsonResponse({ ok: true, id, slug, url: `${origin}/yazi/${slug}`, created: !ex, status });
}

// ---------------------------------------------------------------- medya
async function putMedia(env, request, rawName) {
  const b = await readBody(request, MAX_IMAGE);
  if (b.tooBig) return bad('Görsel 8 MB sınırını aşıyor.', 413);
  if (!b.buf.length) return bad('Görsel gövdesi boş.');
  const type = sniffImage(b.buf);
  if (!type) return bad('Yalnız JPEG, PNG, WebP veya GIF görsel kabul edilir.', 415);
  const name = cleanMediaName(rawName, type);
  await env.MEDIA.put(`m/${name}`, b.buf, { httpMetadata: { contentType: MIME_BY_EXT[type], cacheControl: CACHE_IMG } });
  return jsonResponse({ ok: true, url: `/media/${name}`, name, size: b.buf.length });
}

// ---------------------------------------------------------------- dışa aktarma
async function exportAll(env) {
  const t = (name) => all(env, `SELECT * FROM ${name}`);
  return jsonResponse({
    ok: true, version: SYNC_VERSION, exported_at: nowIso(),
    settings: await t('settings'), listings: await t('listings'), posts: await t('posts'), blog_categories: await t('blog_categories'),
    tags: await t('tags'), menu_items: await t('menu_items'), pages: await t('pages'), banners: await t('banners'),
  });
}

// ---------------------------------------------------------------- yönlendirme
async function authorized(request, env) {
  const tok = String(env.SITE_TOKEN || '');
  if (!tok) return false;
  const m = /^Bearer\s+(.+)$/i.exec(request.headers.get('authorization') || '');
  if (!m) return false;
  return await safeEqual(m[1].trim(), tok);
}
const idOf = (s) => (/^\d{1,15}$/.test(s || '') && Number(s) > 0 ? Number(s) : 0);

export async function handleSync(request, env, url) {
  if (!(await authorized(request, env))) return jsonResponse({ ok: false, error: 'Yetkisiz: geçersiz anahtar.' }, 401, { 'www-authenticate': 'Bearer' });
  const rest = url.pathname.slice('/api/sync/'.length).replace(/\/+$/, '');
  const seg = rest.split('/');
  const m = request.method.toUpperCase();
  const origin = String(env.SITE_URL || url.origin).replace(/\/+$/, '');
  const notAllowed = () => bad('Bu uçta bu yöntem kullanılamaz.', 405);

  switch (seg[0]) {
    case 'ping':
      if (seg.length !== 1) break;
      if (m !== 'GET' && m !== 'HEAD') return notAllowed();
      return jsonResponse({ ok: true, version: SYNC_VERSION, listings: Number(await scalar(env, 'SELECT COUNT(*) FROM listings')) || 0, posts: Number(await scalar(env, 'SELECT COUNT(*) FROM posts')) || 0 });
    case 'listing': {
      if (seg.length !== 2) break;
      const id = idOf(seg[1]);
      if (!id) return bad('Geçersiz ilan numarası.');
      if (m === 'PUT') return putListing(env, request, id);
      if (m === 'DELETE') return deleteListing(env, id);
      return notAllowed();
    }
    case 'photo': {
      if (seg.length !== 3) break;
      const id = idOf(seg[1]);
      const n = /^\d{1,3}$/.test(seg[2]) ? Number(seg[2]) : -1;
      if (!id || n < 0 || n >= MAX_PHOTOS) return bad('Geçersiz ilan numarası veya fotoğraf sırası.');
      if (m === 'PUT') return putPhoto(env, request, id, n);
      if (m === 'DELETE') return deletePhoto(env, id, n);
      return notAllowed();
    }
    case 'tags':
      if (seg.length !== 1) break;
      return m === 'PUT' ? putTags(env, request) : notAllowed();
    case 'post':
      if (seg.length !== 1) break;
      return m === 'PUT' ? putPost(env, request, origin) : notAllowed();
    case 'media': {
      if (seg.length < 2) break;
      if (m !== 'PUT') return notAllowed();
      let name = seg.slice(1).join('/');
      try { name = decodeURIComponent(name); } catch { return bad('Geçersiz dosya adı.'); }
      return putMedia(env, request, name);
    }
    case 'export':
      if (seg.length !== 1) break;
      return m === 'GET' ? exportAll(env) : notAllowed();
  }
  return bad('Böyle bir uç yok.', 404);
}
