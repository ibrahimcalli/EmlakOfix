// Yonetim paneli girisi: /admin ve /admin/* (giris, kurulum, SSO, oturum, CSRF, bolum yonlendirme).
import { html, raw, esc, redirect, htmlResponse, jsonResponse, safeUrl } from './util.js';
import { all, run, logActivity } from './env.js';
import { hashPassword } from './crypto.js';
import {
  clientIp, getSession, createSession, sessionCookie, clearSessionCookie, destroySession, csrfFor, csrfOk, preCsrf, preCsrfOk,
  adminCount, createFirstAdmin, tryLogin, ssoLogin, validUsername, passwordProblem,
} from './auth.js';
import { readBody, readFlash, shell, bareShell, secured, jsonErr, notFoundPage, MAX_BODY, fld } from './admin_ui.js';
import * as listings from './admin_listings.js';
import * as content from './admin_content.js';
import * as system from './admin_system.js';

const MODULES = [listings, content, system];
const last = (o, k) => (o[k] ? String(o[k][o[k].length - 1]) : '');

function loginPage(msg, token, next, setCookie, status = 200) {
  const body = html`<div class="card login"><img class="logo" src="/logo.png" alt="Logo"><h1>Yönetici Girişi</h1>
${msg ? html`<div class="flash err" role="alert">${msg}</div>` : ''}
<form method="post" action="/admin/giris" autocomplete="on"><input type="hidden" name="_csrf" value="${token}"><input type="hidden" name="next" value="${next || ''}">
<div class="fld"><label for="f_username">Kullanıcı adı</label><input id="f_username" name="username" type="text" autocomplete="username" required autofocus></div>
<div class="fld"><label for="f_password">Parola</label><input id="f_password" name="password" type="password" autocomplete="current-password" required></div>
<button class="btn primary block" type="submit">Giriş yap</button></form>
<p class="muted small">Programdan açtıysanız bu ekranı görmezsiniz; doğrudan panele girilir.</p></div>`;
  return bareShell('Giriş', body, status, setCookie ? [setCookie] : []);
}
function setupPage(msg, token, setCookie, values = {}, status = 200) {
  const body = html`<div class="card login"><h1>İlk Kurulum</h1><p>Henüz yönetici yok. İlk yönetici hesabını oluşturun (bu ekran bir kez görünür).</p>
${msg ? html`<div class="flash err" role="alert">${msg}</div>` : ''}
<form method="post" action="/admin/kurulum"><input type="hidden" name="_csrf" value="${token}">
${fld({ name: 'username', label: 'Kullanıcı adı', value: values.username || '', req: true, help: '3-32 karakter: küçük harf, rakam, nokta, tire, alt çizgi' })}
${fld({ name: 'display_name', label: 'Görünen ad', value: values.display_name || '' })}
${fld({ name: 'password', label: 'Parola', type: 'password', req: true, help: 'En az 8 karakter' })}
${fld({ name: 'password2', label: 'Parola (tekrar)', type: 'password', req: true })}
<button class="btn primary block" type="submit">Yöneticiyi oluştur</button></form></div>`;
  return bareShell('Kurulum', body, status, setCookie ? [setCookie] : []);
}
const safeNext = (n) => (typeof n === 'string' && /^\/admin(\/[\w\-./?=&%]*)?$/.test(n) && !n.startsWith('/admin/giris') ? n : '/admin');

async function handleLogin(request, env, ip) {
  const { token, setCookie } = await preCsrf(env, request);
  const url = new URL(request.url);
  if (!(await adminCount(env))) return redirect('/admin/kurulum', 302);
  if (request.method === 'GET' || request.method === 'HEAD') return loginPage('', token, safeNext(url.searchParams.get('next')), setCookie);
  if (request.method !== 'POST') return secured(new Response('Yöntem desteklenmiyor', { status: 405 }));
  const { form } = await readBody(request);
  const next = safeNext(last(form, 'next'));
  if (!(await preCsrfOk(env, request, last(form, '_csrf')))) return loginPage('Güvenlik doğrulaması başarısız. Sayfayı yenileyip tekrar deneyin.', token, next, setCookie, 403);
  const r = await tryLogin(env, ip, last(form, 'username'), last(form, 'password'));
  if (!r.ok) return loginPage(r.error, token, next, setCookie, r.status);
  const s = await createSession(env, r.admin.id, ip);
  await logActivity(env, r.admin.username, 'Giriş yaptı');
  return secured(redirect(next, 303, [sessionCookie(s.token, s.maxAge)]));
}

async function handleSetup(request, env, ip) {
  if (await adminCount(env)) return secured(new Response('Bulunamadı', { status: 404 }));
  const { token, setCookie } = await preCsrf(env, request);
  if (request.method === 'GET' || request.method === 'HEAD') return setupPage('', token, setCookie);
  if (request.method !== 'POST') return secured(new Response('Yöntem desteklenmiyor', { status: 405 }));
  const { form } = await readBody(request);
  const values = { username: last(form, 'username').trim().toLowerCase(), display_name: last(form, 'display_name').trim().slice(0, 80) };
  if (!(await preCsrfOk(env, request, last(form, '_csrf')))) return setupPage('Güvenlik doğrulaması başarısız. Sayfayı yenileyin.', token, setCookie, values, 403);
  if (!validUsername(values.username)) return setupPage('Kullanıcı adı 3-32 karakter olmalı (küçük harf, rakam, . _ -).', token, setCookie, values, 400);
  const pw = last(form, 'password');
  if (pw !== last(form, 'password2')) return setupPage('Parolalar uyuşmuyor.', token, setCookie, values, 400);
  const prob = passwordProblem(pw);
  if (prob) return setupPage(prob, token, setCookie, values, 400);
  const a = await createFirstAdmin(env, values.username, values.display_name, await hashPassword(pw));
  if (!a) return secured(new Response('Bulunamadı', { status: 404 }));
  const s = await createSession(env, a.id, ip);
  await logActivity(env, values.username, 'İlk yönetici oluşturuldu');
  return secured(redirect('/admin', 303, [sessionCookie(s.token, s.maxAge)]));
}

