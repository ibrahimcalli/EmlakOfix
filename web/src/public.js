// Genel site: ana sayfa, ilan listeleri/detay, blog, sayfalar, iletisim formu, sitemap, robots, bakim modu.
import { all, first, run, scalar } from './env.js';
import {
  esc, html, raw, safeUrl, fmtInt, fmtDate, parseJson, toInt, toNum, truthy, stripTags, truncate, clamp,
  htmlResponse, textResponse, redirect, cookie, parseCookies, nowIso,
} from './util.js';
import { getSettings, sBool, sInt, sJson } from './settings.js';
import { sanitizeHtml } from './sanitize.js';
import { t, LANGS, normalizeLang } from './i18n.js';
import {
  shell, href, qs, langPrefix, absUrl, pickLang, mediaUrl, photoList, photoUrl, listingSlugPath, typeLabel, catLabel,
  buildMenuTree, defaultMenuRows, menuSlug, cardHtml, postCardHtml, paginationHtml, securityHeaders, jsonLdScript,
  priceText, statusRibbon, tagBadges, locationText, postImage,
} from './layout.js';

const SORTS = ['featured', 'newest', 'price_asc', 'price_desc'];
const SORT_SQL = {
  featured: 'l.featured DESC, (l.sort_key=0) ASC, l.sort_key ASC, l.published_at DESC, l.id DESC',
  newest: 'l.published_at DESC, l.id DESC',
  price_asc: '(l.price=0) ASC, l.price ASC, l.id DESC',
  price_desc: 'l.price DESC, l.id DESC',
};
const LEAFLET = 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/';

// ---------- yardimcilar
const likeEsc = (s) => String(s).replace(/[\\%_]/g, '\\$&');
const decodeSeg = (s) => { try { return decodeURIComponent(s); } catch { return s; } };
const waDigits = (p) => {
  let d = String(p || '').replace(/\D+/g, '');
  if (!d) return '';
  if (d.startsWith('00')) d = d.slice(2); else if (d.startsWith('0')) d = `90${d.slice(1)}`; else if (d.length === 10 && d.startsWith('5')) d = `90${d}`;
  return d;
};
const telHref = (p) => `tel:${String(p || '').replace(/[^\d+]/g, '')}`;
const visWhere = (s, a = 'l') => (sBool(s, 'show_sold_listings')
  ? `${a}.visible=1 AND ${a}.prog_status IN ('Yayında','Kiralandı','Satıldı')`
  : `${a}.visible=1 AND ${a}.prog_status='Yayında'`);
const clientIp = (request) => (request.headers.get('cf-connecting-ip') || String(request.headers.get('x-forwarded-for') || '').split(',')[0].trim() || 'unknown').slice(0, 64);

function langList(s) {
  const def = normalizeLang(s.default_lang);
  const en = String(s.enabled_langs || '').split(',').map((x) => x.trim());
  return { def, langs: LANGS.filter((l) => en.includes(l) || l === def) };
}

async function buildCtx(request, env, url) {
  const s = await getSettings(env);
  const { def, langs } = langList(s);
  let path = url.pathname.replace(/\/{2,}/g, '/');
  let lang = def;
  let redirectTo = null;
  const m = path.match(/^\/(tr|en|ru)(\/.*)?$/);
  if (m && langs.includes(m[1])) {
    lang = m[1];
    path = m[2] || '/';
    if (lang === def) redirectTo = path + url.search;
  }
  if (path.length > 1) path = path.replace(/\/+$/, '') || '/';
  const rows = (await all(env, "SELECT * FROM menu_items WHERE menu='main' AND visible=1 ORDER BY sort_key, id")).filter(() => true);
  const ids = new Set(rows.map((r) => r.id));
  const usable = rows.filter((r) => !r.parent_id || ids.has(r.parent_id));
  const pc = { lang, defLang: def, langs, s, https: url.protocol === 'https:', origin: String(env.SITE_URL || url.origin).replace(/\/+$/, ''), search: url.search };
  pc.prefix = langPrefix(pc, lang);
  pc.neutral = path;
  pc.here = href(pc, path);
  pc.menuRows = usable;
  pc.menu = buildMenuTree(pc, usable.length ? usable : defaultMenuRows());
  return { pc, path, redirectTo };
}

function respond(pc, status, o, extra = {}) {
  return htmlResponse(shell(pc, o), status, { ...securityHeaders(pc.s, pc.https), 'cache-control': 'public, max-age=0, must-revalidate', vary: 'Cookie', ...extra });
}
function notFound(pc, sidebar = '') {
  const body = html`<section class="notfound"><h1>${t(pc.lang, 'notfound_title')}</h1><p>${t(pc.lang, 'notfound_text')}</p><p><a class="btn" href="${href(pc, '/')}">${t(pc.lang, 'back_home')}</a> <a class="btn ghost" href="${href(pc, '/#ilanlar')}">${t(pc.lang, 'browse_listings')}</a></p></section>`;
  return respond(pc, 404, { title: t(pc.lang, 'notfound_title'), body, sidebar, noindex: true, bodyClass: 'p-404' });
}
function crumbs(pc, list) {
  return html`<nav class="crumbs" aria-label="${t(pc.lang, 'breadcrumb')}"><ol>${list.map(([label, h], i) => (i === list.length - 1 || !h ? html`<li aria-current="page">${label}</li>` : html`<li><a href="${h}">${label}</a></li>`))}</ol></nav>`;
}
const opt = (value, label, cur) => html`<option value="${value}"${String(cur) === String(value) ? raw(' selected') : ''}>${label}</option>`;

