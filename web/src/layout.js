// Genel site iskeleti ve ortak parcalar: <head>, ustbilgi, menu, altbilgi, kartlar, sayfalama, guvenlik basliklari.
import { esc, html, raw, safeUrl, slugify, fmtPrice, fmtInt, parseJson, stripTags, truncate, fmtDate } from './util.js';
import { t, LANGS, LANG_SHORT, LANG_NAMES, HTML_LANG, OG_LOCALE } from './i18n.js';
import { themeCss, fontLinks, badgeStyle } from './theme.js';

export const ASSET_V = '3';
export const SOCIALS = [
  ['facebook', 'Facebook', 'FB'], ['instagram', 'Instagram', 'IG'], ['youtube', 'YouTube', 'YT'], ['linkedin', 'LinkedIn', 'IN'],
  ['twitter', 'X / Twitter', 'X'], ['pinterest', 'Pinterest', 'PI'], ['tiktok', 'TikTok', 'TT'],
];
const CAT_NAMES = {
  en: { Villa: 'Villa', Daire: 'Apartment', Arazi: 'Land', 'Dükkan': 'Shop', 'İş Yeri': 'Commercial', 'Müstakil Ev': 'Detached house', Tarla: 'Field' },
  ru: { Villa: 'Вилла', Daire: 'Квартира', Arazi: 'Участок', 'Dükkan': 'Магазин', 'İş Yeri': 'Коммерческая', 'Müstakil Ev': 'Отдельный дом', Tarla: 'Поле' },
};

