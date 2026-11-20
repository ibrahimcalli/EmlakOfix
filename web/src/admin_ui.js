// Yonetim paneli ortak arayuz parcalari: sayfa kabugu, form alanlari, sayfalama, govde okuma, gorsel yukleme.
import { html, raw, esc, redirect, htmlResponse, jsonResponse, cookie, parseCookies, parseJson, randomHexSafe } from './admin_util.js';
import { FLASH_COOKIE } from './auth.js';

export const MAX_BODY = 16 * 1024 * 1024;
export const MAX_IMAGE = 8 * 1024 * 1024;

export const NAV = [
  ['Genel', [['/admin', 'Panel'], ['/admin/mesajlar', 'Mesajlar']]],
  ['İçerik', [
    ['/admin/ilanlar', 'İlanlar'], ['/admin/etiketler', 'Etiketler'], ['/admin/menu', 'Menü'],
    ['/admin/yazilar', 'Blog yazıları'], ['/admin/haberler', 'Haberler'], ['/admin/kategoriler', 'Blog kategorileri'],
    ['/admin/sayfalar', 'Sayfalar'], ['/admin/bannerlar', 'Bannerlar'], ['/admin/medya', 'Medya kitaplığı'],
  ]],
  ['Site', [['/admin/ayarlar/gorunum', 'Tema ve Görünüm'], ['/admin/ayarlar/genel', 'Genel ayarlar']]],
  ['Sistem', [['/admin/kullanicilar', 'Kullanıcılar'], ['/admin/yedek', 'Yedek'], ['/admin/gunluk', 'Etkinlik günlüğü'], ['/admin/testler', 'Testler']]],
];

export const security = () => ({
  'cache-control': 'no-store',
  'x-frame-options': 'DENY',
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'same-origin',
  'content-security-policy': "default-src 'self'; img-src 'self' data: blob: https:; style-src 'self' 'unsafe-inline'; script-src 'self'; frame-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
  'x-robots-tag': 'noindex, nofollow',
});

// ---- Cerez ile tek seferlik bildirim
export function flashCookie(kind, msg) { return cookie(FLASH_COOKIE, JSON.stringify({ k: kind, m: String(msg).slice(0, 300) }), { path: '/admin', httpOnly: true, secure: true, sameSite: 'Lax', maxAge: 60 }); }
export const clearFlash = () => cookie(FLASH_COOKIE, '', { path: '/admin', httpOnly: true, secure: true, sameSite: 'Lax', maxAge: 0 });
export function readFlash(request) {
  const c = parseCookies(request.headers.get('cookie'))[FLASH_COOKIE];
  const o = c ? parseJson(c, null) : null;
  return o && o.m ? { kind: o.k === 'err' ? 'err' : 'ok', msg: String(o.m) } : null;
}

// ---- Yanitlar
export function secured(res) {
  for (const [k, v] of Object.entries(security())) res.headers.set(k, v);
  return res;
}
export function goto(ctx, location, kind, msg) {
  const cookies = kind ? [flashCookie(kind, msg)] : [];
  if (ctx.isAjax) return secured(jsonResponse({ ok: kind !== 'err', message: msg || '', redirect: location }, kind === 'err' ? 400 : 200, cookies.length ? { 'set-cookie': cookies[0] } : {}));
  return secured(redirect(location, 303, cookies));
}
export const okRedirect = (ctx, location, msg) => goto(ctx, location, 'ok', msg);
export const errRedirect = (ctx, location, msg) => goto(ctx, location, 'err', msg);
export const jsonErr = (msg, status = 400) => secured(jsonResponse({ ok: false, error: msg }, status));
export const jsonOk = (obj = {}) => secured(jsonResponse({ ok: true, ...obj }));

export function notFoundPage(ctx, what = 'Sayfa') {
  return shell(ctx, { title: 'Bulunamadı', body: html`<div class="card"><h2>${what} bulunamadı</h2><p>Aradığınız kayıt silinmiş veya hiç var olmamış olabilir.</p><p><a class="btn" href="/admin">Panele dön</a></p></div>` }, 404);
}

// ---- Sayfa kabugu
export function csrfField(ctx) { return html`<input type="hidden" name="_csrf" value="${ctx.csrf}">`; }
export function postForm(ctx, action, inner, { cls = '', confirm = '', multipart = false, ajax = false, id = '' } = {}) {
  return html`<form method="post" action="${action}" class="${cls}"${confirm ? raw(` data-confirm="${esc(confirm)}"`) : ''}${multipart ? raw(' enctype="multipart/form-data"') : ''}${ajax ? raw(' data-ajax="1"') : ''}${id ? raw(` id="${esc(id)}"`) : ''}>${csrfField(ctx)}${inner}</form>`;
}
export function btnPost(ctx, action, label, { cls = 'btn sm', confirm = '', fields = {} } = {}) {
  const hid = Object.entries(fields).map(([k, v]) => html`<input type="hidden" name="${k}" value="${v}">`);
  return postForm(ctx, action, html`${hid}<button class="${cls}" type="submit">${label}</button>`, { cls: 'inline', confirm });
}