// ---------- kenar cubugu
async function tagMapOf(env) {
  const rows = await all(env, 'SELECT * FROM tags ORDER BY sort_key, name');
  return { rows, map: new Map(rows.map((r) => [r.name, r])) };
}
async function buildSidebar(env, pc, kind) {
  const s = pc.s;
  if (s.sidebar_position === 'hidden') return '';
  if (kind === 'listings' && !sBool(s, 'sidebar_on_listings')) return '';
  if (kind === 'blog' && !sBool(s, 'sidebar_on_blog')) return '';
  const order = sJson(s, 'sidebar_widgets', []);
  const lang = pc.lang;
  const box = (key, title, inner) => html`<section class="widget w-${key}"><h2 class="w-title">${title}</h2>${inner}</section>`;
  const out = [];
  for (const w of Array.isArray(order) ? order : []) {
    if (w === 'search') {
      out.push(box(w, t(lang, 'w_search'), html`<form class="w-search" role="search" method="get" action="${href(pc, '/ara')}"><label class="sr-only" for="ws-q">${t(lang, 'search')}</label><input id="ws-q" type="search" name="q" placeholder="${t(lang, 'search_ph')}" maxlength="100"><button type="submit">${t(lang, 'search')}</button></form>`));
    } else if (w === 'pages') {
      const rows = await all(env, 'SELECT slug,title,title_en,title_ru FROM pages WHERE visible=1 ORDER BY id');
      if (rows.length) out.push(box(w, t(lang, 'w_pages'), html`<ul class="w-list">${rows.map((r) => html`<li><a href="${href(pc, `/${encodeURIComponent(r.slug)}`)}">${pickLang(r, 'title', lang)}</a></li>`)}</ul>`));
    } else if (w === 'categories') {
      const rows = await all(env, "SELECT c.*, (SELECT COUNT(*) FROM posts p WHERE p.category_id=c.id AND p.status='yayinda') n FROM blog_categories c WHERE c.kind='blog' ORDER BY c.sort_key, c.id");
      if (rows.length) out.push(box(w, t(lang, 'w_categories'), html`<ul class="w-list">${rows.map((r) => html`<li><a href="${href(pc, `/blog/${encodeURIComponent(r.slug)}`)}">${pickLang(r, 'name', lang)}</a> <span class="cnt">${r.n}</span></li>`)}</ul>`));
    } else if (w === 'news') {
      const cats = await all(env, "SELECT c.*, (SELECT COUNT(*) FROM posts p WHERE p.category_id=c.id AND p.status='yayinda') n FROM blog_categories c WHERE c.kind='news' ORDER BY c.sort_key, c.id");
      if (cats.length) {
        out.push(box(w, t(lang, 'w_news'), html`<ul class="w-list">${cats.map((r) => html`<li><a href="${href(pc, `/blog/${encodeURIComponent(r.slug)}`)}">${pickLang(r, 'name', lang)}</a> <span class="cnt">${r.n}</span></li>`)}</ul>`));
        const posts = await all(env, "SELECT p.* FROM posts p JOIN blog_categories c ON c.id=p.category_id WHERE c.kind='news' AND p.status='yayinda' ORDER BY p.published_at DESC, p.id DESC LIMIT 5");
        if (posts.length) out.push(box('news2', t(lang, 'w_news2'), html`<ul class="w-list">${posts.map((p) => html`<li><a href="${href(pc, `/yazi/${encodeURIComponent(p.slug)}`)}">${pickLang(p, 'title', lang)}</a></li>`)}</ul>`));
      }
    } else if (w === 'blogposts') {
      const posts = await all(env, "SELECT * FROM posts WHERE status='yayinda' ORDER BY published_at DESC, id DESC LIMIT 5");
      if (posts.length) out.push(box(w, t(lang, 'w_blogposts'), html`<ul class="w-list">${posts.map((p) => html`<li><a href="${href(pc, `/yazi/${encodeURIComponent(p.slug)}`)}">${pickLang(p, 'title', lang)}</a></li>`)}</ul>`));
    } else if (w === 'tags') {
      const { rows } = await tagMapOf(env);
      if (rows.length) out.push(box(w, t(lang, 'w_tags'), html`<ul class="w-tags">${rows.slice(0, 40).map((r) => html`<li><a href="${href(pc, '/') + qs({ tag: r.name })}">${pickLang(r, 'name', lang)}</a></li>`)}</ul>`));
    } else if (w === 'contact') {
      const wa = waDigits(s.whatsapp);
      out.push(box(w, t(lang, 'w_contact'), html`<ul class="w-list w-contact">${s.phone ? html`<li><a href="${telHref(s.phone)}">${s.phone}</a></li>` : ''}${wa ? html`<li><a href="https://wa.me/${wa}" rel="noopener" target="_blank">${t(lang, 'whatsapp')}</a></li>` : ''}${s.email ? html`<li><a href="mailto:${s.email}">${s.email}</a></li>` : ''}${s.address ? html`<li>${s.address}</li>` : ''}</ul>`));
    }
  }
  return out.length ? html`${out}` : '';
}

// ---------- ilan sorgulari
function parseFilters(url, s) {
  const g = (k, n = 80) => String(url.searchParams.get(k) || '').trim().slice(0, n);
  const sortRaw = g('sort', 20);
  const dflt = SORTS.includes(s.listing_sort) ? s.listing_sort : 'featured';
  const min = toNum(g('min', 20), 0);
  const max = toNum(g('max', 20), 0);
  return {
    q: g('q', 100), type: g('type'), cat: g('cat'), district: g('district'), tag: g('tag'),
    min: min > 0 ? min : 0, max: max > 0 ? max : 0, sort: SORTS.includes(sortRaw) ? sortRaw : dflt, sortParam: SORTS.includes(sortRaw) ? sortRaw : '',
    page: Math.max(1, toInt(url.searchParams.get('page'), 1)),
  };
}
const filterParams = (f, skip = []) => {
  const o = { q: f.q, type: f.type, cat: f.cat, district: f.district, tag: f.tag, min: f.min || '', max: f.max || '', sort: f.sortParam };
  for (const k of skip) delete o[k];
  return o;
};
const hasFilter = (f) => !!(f.q || f.type || f.cat || f.district || f.tag || f.min || f.max);

function splitTarget(target) {
  const [a = '', b = ''] = String(target || '').split('|').map((x) => x.trim());
  return [a, b];
}
async function queryListings(env, pc, f, base = {}) {
  const s = pc.s;
  const w = [visWhere(s)];
  const a = [];
  const type = base.type || f.type;
  const cat = base.cat || f.cat;
  if (type) { w.push('l.listing_type=?'); a.push(type); }
  if (cat) { w.push('l.category=?'); a.push(cat); }
  if (f.district) { w.push('l.district=?'); a.push(f.district); }
  if (f.min) { w.push('l.price>=?'); a.push(f.min); }
  if (f.max) { w.push('l.price<=?'); a.push(f.max); }
  if (f.tag) { w.push("l.tags LIKE ? ESCAPE '\\'"); a.push(`%${likeEsc(JSON.stringify(f.tag))}%`); }
  if (f.q) {
    const like = `%${likeEsc(f.q)}%`;
    const cols = ['l.title', 'l.title_en', 'l.title_ru', 'l.district', 'l.neighborhood', 'l.category', 'l.address', 'l.desc_tr', 'l.slug'];
    w.push(`(${cols.map((c) => `${c} LIKE ? ESCAPE '\\'`).join(' OR ')} OR CAST(l.id AS TEXT)=?)`);
    a.push(...cols.map(() => like), f.q);
  }
  if (base.others) {
    for (const it of pc.menuRows.filter((r) => r.kind === 'listings')) {
      const [ta, tb] = splitTarget(it.target);
      if (ta === '*' && tb === '*') continue;
      const parts = [];
      if (ta && ta !== '*') { parts.push('l.listing_type=?'); a.push(ta); }
      if (tb && tb !== '*') { parts.push('l.category=?'); a.push(tb); }
      if (parts.length) w.push(`NOT (${parts.join(' AND ')})`);
    }
  }
  const where = w.join(' AND ');
  const per = clamp(sInt(s, 'home_per_page', 12), 2, 60);
  const total = Number(await scalar(env, `SELECT COUNT(*) FROM listings l WHERE ${where}`, ...a)) || 0;
  const pages = Math.max(1, Math.ceil(total / per));
  const page = clamp(f.page, 1, pages);
  const rows = await all(env, `SELECT l.* FROM listings l WHERE ${where} ORDER BY ${SORT_SQL[f.sort] || SORT_SQL.featured} LIMIT ? OFFSET ?`, ...a, per, (page - 1) * per);
  return { rows, total, pages, page, per };
}

