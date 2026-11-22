// Ice aktarma yardimcilari testleri (agsiz).
import { decodeEntities, cleanSite, wpToPost, extractArticle, savePost } from './admin_import.js';
import { first } from './env.js';
import { TP, call } from './testkit.js';
import { adminSession } from './admin_kit.js';
import { ensureSeed } from './seed.js';
const eq = (a, b, m) => { if (a !== b) throw new Error(`${m || 'eşit değil'}: ${JSON.stringify(a)} ≠ ${JSON.stringify(b)}`); };
const long = '<p>' + 'Tam metin cümlesi ışık ıslak İstanbul. '.repeat(30) + '</p>';
export const tests = [
  { module: 'import', name: 'HTML varlıkları çözülür', run: async () => { eq(decodeEntities('Kira &#8217;da &amp; &ndash; &#x131;'), 'Kira ’da & – ı'); } },
  { module: 'import', name: 'Site adresi doğrulanır', run: async () => { eq(cleanSite('www.ibrahimcalli.com.tr/'), 'https://www.ibrahimcalli.com.tr'); eq(cleanSite('javascript:alert(1)'), ''); eq(cleanSite('localhost'), ''); } },
  { module: 'import', name: 'WordPress kaydı yazıya çevrilir', run: async () => { const p = wpToPost({ id: 5, slug: 'a-b', link: 'https://x.com/a-b', title: { rendered: 'Ba&#8217;slık' }, excerpt: { rendered: '<p>Özet</p>' }, content: { rendered: long }, date_gmt: '2026-01-02T03:04:05', _embedded: { 'wp:featuredmedia': [{ source_url: 'https://x.com/i.jpg' }] } }); eq(p.title, 'Ba’slık'); eq(p.excerpt, 'Özet'); eq(p.image, 'https://x.com/i.jpg'); eq(p.date, '2026-01-02T03:04:05Z'); } },
  { module: 'import', name: 'Düz sayfadan ana metin ayıklanır', run: async () => { const p = extractArticle(`<html><head><meta property="og:image" content="https://x.com/o.jpg"><meta name="description" content="Kısa"></head><body><nav>menü</nav><h1>Başlık Burada</h1><div class="entry-content"><div>${long}</div><p>son</p></div><footer>alt</footer></body></html>`); eq(p.title, 'Başlık Burada'); eq(p.image, 'https://x.com/o.jpg'); if (!p.body.includes('Tam metin') || !p.body.includes('son') || p.body.includes('alt<')) throw new Error('metin yanlış ayıklandı'); } },
  { module: 'import', name: 'İçe aktarılan yazı tam metinle kaydedilir ve tekrarında güncellenir', run: async (env) => {
    const title = TP + 'içe-aktar-yazı'; const slug = 'zz-test-ice-aktar';
    try {
      eq(await savePost(env, { slug, title, excerpt: 'ö', body: long, image: '', date: '2026-01-01T00:00:00Z' }, 0, 't'), 'eklendi');
      const a = await first(env, 'SELECT body,source FROM posts WHERE slug=?', slug); if (a.body.length < 800) throw new Error('metin kısa'); eq(a.source, 'import');
      eq(await savePost(env, { slug, title, excerpt: 'ö', body: long + '<p>ek</p>', image: '', date: '' }, 0, 't'), 'güncellendi');
      const b = await first(env, 'SELECT body FROM posts WHERE slug=?', slug); if (!b.body.includes('ek')) throw new Error('güncellenmedi');
      let hata = false; try { await savePost(env, { slug: slug + '2', title: title + '2', body: '<p>kısa</p>' }, 0, 't'); } catch { hata = true; } if (!hata) throw new Error('kısa metin reddedilmeliydi');
    } finally { await env.DB.prepare('DELETE FROM posts WHERE slug LIKE ?').bind('zz-test-ice-aktar%').run(); }
  } },
  { module: 'import', name: 'İçe aktarma sayfası açılır ve listede bağlantı var', run: async (env) => {
    await ensureSeed(env); const a = await adminSession(env);
    const r = await a.get('/admin/yazilar/ice-aktar'); if (r.status !== 200) throw new Error('HTTP ' + r.status);
    const h = await r.text(); if (!h.includes('Yazıları listele') || !h.includes('Adreslerden aktar')) throw new Error('form yok');
    const l = await (await a.get('/admin/yazilar')).text(); if (!l.includes('/admin/yazilar/ice-aktar')) throw new Error('liste bağlantısı yok');
  } },
  { module: 'blog', name: 'Blog: kart (kare resim) varsayılan, liste seçeneği var', run: async (env) => {
    await ensureSeed(env);
    const h = await (await call(env, '/blog')).text();
    if (!h.includes('view-thumb') || !h.includes('vt-list') || !h.includes('/assets/blog/')) throw new Error('kart görünümü/geçiş/resim yok');
    const l = await (await call(env, '/blog?view=list')).text(); if (!l.includes('view-list')) throw new Error('liste görünümü yok');
    const c = await (await call(env, '/blog/arazi-cesitleri-ve-imar-durumlari')).text(); if (!c.includes('view-thumb') || !c.includes('ticari-arsa-nedir.jpg')) throw new Error('kategori sayfası thumb değil');
  } },
  { module: 'blog', name: 'Yazı sayfasında geniş banner görseli var', run: async (env) => {
    await ensureSeed(env); const h = await (await call(env, '/yazi/ticari-arsa-nedir')).text();
    if (!h.includes('post-img') || !h.includes('/assets/blog/arazi-cesitleri-ve-imar-durumlari/ticari-arsa-nedir.jpg')) throw new Error('banner yok');
  } },
  { module: 'blog', name: 'Ana sayfada çok bannerlı kaydırıcı çalışır (otomatik geçiş ayarı)', run: async (env) => {
    await ensureSeed(env);
    try {
      for (let i = 1; i <= 3; i++) await env.DB.prepare("INSERT INTO banners(title,subtitle,image,link,button,sort_key,visible) VALUES(?,?,?,?,?,?,1)").bind(TP + 'b' + i, '', '/assets/banner/fethiye-evler.jpg', '', '', 900 + i).run();
      const h = await (await call(env, '/')).text();
      if ((h.match(/class="slide( active)?"/g) || []).length < 3 || !h.includes('data-slider') || !/data-autoplay="[01]"/.test(h)) throw new Error('kaydırıcı eksik');
    } finally { await env.DB.prepare('DELETE FROM banners WHERE title LIKE ?').bind(TP + 'b%').run(); }
  } },
];