// ---------- yardimcilar
export const typeLabel = (lang, type) => (type === 'Satılık' ? t(lang, 'for_sale') : type === 'Kiralık' ? t(lang, 'for_rent') : type);
export const catLabel = (lang, cat) => (CAT_NAMES[lang] && CAT_NAMES[lang][cat]) || cat;
// Alan secimi: lang=en ve <alan>_en doluysa o, degilse TR
export function pickLang(row, base, lang, trKey = base) {
  const v = lang === 'en' ? row[`${base}_en`] : lang === 'ru' ? row[`${base}_ru`] : '';
  return (v && String(v).trim()) ? v : row[trKey];
}
export function qs(params) {
  const u = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) if (v !== '' && v !== null && v !== undefined && v !== false) u.set(k, String(v));
  const s = u.toString();
  return s ? `?${s}` : '';
}
// Dil onekli adres (yol "/" ile baslar)
export function href(pc, path) {
  const p = path === '/' ? '' : path;
  const full = `${pc.prefix}${p}` || '/';
  return full;
}
export const langPrefix = (pc, lang) => (lang === pc.defLang ? '' : `/${lang}`);
export const absUrl = (pc, path) => `${pc.origin}${path}`;
export function mediaUrl(v) {
  const s = String(v || '').trim();
  if (!s) return '';
  if (/^(https?:)?\/\//i.test(s) || s.startsWith('/')) return safeUrl(s.startsWith('//') ? `https:${s}` : s);
  return `/media/${encodeURIComponent(s)}`;
}
export function photoList(l) {
  const order = parseJson(l.photo_order, []);
  let list = Array.isArray(order) ? order.map((x) => Number(x)).filter((x) => Number.isInteger(x) && x >= 0) : [];
  list = [...new Set(list)];
  if (!list.length) list = Array.from({ length: Math.max(0, Math.min(60, Number(l.photo_count) || 0)) }, (_, i) => i);
  return list.slice(0, 60);
}
export const photoUrl = (id, n) => `/img/l/${id}/${n}.jpg`;
export const listingSlugPath = (l) => `/ilan/${encodeURIComponent(l.slug)}`;

function rateTry(s, cur) {
  const c = String(cur || 'TRY').toUpperCase();
  if (c === 'TRY' || c === 'TL') return 1;
  const r = parseFloat(s[`fx_rate_${c.toLowerCase()}`]);
  return Number.isFinite(r) && r > 0 ? r : 0;
}
// Fiyat metni ve (ayar aciksa) doviz karsiligi
export function priceText(pc, l) {
  if (!(Number(l.price) > 0)) return { main: t(pc.lang, 'price_ask'), fx: '' };
  const main = fmtPrice(l.price, l.currency, pc.lang);
  let fx = '';
  if (String(pc.s.fx_show) === '1') {
    const tryVal = Number(l.price) * rateTry(pc.s, l.currency);
    if (tryVal > 0) {
      const own = String(l.currency || 'TRY').toUpperCase();
      const out = [];
      if (own !== 'TRY' && own !== 'TL') out.push(fmtPrice(tryVal, 'TRY', pc.lang));
      for (const c of ['EUR', 'USD', 'GBP']) {
        if (c === own) continue;
        const r = rateTry(pc.s, c);
        if (r > 0) out.push(fmtPrice(tryVal / r, c, pc.lang));
        if (out.length >= 2) break;
      }
      if (out.length) fx = `${t(pc.lang, 'approx')} ${out.join(' · ')}`;
    }
  }
  return { main, fx };
}

// ---------- menu
export function menuSlug(item) {
  if (item.slug) return item.slug;
  if (item.kind === 'listings') {
    const [a = '', b = ''] = String(item.target || '').split('|');
    if (a.trim() === '*' && b.trim() === '*') return 'diger-ilanlar';
    return slugify(`${a === '*' ? '' : a} ${b === '*' ? '' : b}`) || slugify(item.label) || `liste-${item.id}`;
  }
  return slugify(item.label) || `m-${item.id}`;
}
export function menuHref(pc, item) {
  switch (item.kind) {
    case 'home': return href(pc, '/');
    case 'blog': return href(pc, '/blog');
    case 'listings': return href(pc, `/${menuSlug(item)}`);
    case 'page': return href(pc, `/${String(item.target || item.slug || '').replace(/^\/+/, '')}`);
    case 'blogcat': return href(pc, `/blog/${String(item.target || item.slug || '').replace(/^\/+/, '')}`);
    default: return safeUrl(item.target) || '#';
  }
}
export function buildMenuTree(pc, rows) {
  const top = [];
  const by = new Map();
  for (const r of rows) by.set(r.id, { ...r, label: pickLang(r, 'label', pc.lang), href: menuHref(pc, r), children: [] });
  for (const r of rows) {
    const node = by.get(r.id);
    const parent = r.parent_id ? by.get(r.parent_id) : null;
    if (parent) parent.children.push(node); else top.push(node);
  }
  return top;
}
export function defaultMenuRows() {
  return [
    { id: -1, parent_id: 0, label: 'Ana Sayfa', label_en: 'Home', label_ru: 'Главная', kind: 'home', target: '', slug: '' },
    { id: -2, parent_id: 0, label: 'Blog', label_en: 'Blog', label_ru: 'Блог', kind: 'blog', target: '', slug: '' },
    { id: -3, parent_id: 0, label: 'İletişim', label_en: 'Contact', label_ru: 'Контакты', kind: 'page', target: 'iletisim', slug: '' },
  ];
}

function menuHtml(pc) {
  const cur = pc.here;
  const isCur = (n) => n.href === cur;
  const li = (n) => {
    const kids = n.children || [];
    const active = isCur(n) || kids.some(isCur);
    const ext = n.kind === 'link' && /^https?:/i.test(n.href);
    return html`<li class="${kids.length ? 'has-sub' : ''}${active ? ' active' : ''}">
      <a href="${n.href}"${isCur(n) ? raw(' aria-current="page"') : ''}${ext ? raw(' rel="noopener" target="_blank"') : ''}>${n.label}</a>${kids.length ? html`<button type="button" class="sub-toggle" aria-expanded="false" aria-label="${n.label}"><span aria-hidden="true">▾</span></button><ul class="sub">${kids.map((k) => html`<li><a href="${k.href}"${isCur(k) ? raw(' aria-current="page"') : ''}>${k.label}</a></li>`)}</ul>` : ''}</li>`;
  };
  return html`<ul class="menu">${pc.menu.map(li)}</ul>`;
}

// ---------- ust/alt bilgi
function langSwitch(pc) {
  return html`<ul class="langs" aria-label="${t(pc.lang, 'language')}">${pc.langs.map((l) => html`<li><a href="${langPrefix(pc, l) + (pc.neutral === '/' ? (langPrefix(pc, l) ? '' : '/') : pc.neutral) + pc.search}" hreflang="${l}" lang="${l}"${l === pc.lang ? raw(' aria-current="true"') : ''} title="${LANG_NAMES[l]}">${LANG_SHORT[l]}</a></li>`)}</ul>`;
}
function socialLinks(pc, cls = 'social') {
  const items = SOCIALS.map(([k, name, short]) => [safeUrl(pc.s[`social_${k}`]), name, short]).filter(([u]) => u);
  if (!items.length) return '';
  return html`<ul class="${cls}">${items.map(([u, name, short]) => html`<li><a href="${u}" target="_blank" rel="noopener noreferrer" aria-label="${name}">${short}</a></li>`)}</ul>`;
}
const telHref = (p) => `tel:${String(p || '').replace(/[^\d+]/g, '')}`;

function headerHtml(pc) {
  const s = pc.s;
  const hs = ['classic', 'centered', 'compact'].includes(s.header_style) ? s.header_style : 'classic';
  const logo = safeUrl(s.logo_url) || '/logo.png';
  return html`${String(s.show_topbar) === '1' ? html`<div class="topbar"><div class="wrap topbar-row"><span class="tb-text">${[s.address, s.email].filter(Boolean).join(' · ')}</span>${socialLinks(pc, 'social tb-social')}</div></div>` : ''}
<header class="site-header hs-${hs}${String(s.sticky_header) === '1' ? ' sticky' : ''}">
  <div class="wrap hdr-row">
    <a class="logo" href="${href(pc, '/')}"><img src="${logo}" alt="${s.site_name}" height="56"></a>
    ${s.phone ? html`<a class="hdr-phone" href="${telHref(s.phone)}"><span class="ph-ico" aria-hidden="true">☎</span><span>${s.phone}</span></a>` : ''}
    <button type="button" class="nav-toggle" aria-expanded="false" aria-controls="nav" aria-label="${t(pc.lang, 'menu')}"><span></span><span></span><span></span></button>
    <nav id="nav" class="nav" aria-label="${t(pc.lang, 'menu')}">${menuHtml(pc)}</nav>
    ${langSwitch(pc)}
  </div>
</header>`;
}

export function footerText(pc) {
  const y = String(new Date().getFullYear());
  return String(pc.s.footer_text || '').replace(/\{yıl\}|\{yil\}|\{year\}/gi, y);
}
function footerHtml(pc) {
  const s = pc.s;
  return html`<footer class="site-footer"><div class="wrap ftr-grid">
  <div class="ftr-col"><strong class="ftr-title">${s.site_name}</strong><p>${s.site_tagline}</p></div>
  <div class="ftr-col"><strong class="ftr-title">${t(pc.lang, 'contact_info')}</strong>
    <ul class="ftr-list">${s.phone ? html`<li><a href="${telHref(s.phone)}">${s.phone}</a></li>` : ''}${s.email ? html`<li><a href="mailto:${s.email}">${s.email}</a></li>` : ''}${s.address ? html`<li>${s.address}</li>` : ''}</ul></div>
  <div class="ftr-col">${socialLinks(pc, 'social') ? html`<strong class="ftr-title">${t(pc.lang, 'followus')}</strong>${socialLinks(pc)}` : ''}</div>
</div><div class="ftr-bottom"><div class="wrap"><p>${footerText(pc)}</p></div></div></footer>`;
}

// ---------- guvenlik basliklari
export function securityHeaders(s, isHttps = true) {
  const inlineJs = !!(String(s.custom_head || '').trim() || String(s.custom_footer_html || '').trim() || String(s.analytics_id || '').trim());
  const csp = [
    "default-src 'self'",
    `script-src 'self'${inlineJs ? " 'unsafe-inline'" : ''} https://cdnjs.cloudflare.com https://www.googletagmanager.com`,
    "style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com https://fonts.googleapis.com",
    'font-src https://fonts.gstatic.com data:',
    "img-src 'self' data: blob: https:",
    "connect-src 'self' https://www.google-analytics.com https://*.google-analytics.com https://*.analytics.google.com",
    "frame-src 'self' https://www.google.com https://www.youtube.com",
    "object-src 'none'", "base-uri 'self'", "form-action 'self'", "frame-ancestors 'self'",
  ].join('; ');
  const h = {
    'content-security-policy': csp,
    'x-content-type-options': 'nosniff',
    'x-frame-options': 'SAMEORIGIN',
    'referrer-policy': 'strict-origin-when-cross-origin',
    'permissions-policy': 'geolocation=(), camera=(), microphone=(), payment=()',
  };
  if (isHttps) h['strict-transport-security'] = 'max-age=31536000; includeSubDomains';
  return h;
}

export function jsonLdScript(obj) {
  const j = JSON.stringify(obj).replace(/</g, '\\u003c').replace(/>/g, '\\u003e').replace(/&/g, '\\u0026').replace(/\u2028/g, '\\u2028').replace(/\u2029/g, '\\u2029');
  return raw(`<script type="application/ld+json">${j}</script>`);
}

// ---------- sayfa iskeleti
// o: {title, desc, path (dilden bagimsiz yol), body, sidebar, sideKind, image, ogType, jsonld[], noindex, bodyClass, canonicalSearch}
export function shell(pc, o) {
  const s = pc.s;
  const lang = pc.lang;
  const home = o.isHome;
  const title = home ? `${s.site_name}${s.site_tagline ? ' — ' + s.site_tagline : ''}` : `${o.title}${s.seo_title_suffix || ''}`;
  const desc = truncate(stripTags(o.desc || s.seo_description || ''), 200);
  const canon = o.path !== undefined ? absUrl(pc, href(pc, o.path) + (o.canonicalSearch || '')) : '';
  const hrefl = o.path !== undefined ? pc.langs.map((l) => `<link rel="alternate" hreflang="${l}" href="${esc(absUrl(pc, (langPrefix(pc, l) + (o.path === '/' ? '' : o.path)) || '/'))}">`).join('') + `<link rel="alternate" hreflang="x-default" href="${esc(absUrl(pc, o.path === '/' ? '/' : o.path))}">` : '';
  const favicon = safeUrl(s.favicon_url) || '/favicon.png';
  const og = html`<meta property="og:site_name" content="${s.site_name}"><meta property="og:title" content="${o.ogTitle || o.title || s.site_name}"><meta property="og:description" content="${desc}"><meta property="og:type" content="${o.ogType || 'website'}"><meta property="og:locale" content="${OG_LOCALE[lang]}">${canon ? html`<meta property="og:url" content="${canon}">` : ''}${o.image ? html`<meta property="og:image" content="${o.image.startsWith('http') ? o.image : absUrl(pc, o.image)}"><meta name="twitter:card" content="summary_large_image"><meta name="twitter:image" content="${o.image.startsWith('http') ? o.image : absUrl(pc, o.image)}">` : raw('<meta name="twitter:card" content="summary">')}`;
  const ga = /^(G|GT|AW|UA)-[A-Za-z0-9-]{4,30}$/.test(String(s.analytics_id || '').trim()) ? String(s.analytics_id).trim() : '';
  const analytics = ga ? raw(`<script async src="https://www.googletagmanager.com/gtag/js?id=${esc(ga)}"></script><script>window.dataLayer=window.dataLayer||[];function gtag(){dataLayer.push(arguments)}gtag('js',new Date());gtag('config','${esc(ga)}');</script>`) : '';
  const sideOn = o.sidebar && String(o.sidebar) !== '' && s.sidebar_position !== 'hidden';
  const layoutCls = sideOn ? `layout side-${s.sidebar_position === 'left' ? 'left' : 'right'}` : 'layout side-none';
  const customCss = String(s.custom_css || '').replace(/<\/style/gi, '<\\/style');
  return html`<!doctype html>
<html lang="${HTML_LANG[lang]}" data-theme="${s.theme_mode}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${title}</title>
<meta name="description" content="${desc}">
${o.noindex ? raw('<meta name="robots" content="noindex,nofollow">') : ''}
${canon ? html`<link rel="canonical" href="${canon}">` : ''}${raw(hrefl)}
<link rel="icon" href="${favicon}">
${og}
${raw(fontLinks(s))}
<link rel="stylesheet" href="/site.css?v=${ASSET_V}">
<style id="theme-css">${raw(themeCss(s))}</style>
${customCss.trim() ? html`<style id="custom-css">${raw(customCss)}</style>` : ''}
${(o.jsonld || []).map(jsonLdScript)}
${analytics}
${raw(String(s.custom_head || ''))}
</head>
<body class="${o.bodyClass || ''}">
<a class="skip" href="#main">${t(lang, 'skip')}</a>
${headerHtml(pc)}
${o.hero || ''}
<div class="wrap ${layoutCls}">
  <main id="main" class="main" tabindex="-1">${o.body}</main>
  ${sideOn ? html`<aside class="sidebar" aria-label="${t(lang, 'w_pages')}">${o.sidebar}</aside>` : ''}
</div>
${footerHtml(pc)}
${raw(String(s.custom_footer_html || ''))}
<script src="/site.js?v=${ASSET_V}" defer data-l-prev="${t(lang, 'prev_photo')}" data-l-next="${t(lang, 'next_photo')}" data-l-close="${t(lang, 'close')}" data-l-copied="${t(lang, 'copied')}" data-l-photo="${t(lang, 'photo_n')}"></script>
</body>
</html>`;
}

// ---------- kartlar
export function tagBadges(pc, names, tagMap, limit = 4) {
  const list = (Array.isArray(names) ? names : parseJson(names, [])).filter((x) => typeof x === 'string' && x).slice(0, limit);
  return list.map((n) => {
    const tg = tagMap.get(n);
    const label = tg ? pickLang(tg, 'name', pc.lang) : n;
    return html`<li class="tag" style="${badgeStyle(tg ? tg.color : '')}">${label}</li>`;
  });
}
export function statusRibbon(pc, l) {
  if (l.prog_status === 'Satıldı') return html`<span class="ribbon sold">${t(pc.lang, 'sold')}</span>`;
  if (l.prog_status === 'Kiralandı') return html`<span class="ribbon sold">${t(pc.lang, 'rented')}</span>`;
  return '';
}
export const locationText = (l) => [l.neighborhood, l.district].filter(Boolean).join(', ');
export function metaItems(pc, l) {
  const out = [];
  if (l.rooms) out.push(html`<li>${t(pc.lang, 'rooms')}: <b>${l.rooms}</b></li>`);
  if (l.bathrooms) out.push(html`<li>${t(pc.lang, 'baths')}: <b>${l.bathrooms}</b></li>`);
  const m2 = Number(l.net_m2) > 0 ? l.net_m2 : l.gross_m2;
  if (Number(m2) > 0) out.push(html`<li><b>${fmtInt(m2, pc.lang)}</b> ${t(pc.lang, 'm2')}</li>`);
  return out;
}

export function cardHtml(pc, l, tagMap, opt = {}) {
  const lang = pc.lang;
  const title = pickLang(l, 'title', lang);
  const photos = photoList(l);
  const url = href(pc, listingSlugPath(l));
  const pr = priceText(pc, l);
  const sold = l.prog_status === 'Satıldı' || l.prog_status === 'Kiralandı';
  const cover = photos.length ? html`<img src="${photoUrl(l.id, photos[0])}" alt="${title}" width="640" height="480" loading="${opt.eager ? 'eager' : 'lazy'}" decoding="async">` : html`<span class="cm-empty">${t(lang, 'no_photo')}</span>`;
  const desc = opt.view === 'list' ? truncate(stripTags(pickLang(l, 'desc', lang, 'desc_tr') || l.desc_tr), 180) : '';
  return html`<article class="card${sold ? ' is-sold' : ''}" data-id="${l.id}">
  <div class="cm" data-base="/img/l/${l.id}/" data-photos="${photos.join(',')}">
    <a class="cm-link" href="${url}" aria-label="${title}">${cover}</a>
    <ul class="cm-tags">${tagBadges(pc, l.tags, tagMap)}</ul>
    ${l.listing_type ? html`<span class="cm-type ${l.listing_type === 'Kiralık' ? 'rent' : 'sale'}">${typeLabel(lang, l.listing_type)}</span>` : ''}
    ${statusRibbon(pc, l)}
    ${photos.length > 1 ? html`<button type="button" class="cm-btn cm-prev" aria-label="${t(lang, 'prev_photo')}" hidden>‹</button><button type="button" class="cm-btn cm-next" aria-label="${t(lang, 'next_photo')}" hidden>›</button><span class="cm-count" aria-live="polite" hidden>1 / ${photos.length}</span>` : ''}
  </div>
  <div class="card-body">
    <h3 class="card-title"><a href="${url}">${title}</a></h3>
    <p class="card-loc">${locationText(l)}</p>
    ${desc ? html`<p class="card-desc">${desc}</p>` : ''}
    <p class="card-price"><strong>${pr.main}</strong>${pr.fx ? html`<small>${pr.fx}</small>` : ''}</p>
    <ul class="card-meta">${metaItems(pc, l)}</ul>
  </div>
</article>`;
}

export function postImage(p) { return mediaUrl(p.image); }
export function postCardHtml(pc, p, catMap) {
  const lang = pc.lang;
  const title = pickLang(p, 'title', lang);
  const url = href(pc, `/yazi/${encodeURIComponent(p.slug)}`);
  const img = postImage(p);
  const cat = catMap && catMap.get(p.category_id);
  const ex = p.excerpt || truncate(stripTags(pickLang(p, 'body', lang)), 160);
  return html`<article class="post-card">
  <a class="pc-img" href="${url}" tabindex="-1" aria-hidden="true">${img ? html`<img src="${img}" alt="" width="640" height="400" loading="lazy" decoding="async">` : html`<span class="pc-empty"></span>`}</a>
  <div class="pc-body">
    ${cat ? html`<a class="pc-cat" href="${href(pc, `/blog/${encodeURIComponent(cat.slug)}`)}">${pickLang(cat, 'name', lang)}</a>` : ''}
    <h3><a href="${url}">${title}</a></h3>
    <p class="pc-date">${fmtDate(p.published_at, lang)}</p>
    <p>${truncate(ex, 170)}</p>
    <a class="more" href="${url}">${t(lang, 'read_more')} →</a>
  </div>
</article>`;
}

// ---------- sayfalama
export function paginationHtml(pc, basePath, params, page, pages) {
  if (pages <= 1) return '';
  const link = (n) => href(pc, basePath) + qs({ ...params, page: n > 1 ? n : '' });
  const nums = [];
  for (let i = 1; i <= pages; i++) if (i === 1 || i === pages || Math.abs(i - page) <= 1) nums.push(i);
  const items = [];
  let last = 0;
  for (const n of nums) {
    if (n - last > 1) items.push(html`<li class="gap" aria-hidden="true">…</li>`);
    items.push(n === page ? html`<li><span class="cur" aria-current="page">${n}</span></li>` : html`<li><a href="${link(n)}" aria-label="${t(pc.lang, 'page')} ${n}">${n}</a></li>`);
    last = n;
  }
  return html`<nav class="pager" aria-label="${t(pc.lang, 'pagination')}"><ul>
    ${page > 1 ? html`<li><a rel="prev" href="${link(page - 1)}">‹ ${t(pc.lang, 'prev')}</a></li>` : ''}${items}${page < pages ? html`<li><a rel="next" href="${link(page + 1)}">${t(pc.lang, 'next')} ›</a></li>` : ''}
  </ul><p class="pager-info">${t(pc.lang, 'page_of', page, pages)}</p></nav>`;
}

export { LANGS };