async function filterChoices(env, pc) {
  const v = visWhere(pc.s);
  const col = async (c) => (await all(env, `SELECT DISTINCT l.${c} v FROM listings l WHERE ${v} AND l.${c}<>'' ORDER BY l.${c}`)).map((r) => r.v);
  return { types: await col('listing_type'), cats: await col('category'), districts: await col('district') };
}

function resolveView(request, url, s) {
  let view = s.home_default_view === 'list' ? 'list' : 'thumb';
  let set = false;
  if (!sBool(s, 'home_show_view_toggle')) return { view, set };
  const qv = url.searchParams.get('view');
  const ck = parseCookies(request.headers.get('cookie')).lv;
  if (qv === 'list' || qv === 'thumb') { view = qv; set = true; } else if (ck === 'list' || ck === 'thumb') view = ck;
  return { view, set };
}

function filterBar(pc, f, ch, basePath, fixed = {}) {
  const lang = pc.lang;
  return html`<form class="filters" id="ff" method="get" action="${href(pc, basePath)}#ilanlar" role="search">
  <div class="f-grid">
    <label class="f-q"><span class="sr-only">${t(lang, 'search')}</span><input type="search" name="q" value="${f.q}" placeholder="${t(lang, 'search_ph')}" maxlength="100"></label>
    ${fixed.type ? '' : html`<label><span class="sr-only">${t(lang, 'type')}</span><select name="type"><option value="">${t(lang, 'all_types')}</option>${ch.types.map((x) => opt(x, typeLabel(lang, x), f.type))}</select></label>`}
    ${fixed.cat ? '' : html`<label><span class="sr-only">${t(lang, 'category')}</span><select name="cat"><option value="">${t(lang, 'all_categories')}</option>${ch.cats.map((x) => opt(x, catLabel(lang, x), f.cat))}</select></label>`}
    <label><span class="sr-only">${t(lang, 'district')}</span><select name="district"><option value="">${t(lang, 'all_districts')}</option>${ch.districts.map((x) => opt(x, x, f.district))}</select></label>
    <label><span class="sr-only">${t(lang, 'min_price')}</span><input type="number" inputmode="numeric" min="0" name="min" value="${f.min || ''}" placeholder="${t(lang, 'min_price')}"></label>
    <label><span class="sr-only">${t(lang, 'max_price')}</span><input type="number" inputmode="numeric" min="0" name="max" value="${f.max || ''}" placeholder="${t(lang, 'max_price')}"></label>
    <label><span class="sr-only">${t(lang, 'tag')}</span><select name="tag"><option value="">${t(lang, 'all_tags')}</option>${ch.tags.map((x) => opt(x.name, pickLang(x, 'name', lang), f.tag))}</select></label>
    <div class="f-actions"><button type="submit" class="btn">${t(lang, 'filter')}</button>${hasFilter(f) ? html`<a class="btn ghost" href="${href(pc, basePath)}#ilanlar">${t(lang, 'clear')}</a>` : ''}</div>
  </div>
</form>`;
}

function listingsBlock(pc, request, url, res, f, view, tagMap, basePath, ch, opts = {}) {
  const lang = pc.lang;
  const s = pc.s;
  const cols = [2, 3, 4].includes(sInt(s, 'home_columns', 3)) ? sInt(s, 'home_columns', 3) : 3;
  const params = filterParams(f);
  const toggle = sBool(s, 'home_show_view_toggle')
    ? html`<div class="view-toggle" role="group" aria-label="${t(lang, 'view_thumb')} / ${t(lang, 'view_list')}"><a href="${href(pc, basePath) + qs({ ...params, view: 'thumb' })}#ilanlar" class="vt vt-thumb${view === 'thumb' ? ' on' : ''}" aria-label="${t(lang, 'view_thumb')}" title="${t(lang, 'view_thumb')}"${view === 'thumb' ? raw(' aria-current="true"') : ''}>▦</a><a href="${href(pc, basePath) + qs({ ...params, view: 'list' })}#ilanlar" class="vt vt-list${view === 'list' ? ' on' : ''}" aria-label="${t(lang, 'view_list')}" title="${t(lang, 'view_list')}"${view === 'list' ? raw(' aria-current="true"') : ''}>☰</a></div>`
    : '';
  const grid = res.rows.length
    ? html`<div class="grid cols-${cols} view-${view}">${res.rows.map((l, i) => cardHtml(pc, l, tagMap, { view, eager: i < 2 }))}</div>`
    : html`<p class="empty">${t(lang, 'no_results')}</p>`;
  return html`<section class="listings" id="ilanlar" aria-label="${t(lang, 'listings')}">
  ${opts.heading ? html`<h2 class="sec-title">${opts.heading}</h2>` : ''}
  ${opts.filter === false ? '' : filterBar(pc, f, ch, basePath, opts.fixed || {})}
  <div class="toolbar"><p class="count">${t(lang, 'results', fmtInt(res.total, lang))}</p>
    <label class="sort"><span>${t(lang, 'sort')}</span><select name="sort" form="ff" data-autosubmit>${SORTS.map((x) => opt(x, t(lang, `sort_${x}`), f.sort))}</select></label>${toggle}</div>
  ${grid}
  ${paginationHtml(pc, basePath, { ...filterParams(f, ['sort']), sort: f.sortParam, ...(opts.keep || {}) }, res.page, res.pages)}
</section>`;
}

async function listingPageCommon(env, pc, f) {
  const ch = await filterChoices(env, pc);
  const tg = await tagMapOf(env);
  ch.tags = tg.rows;
  return { ch, tagMap: tg.map };
}

