// Yonetim paneli: baska bir siteden (WordPress REST API ya da tek tek adres) TAM METIN yazi ice aktarma.
import { html, raw, esc, toInt, nowIso, stripTags, truncate, safeUrl } from './util.js';
import { sanitizeHtml } from './sanitize.js';
import { all, first, run, logActivity } from './env.js';
import { shell, fld, postForm, okRedirect, errRedirect } from './admin_ui.js';
import { uniqueSlug, RESERVED_SLUGS } from './admin_util.js';

const MAX_PER_CLICK = 8;
const UA = { 'user-agent': 'Mozilla/5.0 (compatible; SahibindenFethiyeImport/1.0)', accept: 'application/json,text/html;q=0.9,*/*;q=0.5' };

export function decodeEntities(s) {
  const map = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', hellip: '…', ndash: '–', mdash: '—', rsquo: '’', lsquo: '‘', ldquo: '“', rdquo: '”' };
  return String(s || '').replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (m, e) => {
    if (e[0] === '#') { const n = e[1].toLowerCase() === 'x' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10); try { return String.fromCodePoint(n); } catch { return m; } }
    return map[e.toLowerCase()] ?? m;
  });
}
export function cleanSite(u) {
  let s = String(u || '').trim().replace(/\/+$/, '');
  if (s && !/^https?:\/\//i.test(s)) s = 'https://' + s;
  try { const x = new URL(s); if (x.protocol !== 'https:' && x.protocol !== 'http:') return ''; if (!x.hostname.includes('.')) return ''; return x.origin; } catch { return ''; }
}
export function wpToPost(it) {
  const media = it._embedded && it._embedded['wp:featuredmedia'] && it._embedded['wp:featuredmedia'][0];
  return {
    wpId: it.id, slug: it.slug || '', link: it.link || '',
    title: decodeEntities(stripTags((it.title && it.title.rendered) || '')).trim(),
    excerpt: truncate(decodeEntities(stripTags((it.excerpt && it.excerpt.rendered) || '')).replace(/\s+/g, ' ').trim(), 480),
    body: (it.content && it.content.rendered) || '', image: (media && media.source_url) || '', date: it.date_gmt ? it.date_gmt + 'Z' : (it.date || ''),
  };
}
// Duz HTML sayfasindan yazi cikarir (WordPress API yoksa).
export function extractArticle(page, url = '') {
  const meta = (p) => { const m = page.match(new RegExp(`<meta[^>]+(?:property|name)=["']${p}["'][^>]*content=["']([^"']*)["']`, 'i')) || page.match(new RegExp(`<meta[^>]+content=["']([^"']*)["'][^>]*(?:property|name)=["']${p}["']`, 'i')); return m ? decodeEntities(m[1]) : ''; };
  const h1 = (page.match(/<h1[^>]*>([\s\S]*?)<\/h1>/i) || [])[1] || '';
  const title = decodeEntities(stripTags(h1)).trim() || meta('og:title') || decodeEntities(stripTags((page.match(/<title[^>]*>([\s\S]*?)<\/title>/i) || [])[1] || '')).trim();
  let body = '';
  const cands = [/<div[^>]+class=["'][^"']*(?:entry-content|post-content|article-content|post-body|td-post-content)[^"']*["'][^>]*>/i, /<article[^>]*>/i, /<main[^>]*>/i];
  for (const re of cands) {
    const m = re.exec(page); if (!m) continue;
    const start = m.index + m[0].length; const tag = m[0].startsWith('<div') ? 'div' : m[0].startsWith('<article') ? 'article' : 'main';
    let depth = 1; const t = new RegExp(`<(/?)${tag}\\b[^>]*>`, 'gi'); t.lastIndex = start; let e; let end = page.length;
    while ((e = t.exec(page))) { depth += e[1] ? -1 : 1; if (depth === 0) { end = e.index; break; } }
    body = page.slice(start, end); if (stripTags(body).length > 200) break;
  }
  body = body.replace(/<h1[^>]*>[\s\S]*?<\/h1>/i, '');
  return { slug: '', link: url, title, excerpt: truncate(meta('og:description') || meta('description'), 480), body, image: meta('og:image'), date: meta('article:published_time') };
}
async function getText(url) {
  const r = await fetch(url, { headers: UA, redirect: 'follow' });
  if (!r.ok) throw new Error(`${url} → HTTP ${r.status}`);
  return await r.text();
}
async function wpList(site, ids) {
  const q = `_embed=wp:featuredmedia&per_page=100&orderby=date&order=desc&_fields=id,slug,link,title,excerpt,content,date_gmt,date,_embedded` + (ids && ids.length ? `&include=${ids.join(',')}` : '');
  const t = await getText(`${site}/wp-json/wp/v2/posts?${q}`);
  let j; try { j = JSON.parse(t); } catch { throw new Error('WordPress API yanıtı okunamadı (site WordPress değil ya da API kapalı).'); }
  if (!Array.isArray(j)) throw new Error('WordPress API beklenen biçimde yanıt vermedi.');
  return j.map(wpToPost);
}
export async function savePost(env, p, catId, who) {
  const body = sanitizeHtml(p.body);
  if (stripTags(body).length < 40) throw new Error(`“${p.title}” için metin bulunamadı.`);
  const old = (p.slug && await first(env, 'SELECT id,slug FROM posts WHERE slug=?', p.slug)) || await first(env, 'SELECT id,slug FROM posts WHERE title=?', p.title);
  const pub = p.date && !isNaN(new Date(p.date)) ? new Date(p.date).toISOString() : nowIso();
  const img = p.image && safeUrl(p.image) ? p.image.slice(0, 300) : '';
  if (old) {
    await run(env, `UPDATE posts SET body=?,excerpt=CASE WHEN ?<>'' THEN ? ELSE excerpt END,image=CASE WHEN ?<>'' THEN ? ELSE image END,category_id=?,published_at=?,updated_at=?,source='import' WHERE id=?`, body, p.excerpt, p.excerpt, img, img, catId, pub, nowIso(), old.id);
    return 'güncellendi';
  }
  let slug = await uniqueSlug(env, 'posts', p.slug || p.title, 0, 'yazi'); if (RESERVED_SLUGS.has(slug)) slug += '-yazi';
  await run(env, `INSERT INTO posts(slug,category_id,title,excerpt,body,image,status,author,published_at,updated_at,source) VALUES(?,?,?,?,?,?,'yayinda',?,?,?,'import')`, slug, catId, p.title.slice(0, 200), p.excerpt, body, img, who || '', pub, nowIso());
  return 'eklendi';
}

export async function importer(ctx, base, cats, kind) {
  const env = ctx.env; const back = `${base}/ice-aktar`;
  const catOpts = cats.map((c) => [c.id, c.name]);
  const catIds = new Set(cats.map((c) => c.id));
  if (ctx.method === 'POST') {
    const site = cleanSite(ctx.s('site')); const mode = ctx.s('mode');
    const defCat = toInt(ctx.s('cat_all'), 0);
    const pickCat = (key) => { const c = toInt(ctx.s(key), 0) || defCat; return catIds.has(c) ? c : (cats[0] ? cats[0].id : 0); };
    const report = [];
    try {
      if (mode === 'urls') {
        const urls = ctx.s('urls').split(/\s+/).map((x) => x.trim()).filter(Boolean).slice(0, MAX_PER_CLICK);
        if (!urls.length) return errRedirect(ctx, back, 'Adres yazılmadı.');
        for (const u of urls) {
          const c = cleanSite(u); if (!c) { report.push(`${u}: geçersiz adres`); continue; }
          try { const p = extractArticle(await getText(u), u); if (!p.title) throw new Error('başlık bulunamadı'); report.push(`${p.title}: ${await savePost(env, p, pickCat('cat_all'), ctx.admin.display_name)}`); } catch (e) { report.push(`${u}: ${e.message}`); }
        }
      } else {
        if (!site) return errRedirect(ctx, back, 'Site adresi geçersiz.');
        const ids = ctx.s('ids').split(',').map((x) => toInt(x, 0)).filter((x) => x && ctx.s('sel_' + x)).slice(0, MAX_PER_CLICK);
        if (!ids.length) return errRedirect(ctx, `${back}?site=${encodeURIComponent(site)}`, 'Yazı seçilmedi (her seferde en çok ' + MAX_PER_CLICK + ' yazı).');
        const list = await wpList(site, ids);
        for (const p of list) { try { report.push(`${p.title}: ${await savePost(env, p, pickCat('cat_' + p.wpId), ctx.admin.display_name)}`); } catch (e) { report.push(`${p.title}: ${e.message}`); } }
      }
    } catch (e) { return errRedirect(ctx, back, 'İçe aktarma hatası: ' + e.message); }
    await logActivity(env, ctx.who, `Yazı içe aktarıldı (${report.length})`);
    return okRedirect(ctx, `${back}${site ? '?site=' + encodeURIComponent(site) : ''}`, report.join(' • ').slice(0, 900));
  }
  const site = cleanSite(ctx.url.searchParams.get('site') || '');
  let listing = null; let err = '';
  if (site) { try { listing = await wpList(site); } catch (e) { err = e.message; } }
  const body = html`<p><a class="btn ghost" href="${base}">← Liste</a></p>
<div class="card form"><h2>1) WordPress sitesinden toplu listele</h2><p class="muted">Kaynak site WordPress ise tüm yazılar <b>tam metinleriyle</b> gelir. Listeden seçip kendi kategorinize atarsınız (her tıklamada en çok ${MAX_PER_CLICK} yazı). Zaten var olan (aynı başlık/adres) yazıların metni tam metinle <b>güncellenir</b>.</p>
<form method="get" class="row"><input name="site" value="${site || 'https://www.ibrahimcalli.com.tr'}" style="min-width:18rem"><button class="btn primary" type="submit">Yazıları listele</button></form>${err ? html`<p class="flash err">${err} — aşağıdaki “adreslerle aktar” yöntemini deneyin.</p>` : ''}</div>
${listing ? postForm(ctx, `${base}/ice-aktar`, html`<input type="hidden" name="site" value="${site}"><input type="hidden" name="ids" value="${listing.map((p) => p.wpId).join(',')}">
<div class="card"><div class="row"><label>Varsayılan kategori <select name="cat_all">${catOpts.map(([v, n]) => html`<option value="${v}">${n}</option>`)}</select></label> <button class="btn primary" type="submit">Seçilenleri içe aktar</button></div>
<div class="tablewrap"><table class="tbl"><thead><tr><th></th><th>Başlık</th><th>Tarih</th><th>Metin</th><th>Kategori</th></tr></thead><tbody>${listing.map((p) => html`<tr><td><input type="checkbox" name="sel_${p.wpId}" value="1"></td><td><a href="${p.link}" target="_blank" rel="noopener">${p.title}</a></td><td class="muted nowrap">${String(p.date).slice(0, 10)}</td><td class="muted">${stripTags(p.body).length} krk</td><td><select name="cat_${p.wpId}"><option value="0">(varsayılan)</option>${catOpts.map(([v, n]) => html`<option value="${v}">${n}</option>`)}</select></td></tr>`)}</tbody></table></div></div>`) : ''}
<div class="card form"><h2>2) Adreslerle aktar</h2><p class="muted">Yazı adreslerini her satıra bir tane yapıştırın (en çok ${MAX_PER_CLICK}). Sayfadaki ana metin otomatik ayıklanır.</p>
${postForm(ctx, `${base}/ice-aktar`, html`<input type="hidden" name="mode" value="urls">${fld({ name: 'urls', label: 'Yazı adresleri', type: 'textarea', rows: 5 })}${fld({ name: 'cat_all', label: 'Kategori', type: 'select', options: catOpts, value: catOpts[0] ? catOpts[0][0] : 0 })}<button class="btn primary" type="submit">Adreslerden aktar</button>`)}</div>`;
  return shell(ctx, { title: 'Yazı içe aktar', active: base, body });
}