export function shell(ctx, { title, active = '', body, head = '' }, status = 200) {
  const path = active || ctx.url.pathname;
  const nav = NAV.map(([grp, items]) => html`<div class="navgrp"><div class="navt">${grp}</div>${items.map(([href, label]) => {
    const on = href === '/admin' ? path === '/admin' : path === href || path.startsWith(href + '/');
    return html`<a href="${href}" class="${on ? 'on' : ''}"${on ? raw(' aria-current="page"') : ''}>${label}</a>`;
  })}</div>`);
  const flash = ctx.flash ? html`<div class="flash ${ctx.flash.kind}" role="status">${ctx.flash.msg}</div>` : '';
  const page = html`<!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow"><meta name="csrf" content="${ctx.csrf}"><title>${title} · Yönetim</title><link rel="stylesheet" href="/admin.css"><link rel="icon" href="/favicon.png">${head}</head><body class="adm">
<input type="checkbox" id="navtoggle" class="navtoggle" aria-hidden="true">
<header class="top"><label for="navtoggle" class="burger" aria-label="Menüyü aç/kapat"><span></span><span></span><span></span></label><a class="brand" href="/admin">Yönetim Paneli</a><span class="grow"></span><a class="btn ghost sm" href="/" target="_blank" rel="noopener">Siteyi aç</a></header>
<aside class="side"><nav aria-label="Yönetim menüsü">${nav}</nav>
<div class="me"><div><strong>${ctx.admin.display_name}</strong>${ctx.admin.source === 'program' ? html` <span class="badge prog">program</span>` : ''}<div class="muted">${ctx.admin.username}</div></div>${postForm(ctx, '/admin/cikis', html`<button class="btn sm ghost" type="submit">Çıkış</button>`, { cls: 'inline' })}</div></aside>
<main class="main" id="main"><h1>${title}</h1>${flash}${body}</main>
<script src="/admin.js" defer></script></body></html>`;
  const res = secured(htmlResponse(page, status));
  if (ctx.flash) res.headers.append('set-cookie', clearFlash());
  return res;
}

// Giris/kurulum gibi oturumsuz sayfalar
export function bareShell(title, body, status = 200, extraHeaders = []) {
  const page = html`<!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow"><title>${title} · Yönetim</title><link rel="stylesheet" href="/admin.css"><link rel="icon" href="/favicon.png"></head><body class="adm bare"><main class="loginbox">${body}</main><script src="/admin.js" defer></script></body></html>`;
  const res = secured(htmlResponse(page, status));
  for (const c of extraHeaders) res.headers.append('set-cookie', c);
  return res;
}

// ---- Form alanlari
const optPair = (o) => (Array.isArray(o) ? o : [o, o]);
export function fld(o) {
  const { name, label, type = 'text', value = '', options = [], help = '', req = false, rows = 4, cls = '', ph = '', min, max, step, flag = '', attrs = {}, maxlength } = o;
  const id = `f_${name}`;
  const extra = raw(Object.entries(attrs).map(([k, v]) => ` ${esc(k)}="${esc(v)}"`).join('') + (req ? ' required' : '') + (min !== undefined ? ` min="${esc(min)}"` : '') + (max !== undefined ? ` max="${esc(max)}"` : '') + (step !== undefined ? ` step="${esc(step)}"` : '') + (maxlength ? ` maxlength="${esc(maxlength)}"` : ''));
  const v = value ?? '';
  let input;
  if (type === 'textarea' || type === 'code') input = html`<textarea id="${id}" name="${name}" rows="${rows}" class="${type === 'code' ? 'mono' : ''}" placeholder="${ph}"${extra}>${v}</textarea>`;
  else if (type === 'select') input = html`<select id="${id}" name="${name}"${extra}>${options.map((op) => { const [val, lab] = optPair(op); return html`<option value="${val}"${String(val) === String(v) ? raw(' selected') : ''}>${lab}</option>`; })}</select>`;
  else if (type === 'bool') {
    return html`<div class="fld chk ${cls}"><input type="hidden" name="${name}" value="0"><label><input type="checkbox" id="${id}" name="${name}" value="1"${String(v) === '1' || v === true ? raw(' checked') : ''}> ${label}</label>${flag}${help ? html`<small>${help}</small>` : ''}</div>`;
  } else if (type === 'color') {
    const c = /^#[0-9a-fA-F]{6}$/.test(String(v)) ? v : '#000000';
    input = html`<span class="colorrow"><input type="color" id="${id}" name="${name}" value="${c}"${extra}><code>${c}</code></span>`;
  } else if (type === 'image') {
    input = html`<span class="imgrow"><input type="text" id="${id}" name="${name}" value="${v}" placeholder="${ph || '/media/...'}"${extra}><button type="button" class="btn sm" data-upload data-target="${id}" data-preview="pv_${name}">Yükle</button></span><img class="pv" id="pv_${name}" src="${v}" alt="" ${v ? '' : raw('hidden')}>`;
  } else input = html`<input type="${type === 'number' ? 'number' : type === 'password' ? 'password' : type === 'url' ? 'url' : 'text'}" id="${id}" name="${name}" value="${v}" placeholder="${ph}"${extra}>`;
  return html`<div class="fld ${cls}"><label for="${id}">${label}${req ? html` <b class="req">*</b>` : ''}</label>${flag}${input}${help ? html`<small>${help}</small>` : ''}</div>`;
}