// ---------- ana sayfa
function heroHtml(pc, banners) {
  const s = pc.s;
  const lang = pc.lang;
  const h = clamp(sInt(s, 'banner_height', 420), 160, 720);
  if (!banners.length) {
    return html`<section class="hero brand" style="--hero-h:${h}px"><div class="wrap hero-in"><p class="hero-eyebrow">${s.site_tagline}</p><h2>${s.site_name}</h2><p class="hero-cta"><a class="btn" href="#ilanlar">${t(lang, 'hero_cta')}</a> <a class="btn ghost light" href="${href(pc, '/iletisim')}">${t(lang, 'hero_cta2')}</a></p></div></section>`;
  }
  const many = banners.length > 1;
  const slides = banners.map((b, i) => {
    const img = mediaUrl(b.image);
    const link = safeUrl(b.link);
    const l2 = link && /^\//.test(link) ? href(pc, link) : link;
    return html`<div class="slide${i === 0 ? ' active' : ''}" role="group" aria-roledescription="slide" aria-label="${t(lang, 'slide_n', i + 1)}">
      ${img ? html`<img src="${img}" alt="${b.title}" loading="${i === 0 ? 'eager' : 'lazy'}" decoding="async">` : ''}
      ${link && !b.button ? html`<a class="slide-link" href="${l2}" aria-label="${b.title || t(lang, 'slide_n', i + 1)}"></a>` : ''}
      ${b.title || b.subtitle || b.button ? html`<div class="slide-cap"><div class="wrap">${b.title ? html`<h2>${b.title}</h2>` : ''}${b.subtitle ? html`<p>${b.subtitle}</p>` : ''}${b.button && link ? html`<a class="btn" href="${l2}">${b.button}</a>` : ''}</div></div>` : ''}
    </div>`;
  });
  return html`<section class="hero slider" data-slider data-autoplay="${sBool(s, 'banner_autoplay') ? 1 : 0}" data-interval="${clamp(sInt(s, 'banner_interval', 6), 2, 30) * 1000}" style="--hero-h:${h}px" aria-roledescription="carousel" aria-label="${s.site_name}">
  <div class="slides">${slides}</div>
  ${many ? html`<button type="button" class="hero-btn hero-prev" aria-label="${t(lang, 'slide_prev')}">‹</button><button type="button" class="hero-btn hero-next" aria-label="${t(lang, 'slide_next')}">›</button><div class="dots">${banners.map((b, i) => html`<button type="button" class="dot${i === 0 ? ' active' : ''}" aria-label="${t(lang, 'slide_n', i + 1)}"></button>`)}</div>` : ''}
</section>`;
}

async function homePage(request, env, url, pc) {
  const s = pc.s;
  const f = parseFilters(url, s);
  const { view, set } = resolveView(request, url, s);
  const banners = await all(env, 'SELECT * FROM banners WHERE visible=1 ORDER BY sort_key, id');
  const res = await queryListings(env, pc, f);
  const { ch, tagMap } = await listingPageCommon(env, pc, f);
  const blogN = clamp(sInt(s, 'home_blog_count', 3), 0, 12);
  let blogHtml = '';
  if (blogN > 0 && res.page === 1 && !hasFilter(f)) {
    const posts = await all(env, "SELECT * FROM posts WHERE status='yayinda' ORDER BY published_at DESC, id DESC LIMIT ?", blogN);
    if (posts.length) {
      const cats = new Map((await all(env, 'SELECT * FROM blog_categories')).map((c) => [c.id, c]));
      blogHtml = html`<section class="home-blog"><h2 class="sec-title">${t(pc.lang, 'latest_posts')}</h2><div class="post-grid">${posts.map((p) => postCardHtml(pc, p, cats))}</div><p class="more-row"><a class="btn ghost" href="${href(pc, '/blog')}">${t(pc.lang, 'blog_all')}</a></p></section>`;
    }
  }
  const body = html`<h1 class="sr-only">${s.site_name}</h1>
${String(s.home_intro_html || '').trim() ? html`<div class="intro rich">${raw(sanitizeHtml(s.home_intro_html))}</div>` : ''}
${listingsBlock(pc, request, url, res, f, view, tagMap, '/', ch, { filter: sBool(s, 'show_filter_bar') })}
${blogHtml}`;
  const side = await buildSidebar(env, pc, 'listings');
  const extra = set ? { 'set-cookie': cookie('lv', view, { maxAge: 31536000 }), 'cache-control': 'private, no-cache' } : {};
  return respond(pc, 200, {
    isHome: true, title: s.site_name, path: '/', body, sidebar: side, hero: heroHtml(pc, banners), bodyClass: 'p-home',
    noindex: hasFilter(f), canonicalSearch: res.page > 1 && !hasFilter(f) ? qs({ page: res.page }) : '',
    image: s.logo_url ? mediaUrl(s.logo_url) : '',
  }, 200 && extra);
}

// ---------- menu ilan sayfasi (/satilik-villa, /diger-ilanlar ...)
async function menuListingPage(request, env, url, pc, item) {
  const s = pc.s;
  const f = parseFilters(url, s);
  const [ta, tb] = splitTarget(item.target);
  const others = ta === '*' && tb === '*';
  const base = { type: ta && ta !== '*' ? ta : '', cat: tb && tb !== '*' ? tb : '', others };
  const { view, set } = resolveView(request, url, s);
  const res = await queryListings(env, pc, f, base);
  const { ch, tagMap } = await listingPageCommon(env, pc, f);
  const title = pickLang(item, 'label', pc.lang);
  const slug = menuSlug(item);
  const body = html`${crumbs(pc, [[t(pc.lang, 'home'), href(pc, '/')], [title, '']])}<h1 class="page-title">${title}</h1>
${listingsBlock(pc, request, url, res, f, view, tagMap, `/${slug}`, ch, { fixed: { type: !!base.type || others, cat: !!base.cat || others } })}`;
  const side = await buildSidebar(env, pc, 'listings');
  const extra = set ? { 'set-cookie': cookie('lv', view, { maxAge: 31536000 }), 'cache-control': 'private, no-cache' } : {};
  return respond(pc, 200, { title, path: `/${slug}`, body, sidebar: side, noindex: hasFilter(f), canonicalSearch: res.page > 1 && !hasFilter(f) ? qs({ page: res.page }) : '', bodyClass: 'p-list' }, 200, extra);
}

async function searchPage(request, env, url, pc) {
  const s = pc.s;
  const f = parseFilters(url, s);
  const { view, set } = resolveView(request, url, s);
  const res = await queryListings(env, pc, f);
  const { ch, tagMap } = await listingPageCommon(env, pc, f);
  let posts = [];
  if (f.q) {
    const like = `%${likeEsc(f.q)}%`;
    posts = await all(env, "SELECT * FROM posts WHERE status='yayinda' AND (title LIKE ? ESCAPE '\\' OR title_en LIKE ? ESCAPE '\\' OR title_ru LIKE ? ESCAPE '\\' OR excerpt LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\') ORDER BY published_at DESC LIMIT 6", like, like, like, like, like);
  }
  const cats = new Map((await all(env, 'SELECT * FROM blog_categories')).map((c) => [c.id, c]));
  const body = html`<h1 class="page-title">${t(pc.lang, 'search')}${f.q ? html`: <span class="q">${f.q}</span>` : ''}</h1>
${listingsBlock(pc, request, url, res, f, view, tagMap, '/ara', ch, {})}
${posts.length ? html`<section class="home-blog"><h2 class="sec-title">${t(pc.lang, 'blog')}</h2><div class="post-grid">${posts.map((p) => postCardHtml(pc, p, cats))}</div></section>` : ''}`;
  const extra = set ? { 'set-cookie': cookie('lv', view, { maxAge: 31536000 }), 'cache-control': 'private, no-cache' } : {};
  return respond(pc, 200, { title: t(pc.lang, 'search'), path: '/ara', body, sidebar: await buildSidebar(env, pc, 'listings'), noindex: true, bodyClass: 'p-search' }, 200, extra);
}

