// Cekirdek testler: sema, yardimcilar, temizleyici, ayarlar, seed, R2, kripto.
import { all, first, run, scalar } from './env.js';
import { t, TP, cleanup, call } from './testkit.js';
import { slugify, esc, toNum, fmtPrice, fmtInt, safeUrl, parseCookies, cookie, stripTags, truncate, html, raw, toInt, truthy, placeholders, chunk } from './util.js';
import { hashPassword, verifyPassword, hmacHex, safeEqual } from './crypto.js';
import { sanitizeHtml } from './sanitize.js';
import { SETTING_DEFS, defaultSettings, normalizeSetting, getSettings, setSetting, setSettings, sJson } from './settings.js';
import { ensureSeed } from './seed.js';

const M = 'Çekirdek';
const T = (name, run_) => ({ module: M, name, run: run_ });
const TABLES = ['settings', 'admins', 'sessions', 'listings', 'tags', 'menu_items', 'blog_categories', 'posts', 'pages', 'banners', 'messages', 'activity', 'test_runs'];

export const tests = [
  ...TABLES.map((tb) => T(`şema: ${tb} tablosu var`, async (env) => { const r = await all(env, `PRAGMA table_info(${tb})`); t.ok(r.length > 1, 'tablo yok'); })),
  T('şema: listings.slug benzersiz', async (env) => { const r = await all(env, 'PRAGMA index_list(listings)'); t.ok(r.some((x) => x.unique), 'benzersiz dizin yok'); }),
  T('veritabanı: yaz-oku-sil', async (env) => { await run(env, 'INSERT INTO settings(key,value) VALUES(?,?)', `${TP}k`, 'v'); t.eq((await first(env, 'SELECT value FROM settings WHERE key=?', `${TP}k`)).value, 'v'); await run(env, 'DELETE FROM settings WHERE key=?', `${TP}k`); t.eq(await first(env, 'SELECT value FROM settings WHERE key=?', `${TP}k`), null); }),
  T('veritabanı: bağlı parametre SQL enjeksiyonunu etkisizleştirir', async (env) => { const r = await all(env, 'SELECT * FROM tags WHERE name=?', "x' OR '1'='1"); t.eq(r.length, 0); }),
  T('util.slugify: Türkçe karakterler', async () => { t.eq(slugify('Satılık Deniz Manzaralı Villa'), 'satilik-deniz-manzarali-villa'); t.eq(slugify('ÇĞİÖŞÜ çğıöşü'), 'cgios-u-cgiosu'.replace('s-u', 'su')); }),
  T('util.slugify: sembol ve uzunluk', async () => { t.eq(slugify('A & B!!'), 'a-ve-b'); t.ok(slugify('a'.repeat(200)).length <= 80); t.eq(slugify('???'), ''); }),
  T('util.esc kaçışlar', async () => { t.eq(esc('<a href="x">&\'</a>'), '&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;'); }),
  T('util.html otomatik kaçış ve raw', async () => { t.eq(String(html`<b>${'<i>'}</b>`), '<b>&lt;i&gt;</b>'); t.eq(String(html`<b>${raw('<i>')}</b>`), '<b><i></b>'); t.eq(String(html`${[1, '<']}`), '1&lt;'); }),
  T('util.toNum Türkçe/İngilizce biçimler', async () => { t.eq(toNum('1.250.000'), 1250000); t.eq(toNum('1.250,50'), 1250.5); t.eq(toNum('12,5'), 12.5); t.eq(toNum('abc', 7), 7); t.eq(toNum(5), 5); }),
  T('util.fmtPrice TR/EN/RU', async () => { t.eq(fmtPrice(1550000, 'TRY', 'tr'), '1.550.000 ₺'); t.eq(fmtPrice(1550000, 'EUR', 'en'), '1,550,000 €'); t.ok(/^1\s550\s000 \$$/.test(fmtPrice(1550000, 'USD', 'ru'))); }),
  T('util.safeUrl izinli/yasak', async () => { t.eq(safeUrl('javascript:alert(1)'), ''); t.eq(safeUrl('https://a.com/x'), 'https://a.com/x'); t.eq(safeUrl('/yol'), '/yol'); t.ok(!/^\/\//.test(safeUrl('//evil.com'))); t.eq(safeUrl('data:text/html,x'), ''); t.eq(safeUrl('mailto:a@b.c'), 'mailto:a@b.c'); }),
  T('util.çerez ayrıştırma ve üretme', async () => { const c = parseCookies('a=1; b=x%20y; bad'); t.eq(c.a, '1'); t.eq(c.b, 'x y'); const s = cookie('s', 'v', { httpOnly: true, secure: true, maxAge: 5 }); t.has(s, 'HttpOnly'); t.has(s, 'Secure'); t.has(s, 'Max-Age=5'); t.has(s, 'SameSite=Lax'); }),
  T('util.parola özeti doğru/yanlış', async () => { const h = await hashPassword('Gizli-123'); t.ok(h.startsWith('pbkdf2$')); t.ok(await verifyPassword('Gizli-123', h)); t.ok(!(await verifyPassword('yanlis', h))); t.ok(!(await verifyPassword('x', 'bozuk'))); }),
  T('util.parola özeti her seferinde farklı (tuz)', async () => { t.ok((await hashPassword('aynı')) !== (await hashPassword('aynı'))); }),
  T('util.hmac ve sabit zamanlı karşılaştırma', async () => { t.eq(await hmacHex('k', 'm'), await hmacHex('k', 'm')); t.ok((await hmacHex('k', 'm')) !== (await hmacHex('k2', 'm'))); t.ok(await safeEqual('a', 'a')); t.ok(!(await safeEqual('a', 'b'))); t.ok(!(await safeEqual('a', 'ab'))); }),
  T('util küçük yardımcılar', async () => { t.eq(stripTags('<p>Merhaba <b>dünya</b></p>'), 'Merhaba dünya'); t.eq(truncate('kısa', 10), 'kısa'); t.ok(truncate('çok uzun bir metin burada', 10).endsWith('…')); t.eq(toInt('12abc'), 12); t.ok(truthy('on') && truthy(1) && !truthy('0')); t.eq(placeholders(3), '?,?,?'); t.eq(chunk([1, 2, 3], 2).length, 2); t.eq(fmtInt(1234567), '1.234.567'); }),
  T('sanitize: script ve içeriği atılır', async () => { const o = sanitizeHtml('<p>a</p><script>alert(1)</script><p>b</p>'); t.hasNot(o, 'script'); t.hasNot(o, 'alert'); t.has(o, '<p>b</p>'); }),
  T('sanitize: olay özellikleri atılır', async () => { t.hasNot(sanitizeHtml('<img src="/a.jpg" onerror="x()">'), 'onerror'); t.hasNot(sanitizeHtml('<p onclick="x()">a</p>'), 'onclick'); }),
  T('sanitize: javascript: bağlantısı atılır', async () => { t.hasNot(sanitizeHtml('<a href="javascript:alert(1)">x</a>'), 'javascript'); t.has(sanitizeHtml('<a href="https://a.com">x</a>'), 'https://a.com'); }),
  T('sanitize: iframe/form/style atılır', async () => { const o = sanitizeHtml('<iframe src="https://x.com"></iframe><form><input></form><style>p{}</style>ok'); t.hasNot(o, 'iframe'); t.hasNot(o, 'form'); t.hasNot(o, 'style'); t.has(o, 'ok'); }),
  T('sanitize: tablo ve liste korunur', async () => { const o = sanitizeHtml('<table><tr><td colspan="2">x</td></tr></table><ul><li>a</li></ul>'); t.has(o, '<td colspan="2">'); t.has(o, '<li>a</li>'); }),
  T('sanitize: img tembel yükleme ve alt eklenir', async () => { const o = sanitizeHtml('<img src="/media/a.jpg">'); t.has(o, 'loading="lazy"'); t.has(o, 'alt=""'); }),
  T('sanitize: kapanmamış etiketler kapatılır, h1→h2', async () => { t.eq(sanitizeHtml('<p><b>x'), '<p><b>x</b></p>'); t.has(sanitizeHtml('<h1>t</h1>'), '<h2>t</h2>'); }),
  T('sanitize: target=_blank noopener alır', async () => { t.has(sanitizeHtml('<a href="https://a.com" target="_blank">x</a>'), 'noopener'); }),
  T('sanitize: ham < işareti kaçışlanır', async () => { t.has(sanitizeHtml('5 < 6'), '&lt;'); }),
  T('ayarlar: tüm tanımların varsayılanı geçerli', async () => { for (const d of SETTING_DEFS) t.eq(normalizeSetting(d.key, d.def), d.type === 'number' ? String(Math.min(d.max ?? Infinity, Math.max(d.min ?? -Infinity, parseFloat(d.def)))) : d.def, d.key); }),
  T('ayarlar: anahtarlar benzersiz', async () => { t.eq(new Set(SETTING_DEFS.map((d) => d.key)).size, SETTING_DEFS.length); }),
  T('ayarlar: sayı sınırları', async () => { t.eq(normalizeSetting('home_per_page', '9999'), '60'); t.eq(normalizeSetting('home_per_page', '-5'), '2'); t.eq(normalizeSetting('home_per_page', 'abc'), '12'); }),
  T('ayarlar: seçenek ve renk doğrulama', async () => { t.eq(normalizeSetting('home_columns', '7'), '3'); t.eq(normalizeSetting('home_columns', '4'), '4'); t.eq(normalizeSetting('color_accent', 'kırmızı'), '#d4af37'); t.eq(normalizeSetting('color_accent', '#AABBCC'), '#AABBCC'); }),
  T('ayarlar: bool ve json', async () => { t.eq(normalizeSetting('sticky_header', 'on'), '1'); t.eq(normalizeSetting('sticky_header', ''), '0'); t.eq(normalizeSetting('sidebar_widgets', '{bozuk'), defaultSettings().sidebar_widgets); }),
  T('ayarlar: bilinmeyen anahtar reddedilir', async (env) => { t.eq(await setSetting(env, 'yok_boyle', '1'), false); }),
  T('ayarlar: kaydet ve oku (önbellek sıfırlanır)', async (env) => { await setSetting(env, 'site_name', `${TP}Ad`); t.eq((await getSettings(env)).site_name, `${TP}Ad`); await setSettings(env, { site_name: 'Sahibinden Fethiye', radius: 99 }); const s = await getSettings(env); t.eq(s.radius, '28'); t.eq(sJson(s, 'sidebar_widgets', [])[0], 'search'); }),
  T('seed: menü öğeleri ve slug\'lar', async (env) => { await ensureSeed(env); const m = await all(env, "SELECT slug,kind,target FROM menu_items WHERE menu='main'"); for (const s of ['satilik-villa', 'satilik-daire', 'satilik-arazi', 'kiralik-villa', 'kiralik-daire', 'diger-ilanlar', 'blog', 'hakkimizda', 'iletisim']) t.ok(m.some((x) => x.slug === s), s); t.ok(m.some((x) => x.target === '*|*')); }),
  T('seed: etiketler', async (env) => { await ensureSeed(env); const n = (await all(env, 'SELECT name FROM tags')).map((x) => x.name); for (const s of ['Müstakil', 'Villa', 'Fırsat', 'Yeni', 'Deniz Manzaralı']) t.ok(n.includes(s), s); }),
  T('seed: blog kategorileri ve haber türü', async (env) => { await ensureSeed(env); const c = await all(env, 'SELECT name,kind FROM blog_categories'); t.ok(c.some((x) => x.name === 'ARAZİ ÇEŞİTLERİ VE İMAR DURUMLARI')); t.ok(c.some((x) => x.kind === 'news')); }),
  T('seed: içe aktarılan yazılar yayında ve temiz', async (env) => { await ensureSeed(env); const p = await all(env, "SELECT title,body,status FROM posts WHERE source='import'"); t.ok(p.length >= 5, `yazı sayısı ${p.length}`); for (const x of p) { t.eq(x.status, 'yayinda'); t.hasNot(x.body, '<script'); } }),
  T('seed: ikinci çalıştırma yinelenme yapmaz', async (env) => { await ensureSeed(env); const a = Number(await scalar(env, 'SELECT COUNT(*) FROM menu_items')); await ensureSeed(env); t.eq(Number(await scalar(env, 'SELECT COUNT(*) FROM menu_items')), a); }),
  T('seed: Hakkımızda ve İletişim sayfaları', async (env) => { await ensureSeed(env); for (const s of ['hakkimizda', 'iletisim']) t.ok(await first(env, 'SELECT id FROM pages WHERE slug=?', s), s); }),
  T('R2: yaz-oku-listele-sil', async (env) => { await env.MEDIA.put('m/zz-test-a.txt', 'merhaba', { httpMetadata: { contentType: 'text/plain' } }); const o = await env.MEDIA.get('m/zz-test-a.txt'); t.eq(await o.text(), 'merhaba'); const l = await env.MEDIA.list({ prefix: 'm/zz-test-' }); t.ok(l.objects.length >= 1); await env.MEDIA.delete('m/zz-test-a.txt'); t.eq(await env.MEDIA.get('m/zz-test-a.txt'), null); }),
  T('testkit.cleanup test verisini siler', async (env) => { await run(env, "INSERT INTO tags(name) VALUES('__test__x')"); await env.MEDIA.put('m/zz-test-b', 'x'); await cleanup(env); t.eq(await first(env, "SELECT id FROM tags WHERE name='__test__x'"), null); t.eq(await env.MEDIA.get('m/__test__b'), null); }),
  T('yönlendirici: bilinmeyen yol 404 (kilitlenmez)', async (env) => { const r = await call(env, '/bu-yol-yok-xyz'); t.status(r, 404); }),
  T('yönlendirici: statik logo sunulur', async (env) => { const r = await call(env, '/logo.png'); t.status(r, 200); t.has(r.headers.get('content-type') || '', 'image/png'); }),
];