export function pageLinks(url, page, pages) {
  if (pages <= 1) return '';
  const link = (p, label, cur) => {
    const u = new URLSearchParams(url.searchParams);
    u.set('sayfa', String(p));
    return html`<a href="${url.pathname}?${u.toString()}" class="${cur ? 'on' : ''}"${cur ? raw(' aria-current="page"') : ''}>${label}</a>`;
  };
  const items = [];
  const from = Math.max(1, page - 3), to = Math.min(pages, page + 3);
  if (page > 1) items.push(link(page - 1, '‹ Önceki'));
  if (from > 1) items.push(link(1, '1'), html`<span>…</span>`);
  for (let p = from; p <= to; p++) items.push(link(p, String(p), p === page));
  if (to < pages) items.push(html`<span>…</span>`, link(pages, String(pages)));
  if (page < pages) items.push(link(page + 1, 'Sonraki ›'));
  return html`<nav class="pager" aria-label="Sayfalama">${items}</nav>`;
}

export function emptyRow(cols, msg = 'Kayıt yok.') { return html`<tr><td colspan="${cols}" class="muted center">${msg}</td></tr>`; }

// ---- Govde okuma
export async function readBody(request) {
  const ct = request.headers.get('content-type') || '';
  const form = {};
  const files = {};
  const add = (o, k, v) => { (o[k] ||= []).push(v); };
  try {
    if (/multipart\/form-data|application\/x-www-form-urlencoded/i.test(ct)) {
      const fd = await request.formData();
      for (const [k, v] of fd.entries()) { if (typeof v === 'string') add(form, k, v); else add(files, k, v); }
    } else if (/json/i.test(ct)) {
      const o = await request.json();
      for (const [k, v] of Object.entries(o && typeof o === 'object' ? o : {})) form[k] = (Array.isArray(v) ? v : [v]).map((x) => (x !== null && typeof x === 'object' ? JSON.stringify(x) : String(x ?? '')));
    }
  } catch { return { form, files, bad: true }; }
  return { form, files };
}

// ---- Gorsel yukleme
export function sniffImage(b) {
  if (b.length > 3 && b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return { ext: 'jpg', type: 'image/jpeg' };
  if (b.length > 8 && b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return { ext: 'png', type: 'image/png' };
  if (b.length > 12 && b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46 && b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50) return { ext: 'webp', type: 'image/webp' };
  if (b.length > 6 && b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x38) return { ext: 'gif', type: 'image/gif' };
  return null;
}
// {ok, bytes, kind} | {ok:false, error}
export async function checkImage(file, { allowGif = false } = {}) {
  if (!file || typeof file === 'string' || typeof file.arrayBuffer !== 'function') return { ok: false, error: 'Dosya seçilmedi.' };
  if (!file.size) return { ok: false, error: 'Dosya boş.' };
  if (file.size > MAX_IMAGE) return { ok: false, error: 'Dosya 8 MB sınırını aşıyor.' };
  const bytes = new Uint8Array(await file.arrayBuffer());
  const kind = sniffImage(bytes);
  if (!kind || (kind.ext === 'gif' && !allowGif)) return { ok: false, error: allowGif ? 'Yalnızca JPEG, PNG, WebP veya GIF yüklenebilir.' : 'Yalnızca JPEG, PNG veya WebP yüklenebilir.' };
  return { ok: true, bytes, kind };
}
export const randomName = () => randomHexSafe(8);

// ---- Sayfalama ve arama yardimcilari
export function paging(url, total, per = 25) {
  const pages = Math.max(1, Math.ceil(total / per));
  const page = Math.min(pages, Math.max(1, parseInt(url.searchParams.get('sayfa') || '1', 10) || 1));
  return { page, pages, per, offset: (page - 1) * per };
}
export const likeArg = (q) => '%' + String(q).replace(/[\\%_]/g, '\\$&') + '%';
export const ESC_LIKE = "ESCAPE '\\'";
export const dt = (iso) => (iso ? String(iso).replace('T', ' ').slice(0, 16) : '');