// ---------- ilan detayi
function approxPoint(lat, lon, id) {
  const h = (Math.imul(Number(id) || 1, 2654435761) >>> 0);
  const ang = ((h % 360) * Math.PI) / 180;
  const dist = 80 + ((h >>> 9) % 120);
  const dLat = (dist * Math.cos(ang)) / 111320;
  const dLon = (dist * Math.sin(ang)) / (111320 * Math.max(0.2, Math.cos((lat * Math.PI) / 180)));
  return [lat + dLat, lon + dLon];
}
const hasOverride = (l, k) => !!parseJson(l.overrides, {})[k];

export function contactForm(pc, { listingId = 0, next = '', url = null } = {}) {
  const lang = pc.lang;
  const sent = url && url.searchParams.get('sent');
  const err = url && url.searchParams.get('err');
  const errKey = { invalid: 'f_err_invalid', rate: 'f_err_rate', fast: 'f_err_fast', origin: 'f_err_origin' }[err];
  return html`<form class="cform" id="form" method="post" action="${href(pc, '/iletisim')}">
  ${sent ? html`<p class="notice ok" role="status">${t(lang, 'f_ok')}</p>` : ''}${errKey ? html`<p class="notice err" role="alert">${t(lang, errKey)}</p>` : ''}
  <input type="hidden" name="listing_id" value="${listingId || ''}"><input type="hidden" name="next" value="${next}"><input type="hidden" name="ts" value="${Date.now()}">
  <div class="hp" aria-hidden="true"><label>Website <input type="text" name="website" tabindex="-1" autocomplete="off"></label></div>
  <label>${t(lang, 'f_name')} <span class="req">*</span><input type="text" name="name" required maxlength="100" autocomplete="name"></label>
  <div class="two"><label>${t(lang, 'f_phone')}<input type="tel" name="phone" maxlength="40" autocomplete="tel"></label><label>${t(lang, 'f_email')}<input type="email" name="email" maxlength="120" autocomplete="email"></label></div>
  <label>${t(lang, 'f_message')} <span class="req">*</span><textarea name="body" rows="5" required maxlength="4000"></textarea></label>
  <button type="submit" class="btn">${t(lang, 'f_send')}</button>
</form>`;
}