async function handleSso(request, env, url, ip) {
  if (request.method !== 'GET' && request.method !== 'HEAD') return secured(new Response('Yöntem desteklenmiyor', { status: 405 }));
  const r = await ssoLogin(env, url, ip);
  if (!r.ok) {
    return bareShell('Giriş yapılamadı', html`<div class="card login"><h1>Giriş yapılamadı</h1><div class="flash err" role="alert">${r.error}</div><p><a class="btn" href="/admin/giris">Parola ile giriş</a></p></div>`, r.status || 403);
  }
  const s = await createSession(env, r.adminId, ip);
  await logActivity(env, r.username, 'Programdan (SSO) giriş yaptı');
  return secured(redirect('/admin', 303, [sessionCookie(s.token, s.maxAge)]));
}

export async function handleAdmin(request, env, url) {
  const ip = clientIp(request);
  const method = request.method.toUpperCase();
  const path = url.pathname.replace(/\/+$/, '') || '/admin';
  const parts = path.split('/').slice(2).map((p) => { try { return decodeURIComponent(p); } catch { return p; } });
  const sec = parts[0] || '';
  try {
    if (sec === 'giris') return await handleLogin(request, env, ip);
    if (sec === 'kurulum') return await handleSetup(request, env, ip);
    if (sec === 'sso') return await handleSso(request, env, url, ip);

    if (!['GET', 'HEAD', 'POST'].includes(method)) return secured(new Response('Yöntem desteklenmiyor', { status: 405, headers: { allow: 'GET, HEAD, POST' } }));
    const isAjax = request.headers.get('x-requested-with') === 'fetch' || /application\/json/.test(request.headers.get('accept') || '');
    const session = await getSession(env, request);
    if (!session) {
      if (!(await adminCount(env))) return redirect('/admin/kurulum', 302);
      if (isAjax) return secured(jsonResponse({ ok: false, error: 'Oturum gerekli', login: '/admin/giris' }, 401));
      const next = method === 'GET' && path !== '/admin' ? `?next=${encodeURIComponent(path + url.search)}` : '';
      return secured(redirect('/admin/giris' + next, 302));
    }
    const csrf = await csrfFor(env, session.key);
    let form = {}, files = {};
    if (method === 'POST') {
      const len = parseInt(request.headers.get('content-length') || '0', 10);
      if (len > MAX_BODY) return secured(jsonResponse({ ok: false, error: 'İstek çok büyük (en fazla 16 MB).' }, 413));
      const b = await readBody(request);
      form = b.form; files = b.files;
      const supplied = request.headers.get('x-csrf-token') || last(form, '_csrf');
      if (!(await csrfOk(env, session.key, supplied))) {
        if (isAjax) return secured(jsonResponse({ ok: false, error: 'Güvenlik doğrulaması başarısız. Sayfayı yenileyin.' }, 403));
        return secured(htmlResponse(html`<!doctype html><meta charset="utf-8"><title>Reddedildi</title><link rel="stylesheet" href="/admin.css"><body class="adm bare"><main class="loginbox"><div class="card login"><h1>İstek reddedildi</h1><p>Güvenlik doğrulaması başarısız oldu (CSRF). Sayfayı yenileyip tekrar deneyin.</p><p><a class="btn" href="/admin">Panele dön</a></p></div></main>`, 403));
      }
    }
    const ctx = {
      request, env, url, method, ip, session, admin: session.admin, csrf, form, files, isAjax, parts,
      flash: method === 'GET' ? readFlash(request) : null,
      who: session.admin.username,
      s: (k, d = '') => (form[k] ? String(form[k][form[k].length - 1]).trim() : d),
      r: (k, d = '') => (form[k] ? String(form[k][form[k].length - 1]) : d),
      all: (k) => (form[k] || []).map(String),
      file: (k) => (files[k] || [])[0] || null,
      fileList: (k) => files[k] || [],
    };
    if (sec === 'cikis') {
      if (method !== 'POST') return redirect('/admin', 302);
      await destroySession(env, request);
      await logActivity(env, ctx.who, 'Çıkış yaptı');
      return secured(redirect('/admin/giris', 303, [clearSessionCookie()]));
    }
    for (const m of MODULES) {
      const res = await m.handle(ctx, parts);
      if (res) return res;
    }
    if (isAjax) return jsonErr('Bulunamadı', 404);
    return notFoundPage(ctx);
  } catch (e) {
    console.error('ADMIN HATA', path, e && e.stack || e);
    if (request.headers.get('x-requested-with') === 'fetch') return secured(jsonResponse({ ok: false, error: 'Sunucu hatası: ' + String(e && e.message || e).slice(0, 200) }, 500));
    return secured(htmlResponse(html`<!doctype html><meta charset="utf-8"><title>Hata</title><link rel="stylesheet" href="/admin.css"><body class="adm bare"><main class="loginbox"><div class="card login"><h1>Bir hata oluştu</h1><p>İşlem tamamlanamadı. Lütfen tekrar deneyin; sürerse Testler bölümünü çalıştırın.</p><p><a class="btn" href="/admin">Panele dön</a></p></div></main>`, 500));
  }
}