async function listingDetail(env, url, pc, slug, ctx) {
  const s = pc.s;
  const lang = pc.lang;
  const l = await first(env, `SELECT l.* FROM listings l WHERE l.slug=? AND ${visWhere(s)}`, slug);
  if (!l) return notFound(pc);
  if (ctx && ctx.waitUntil) ctx.waitUntil(run(env, 'UPDATE listings SET views=views+1 WHERE id=?', l.id).catch(() => {}));
  else run(env, 'UPDATE listings SET views=views+1 WHERE id=?', l.id).catch(() => {});
  const title = pickLang(l, 'title', lang);
  const photos = photoList(l);
  const tg = await tagMapOf(env);
  const pr = priceText(pc, l);
  const path = listingSlugPath(l);
  const pageUrl = absUrl(pc, href(pc, path));
  const hide = sBool(s, 'hide_exact_location') || (hasOverride(l, 'hide_exact_location') && truthy(l.hide_exact_location));
  const hasGeo = Number(l.lat) !== 0 && Number(l.lon) !== 0 && Math.abs(Number(l.lat)) <= 90 && Math.abs(Number(l.lon)) <= 180;
  const [mlat, mlon] = hasGeo ? (hide ? approxPoint(Number(l.lat), Number(l.lon), l.id) : [Number(l.lat), Number(l.lon)]) : [0, 0];
  const descText = String(pickLang(l, 'desc', lang, 'desc_tr') || '').trim();
  const descHtml = descText ? raw(esc(descText).replace(/\r?\n/g, '<br>')) : '';
  const yes = t(lang, 'yes');
  const rows = [
    [t(lang, 'listing_no'), l.id], [t(lang, 'type'), typeLabel(lang, l.listing_type)], [t(lang, 'category'), catLabel(lang, l.category)],
    [t(lang, 'district'), l.district], [t(lang, 'neighborhood'), l.neighborhood], [t(lang, 'rooms'), l.rooms], [t(lang, 'baths'), l.bathrooms],
    [t(lang, 'gross_m2'), Number(l.gross_m2) > 0 ? fmtInt(l.gross_m2, lang) : ''], [t(lang, 'net_m2'), Number(l.net_m2) > 0 ? fmtInt(l.net_m2, lang) : ''],
    [t(lang, 'building_age'), l.building_age], [t(lang, 'floor'), l.floor], [t(lang, 'total_floors'), l.total_floors], [t(lang, 'heating'), l.heating], [t(lang, 'deed_status'), l.deed_status],
    [t(lang, 'furnished'), Number(l.furnished) ? yes : ''], [t(lang, 'swap_ok'), Number(l.swap_ok) ? yes : ''], [t(lang, 'credit_ok'), Number(l.credit_ok) ? yes : ''],
    [t(lang, 'published'), fmtDate(l.published_at, lang)],
  ].filter(([, v]) => v !== '' && v !== null && v !== undefined);
  const gallery = photos.length
    ? html`<div class="gallery" data-gallery data-base="/img/l/${l.id}/" data-photos="${photos.join(',')}" data-alt="${title}">
    <div class="g-main"><button type="button" class="g-open" aria-label="${t(lang, 'enlarge')}"><img src="${photoUrl(l.id, photos[0])}" alt="${title}" width="960" height="720" loading="eager" decoding="async"></button>${statusRibbon(pc, l)}
      ${photos.length > 1 ? html`<button type="button" class="cm-btn g-prev" aria-label="${t(lang, 'prev_photo')}">‹</button><button type="button" class="cm-btn g-next" aria-label="${t(lang, 'next_photo')}">›</button>` : ''}<span class="g-count" aria-live="polite">1 / ${photos.length}</span></div>
    ${photos.length > 1 ? html`<ul class="g-thumbs">${photos.map((n, i) => html`<li><button type="button" data-i="${i}" aria-label="${t(lang, 'photo_n', i + 1, photos.length)}"${i === 0 ? raw(' aria-current="true"') : ''}><img src="${photoUrl(l.id, n)}" alt="" width="120" height="90" loading="lazy" decoding="async"></button></li>`)}</ul>` : ''}
  </div>`
    : html`<div class="gallery empty"><span>${t(lang, 'no_photo')}</span>${statusRibbon(pc, l)}</div>`;
  const wa = waDigits(s.whatsapp || l.agent_phone);
  const phone = l.agent_phone || s.phone;
  const waText = t(lang, 'wa_text', l.id, pageUrl);
  const shareText = encodeURIComponent(`${title} ${pageUrl}`);
  const sold = l.prog_status === 'Satıldı' || l.prog_status === 'Kiralandı';
  let similar = '';
  if (sBool(s, 'show_similar')) {
    const rowsS = await all(env, `SELECT l.* FROM listings l WHERE ${visWhere(s)} AND l.id<>? AND (l.category=? OR l.listing_type=?) ORDER BY (l.category=? AND l.listing_type=?) DESC, ABS(l.price-?) ASC, l.id DESC LIMIT 3`, l.id, l.category, l.listing_type, l.category, l.listing_type, Number(l.price) || 0);
    if (rowsS.length) similar = html`<section class="similar"><h2 class="sec-title">${t(lang, 'similar')}</h2><div class="grid cols-3 view-thumb">${rowsS.map((x) => cardHtml(pc, x, tg.map, { view: 'thumb' }))}</div></section>`;
  }
  const body = html`${crumbs(pc, [[t(lang, 'home'), href(pc, '/')], [typeLabel(lang, l.listing_type) || t(lang, 'listings'), ''], [title, '']])}
<article class="detail${sold ? ' is-sold' : ''}">
  <header class="d-head"><h1>${title}</h1><p class="d-loc">${locationText(l)}</p>
    <ul class="d-tags">${tagBadges(pc, l.tags, tg.map, 8)}</ul></header>
  <div class="d-top">
    <div class="d-gal">${gallery}</div>
    <aside class="d-box" aria-label="${t(lang, 'details')}">
      ${l.listing_type ? html`<span class="cm-type d-type ${l.listing_type === 'Kiralık' ? 'rent' : 'sale'}">${typeLabel(lang, l.listing_type)}</span>` : ''}
      <p class="d-price"><strong>${pr.main}</strong>${pr.fx ? html`<small>${pr.fx}</small>` : ''}</p>
      ${l.price_note ? html`<p class="d-note">${l.price_note}</p>` : ''}
      ${sold ? html`<p class="notice err">${t(lang, 'sold_note')}</p>` : ''}
      <div class="d-actions">${wa ? html`<a class="btn wa" href="https://wa.me/${wa}?text=${encodeURIComponent(waText)}" target="_blank" rel="noopener">${t(lang, 'whatsapp')}</a>` : ''}${phone ? html`<a class="btn" href="${telHref(phone)}">${t(lang, 'call')}: ${phone}</a>` : ''}</div>
      ${l.agent_name ? html`<p class="d-agent">${l.agent_name}</p>` : ''}
      <div class="d-share"><span>${t(lang, 'share')}:</span>
        <a href="https://wa.me/?text=${shareText}" target="_blank" rel="noopener">WhatsApp</a>
        <a href="https://www.facebook.com/sharer/sharer.php?u=${encodeURIComponent(pageUrl)}" target="_blank" rel="noopener">Facebook</a>
        <a href="https://twitter.com/intent/tweet?url=${encodeURIComponent(pageUrl)}&amp;text=${encodeURIComponent(title)}" target="_blank" rel="noopener">X</a>
        <button type="button" data-copy="${pageUrl}">${t(lang, 'copy_link')}</button></div>
    </aside>
  </div>
  ${descHtml ? html`<section class="d-sec"><h2>${t(lang, 'description')}</h2><div class="d-desc">${descHtml}</div></section>` : ''}
  <section class="d-sec"><h2>${t(lang, 'features')}</h2><table class="feat"><tbody>${rows.map(([k, v]) => html`<tr><th scope="row">${k}</th><td>${v}</td></tr>`)}</tbody></table></section>
  ${hasGeo ? html`<section class="d-sec"><h2>${t(lang, 'map')}</h2><div id="map" class="map" data-lat="${mlat.toFixed(6)}" data-lon="${mlon.toFixed(6)}" data-approx="${hide ? 1 : 0}" data-radius="300" data-css="${LEAFLET}leaflet.min.css" data-js="${LEAFLET}leaflet.min.js" role="region" aria-label="${t(lang, 'map')}"></div>${hide ? html`<p class="muted">${t(lang, 'map_approx')}</p>` : ''}<p><a href="https://www.openstreetmap.org/?mlat=${mlat.toFixed(6)}&amp;mlon=${mlon.toFixed(6)}#map=15/${mlat.toFixed(5)}/${mlon.toFixed(5)}" target="_blank" rel="noopener">${t(lang, 'open_map')}</a></p></section>` : ''}
  <section class="d-sec d-form"><h2>${t(lang, 'ask_about')}</h2>${contactForm(pc, { listingId: l.id, next: href(pc, path), url })}</section>
  ${similar}
</article>`;
  const jsonld = {
    '@context': 'https://schema.org', '@type': 'RealEstateListing', name: title, url: pageUrl,
    description: truncate(descText || title, 300), datePosted: l.published_at || undefined,
    image: photos.slice(0, 6).map((n) => absUrl(pc, photoUrl(l.id, n))),
    address: { '@type': 'PostalAddress', streetAddress: hide ? undefined : (l.address || undefined), addressLocality: l.district || 'Fethiye', addressRegion: 'Muğla', addressCountry: 'TR' },
    geo: hasGeo && !hide ? { '@type': 'GeoCoordinates', latitude: Number(l.lat), longitude: Number(l.lon) } : undefined,
    offers: Number(l.price) > 0 ? { '@type': 'Offer', price: Number(l.price), priceCurrency: String(l.currency || 'TRY').toUpperCase() === 'TL' ? 'TRY' : String(l.currency || 'TRY').toUpperCase(), availability: sold ? 'https://schema.org/SoldOut' : 'https://schema.org/InStock', url: pageUrl } : undefined,
  };
  const side = await buildSidebar(env, pc, 'listings');
  return respond(pc, 200, {
    title, desc: descText || `${title} — ${locationText(l)}`, path, body, sidebar: side, jsonld: [jsonld], bodyClass: 'p-detail',
    image: photos.length ? photoUrl(l.id, photos[0]) : '', ogType: 'website', ogTitle: title,
  });
}

// ---------- blog
async function blogList(env, url, pc, catSlug) {
  const s = pc.s;
  const lang = pc.lang;
  const cats = await all(env, 'SELECT * FROM blog_categories ORDER BY kind, sort_key, id');
  const catMap = new Map(cats.map((c) => [c.id, c]));
  let cat = null;
  if (catSlug) { cat = cats.find((c) => c.slug === catSlug); if (!cat) return notFound(pc); }
  const per = clamp(sInt(s, 'blog_per_page', 9), 3, 48);
  const where = cat ? "status='yayinda' AND category_id=?" : "status='yayinda'";
  const args = cat ? [cat.id] : [];
  const total = Number(await scalar(env, `SELECT COUNT(*) FROM posts WHERE ${where}`, ...args)) || 0;
  const pages = Math.max(1, Math.ceil(total / per));
  const page = clamp(toInt(url.searchParams.get('page'), 1), 1, pages);
  const posts = await all(env, `SELECT * FROM posts WHERE ${where} ORDER BY published_at DESC, id DESC LIMIT ? OFFSET ?`, ...args, per, (page - 1) * per);
  const title = cat ? pickLang(cat, 'name', lang) : (s.blog_title || t(lang, 'blog'));
  const basePath = cat ? `/blog/${encodeURIComponent(cat.slug)}` : '/blog';
  const body = html`${crumbs(pc, cat ? [[t(lang, 'home'), href(pc, '/')], [s.blog_title || t(lang, 'blog'), href(pc, '/blog')], [title, '']] : [[t(lang, 'home'), href(pc, '/')], [title, '']])}
<h1 class="page-title">${title}</h1>
${cats.length ? html`<ul class="chips" aria-label="${t(lang, 'w_categories')}"><li><a href="${href(pc, '/blog')}"${!cat ? raw(' aria-current="page"') : ''}>${t(lang, 'blog_all')}</a></li>${cats.map((c) => html`<li><a href="${href(pc, `/blog/${encodeURIComponent(c.slug)}`)}"${cat && cat.id === c.id ? raw(' aria-current="page"') : ''}>${pickLang(c, 'name', lang)}</a></li>`)}</ul>` : ''}
${posts.length ? html`<div class="post-grid">${posts.map((p) => postCardHtml(pc, p, catMap))}</div>` : html`<p class="empty">${t(lang, 'blog_empty')}</p>`}
${paginationHtml(pc, basePath, {}, page, pages)}`;
  return respond(pc, 200, { title, path: basePath, body, sidebar: await buildSidebar(env, pc, 'blog'), canonicalSearch: page > 1 ? qs({ page }) : '', bodyClass: 'p-blog' });
}

async function postPage(env, url, pc, slug, ctx) {
  const lang = pc.lang;
  const p = await first(env, "SELECT * FROM posts WHERE slug=? AND status='yayinda'", slug);
  if (!p) return notFound(pc);
  if (ctx && ctx.waitUntil) ctx.waitUntil(run(env, 'UPDATE posts SET views=views+1 WHERE id=?', p.id).catch(() => {}));
  const cat = p.category_id ? await first(env, 'SELECT * FROM blog_categories WHERE id=?', p.category_id) : null;
  const catMap = new Map((await all(env, 'SELECT * FROM blog_categories')).map((c) => [c.id, c]));
  const title = pickLang(p, 'title', lang);
  const bodyHtml = sanitizeHtml(pickLang(p, 'body', lang));
  const img = postImage(p);
  const path = `/yazi/${encodeURIComponent(p.slug)}`;
  const related = await all(env, "SELECT * FROM posts WHERE status='yayinda' AND id<>? ORDER BY (category_id=?) DESC, published_at DESC LIMIT 3", p.id, p.category_id || 0);
  const desc = p.excerpt || truncate(stripTags(bodyHtml), 180);
  const body = html`${crumbs(pc, [[t(lang, 'home'), href(pc, '/')], [pc.s.blog_title || t(lang, 'blog'), href(pc, '/blog')], ...(cat ? [[pickLang(cat, 'name', lang), href(pc, `/blog/${encodeURIComponent(cat.slug)}`)]] : []), [title, '']])}
<article class="post">
  <header><h1 class="page-title">${title}</h1><p class="post-meta">${fmtDate(p.published_at, lang)}${p.author ? html` · ${t(lang, 'by')}: ${p.author}` : ''}${cat ? html` · <a href="${href(pc, `/blog/${encodeURIComponent(cat.slug)}`)}">${pickLang(cat, 'name', lang)}</a>` : ''}</p></header>
  ${img ? html`<figure class="post-img"><img src="${img}" alt="${title}" width="960" height="540" loading="eager" decoding="async"></figure>` : ''}
  <div class="rich post-body">${raw(bodyHtml)}</div>
</article>
${related.length ? html`<section class="related"><h2 class="sec-title">${t(lang, 'related')}</h2><div class="post-grid">${related.map((r) => postCardHtml(pc, r, catMap))}</div></section>` : ''}`;
  const jsonld = { '@context': 'https://schema.org', '@type': 'BlogPosting', headline: title, datePublished: p.published_at || undefined, dateModified: p.updated_at || p.published_at || undefined, image: img ? [img.startsWith('http') ? img : absUrl(pc, img)] : undefined, author: p.author ? { '@type': 'Person', name: p.author } : undefined, publisher: { '@type': 'Organization', name: pc.s.site_name }, mainEntityOfPage: absUrl(pc, href(pc, path)) };
  return respond(pc, 200, { title, desc, path, body, sidebar: await buildSidebar(env, pc, 'blog'), jsonld: [jsonld], image: img, ogType: 'article', ogTitle: title, bodyClass: 'p-post' });
}

// ---------- sayfalar
async function staticPage(env, url, pc, slug) {
  const lang = pc.lang;
  const pg = await first(env, 'SELECT * FROM pages WHERE slug=? AND visible=1', slug);
  const isContact = slug === 'iletisim';
  if (!pg && !isContact) return notFound(pc);
  const title = pg ? pickLang(pg, 'title', lang) : t(lang, 'contact');
  const bodyHtml = pg ? sanitizeHtml(pickLang(pg, 'body', lang)) : '';
  const s = pc.s;
  const wa = waDigits(s.whatsapp);
  const contact = isContact ? html`<div class="contact-grid"><div><h2>${t(lang, 'contact_info')}</h2><ul class="w-list w-contact">${s.phone ? html`<li>${t(lang, 'phone')}: <a href="${telHref(s.phone)}">${s.phone}</a></li>` : ''}${wa ? html`<li><a href="https://wa.me/${wa}" rel="noopener" target="_blank">${t(lang, 'whatsapp')}</a></li>` : ''}${s.email ? html`<li>${t(lang, 'email')}: <a href="mailto:${s.email}">${s.email}</a></li>` : ''}${s.address ? html`<li>${t(lang, 'address')}: ${s.address}</li>` : ''}</ul></div><div><h2>${t(lang, 'contact_us')}</h2>${contactForm(pc, { next: href(pc, '/iletisim'), url })}</div></div>` : '';
  const body = html`${crumbs(pc, [[t(lang, 'home'), href(pc, '/')], [title, '']])}<article class="page"><h1 class="page-title">${title}</h1><div class="rich">${raw(bodyHtml)}</div>${contact}</article>`;
  return respond(pc, 200, { title, desc: stripTags(bodyHtml), path: `/${encodeURIComponent(slug)}`, body, sidebar: await buildSidebar(env, pc, 'blog'), bodyClass: 'p-page' });
}

// ---------- iletisim formu (POST /iletisim)
async function postContact(request, env, url, pc) {
  const lang = pc.lang;
  let fd;
  try { fd = await request.formData(); } catch { fd = new FormData(); }
  const g = (k, n = 200) => String(fd.get(k) ?? '').trim().slice(0, n);
  let next = g('next', 300);
  if (!/^\/(?!\/)[^\\\s]*$/.test(next)) next = href(pc, '/iletisim');
  next = next.split('#')[0].replace(/([?&])(sent|err)=[^&]*/g, '$1').replace(/[?&]$/, '');
  const back = (key, val) => redirect(`${next}${next.includes('?') ? '&' : '?'}${key}=${val}#form`, 303);
  const origin = request.headers.get('origin');
  if (origin && origin !== url.origin && origin !== pc.origin) return textResponse(t(lang, 'f_err_origin'), 403);
  if (g('website', 500)) return back('sent', 1); // bal tuzagi: sessizce basarili gibi
  const ts = toNum(g('ts', 20), 0);
  if (!(ts > 0) || Date.now() - ts < 2500) return back('err', 'fast');
  const ip = clientIp(request);
  const since = new Date(Date.now() - 3600 * 1000).toISOString();
  const cnt = Number(await scalar(env, 'SELECT COUNT(*) FROM messages WHERE ip=? AND created_at>?', ip, since)) || 0;
  if (cnt >= 5) return back('err', 'rate');
  const name = g('name', 100); const phone = g('phone', 40); const email = g('email', 120); const body = g('body', 4000);
  const phoneOk = phone.replace(/\D/g, '').length >= 7;
  const emailOk = /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]{2,}$/.test(email);
  if (!name || !body || !(phoneOk || emailOk) || (email && !emailOk) || (phone && !phoneOk)) return back('err', 'invalid');
  let lid = toInt(g('listing_id', 12), 0);
  if (lid > 0 && !(await first(env, 'SELECT id FROM listings WHERE id=?', lid))) lid = 0;
  await run(env, 'INSERT INTO messages(name,phone,email,body,listing_id,ip,created_at,is_read) VALUES(?,?,?,?,?,?,?,0)', name, phone, email, body, Math.max(0, lid), ip, nowIso());
  return back('sent', 1);
}

// ---------- sitemap / robots
function robots(s, origin) {
  const lines = ['User-agent: *', 'Disallow: /admin', 'Disallow: /api', 'Allow: /', '', `Sitemap: ${origin}/sitemap.xml`];
  const extra = String(s.robots_extra || '').trim();
  if (extra) lines.push('', extra);
  return textResponse(lines.join('\n') + '\n', 200, 'text/plain; charset=utf-8', { 'cache-control': 'public, max-age=3600', 'x-content-type-options': 'nosniff' });
}
async function sitemap(env, s, origin) {
  const { def, langs } = langList(s);
  const pcLike = { defLang: def };
  const ents = [{ path: '/', last: '' }, { path: '/blog', last: '' }];
  const add = (p, last) => ents.push({ path: p, last: String(last || '').slice(0, 10) });
  for (const r of await all(env, 'SELECT slug,updated_at FROM pages WHERE visible=1')) add(`/${encodeURIComponent(r.slug)}`, r.updated_at);
  const seen = new Set();
  for (const r of await all(env, "SELECT * FROM menu_items WHERE menu='main' AND visible=1 AND kind='listings'")) { const sl = menuSlug(r); if (!seen.has(sl)) { seen.add(sl); add(`/${sl}`, ''); } }
  for (const r of await all(env, 'SELECT slug FROM blog_categories')) add(`/blog/${encodeURIComponent(r.slug)}`, '');
  for (const r of await all(env, "SELECT slug,updated_at,published_at FROM posts WHERE status='yayinda' ORDER BY id DESC LIMIT 5000")) add(`/yazi/${encodeURIComponent(r.slug)}`, r.updated_at || r.published_at);
  for (const r of await all(env, `SELECT l.slug,l.updated_at,l.published_at FROM listings l WHERE ${visWhere(s)} ORDER BY l.id DESC LIMIT 5000`)) add(`/ilan/${encodeURIComponent(r.slug)}`, r.updated_at || r.published_at);
  const u = (lg, p) => `${origin}${langPrefix({ defLang: def }, lg)}${p === '/' ? '' : p}` || origin;
  const full = (lg, p) => { const x = u(lg, p); return x === origin ? `${origin}/` : x; };
  void pcLike;
  const xml = ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">'];
  for (const e of ents) {
    for (const lg of langs) {
      xml.push(`<url><loc>${esc(full(lg, e.path))}</loc>${e.last && /^\d{4}-\d\d-\d\d$/.test(e.last) ? `<lastmod>${e.last}</lastmod>` : ''}${langs.map((x) => `<xhtml:link rel="alternate" hreflang="${x}" href="${esc(full(x, e.path))}"/>`).join('')}</url>`);
    }
  }
  xml.push('</urlset>');
  return textResponse(xml.join('\n'), 200, 'application/xml; charset=utf-8', { 'cache-control': 'public, max-age=900', 'x-content-type-options': 'nosniff' });
}

// ---------- bakim modu
function maintenance(pc) {
  const body = html`<section class="notfound maint"><h1>${t(pc.lang, 'maintenance')}</h1><p>${pc.s.maintenance_text}</p></section>`;
  return respond(pc, 503, { title: t(pc.lang, 'maintenance'), body, noindex: true, bodyClass: 'p-maint' }, 503, { 'retry-after': '3600', 'cache-control': 'no-store' });
}

// ---------- giris
export async function handlePublic(request, env, url, ctx) {
  const method = request.method;
  if (!['GET', 'HEAD', 'POST'].includes(method)) return textResponse('Method Not Allowed', 405, 'text/plain; charset=utf-8', { allow: 'GET, HEAD, POST' });
  const p0 = url.pathname;
  if (p0 === '/api' || p0.startsWith('/api/')) return new Response(JSON.stringify({ ok: false, error: 'Bulunamadı' }), { status: 404, headers: { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' } });
  const s0 = await getSettings(env);
  const origin = String(env.SITE_URL || url.origin).replace(/\/+$/, '');
  if (p0 === '/robots.txt') return robots(s0, origin);
  const { pc, path, redirectTo } = await buildCtx(request, env, url);
  if (sBool(pc.s, 'maintenance_mode')) return maintenance(pc);
  if (p0 === '/sitemap.xml') return await sitemap(env, pc.s, origin);
  if (redirectTo !== null) return redirect(redirectTo || '/', 301);
  if (method === 'POST') {
    if (path === '/iletisim') return await postContact(request, env, url, pc);
    return textResponse('Method Not Allowed', 405, 'text/plain; charset=utf-8', { allow: 'GET, HEAD' });
  }
  if (path === '/') return await homePage(request, env, url, pc);
  if (path === '/ara') return await searchPage(request, env, url, pc);
  if (path === '/blog') return await blogList(env, url, pc, '');
  let m;
  if ((m = path.match(/^\/blog\/([^/]+)$/))) return await blogList(env, url, pc, decodeSeg(m[1]));
  if ((m = path.match(/^\/yazi\/([^/]+)$/))) return await postPage(env, url, pc, decodeSeg(m[1]), ctx);
  if ((m = path.match(/^\/ilan\/([^/]+)$/))) return await listingDetail(env, url, pc, decodeSeg(m[1]), ctx);
  if ((m = path.match(/^\/([^/]+)$/))) {
    const slug = decodeSeg(m[1]);
    const item = pc.menuRows.find((r) => r.kind === 'listings' && menuSlug(r) === slug);
    if (item) return await menuListingPage(request, env, url, pc, item);
    return await staticPage(env, url, pc, slug);
  }
  return notFound(pc);
}

export { jsonLdScript };
