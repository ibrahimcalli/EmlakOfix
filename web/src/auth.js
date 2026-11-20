// Yonetim paneli kimlik dogrulama: oturum, CSRF, giris siniri, SSO, ilk kurulum.
import { all, first, run, scalar } from './env.js';
import { nowSec, nowIso, parseCookies, cookie, parseJson, clamp, toInt } from './util.js';
import { sha256Hex, hmacHex, safeEqual, randomHex, verifyPassword } from './crypto.js';
import { getSettings, getRaw, setRaw } from './settings.js';

export const SESSION_COOKIE = 'adm_session';
export const PRE_COOKIE = 'adm_pre';
export const FLASH_COOKIE = 'adm_flash';
export const RL_WINDOW = 900; // sn
export const RL_MAX_USER = 8;
export const RL_MAX_IP = 25;
export const SSO_MAX_AHEAD = 300;

const secret = (env) => String(env.SITE_TOKEN || 'site-token-yok');

export function clientIp(request) {
  const h = request.headers;
  const ip = h.get('cf-connecting-ip') || (h.get('x-forwarded-for') || '').split(',')[0].trim() || h.get('x-real-ip') || '0.0.0.0';
  return ip.slice(0, 64);
}

export const validUsername = (u) => /^[a-z0-9._-]{3,32}$/.test(String(u || ''));
export function passwordProblem(p) {
  const s = String(p ?? '');
  if (s.length < 8) return 'Parola en az 8 karakter olmalı.';
  if (s.length > 200) return 'Parola çok uzun.';
  return '';
}

// ---- CSRF
export async function csrfFor(env, sessionKey) { return hmacHex(secret(env), `csrf|${sessionKey}`); }
export async function csrfOk(env, sessionKey, supplied) {
  if (!supplied) return false;
  return safeEqual(await csrfFor(env, sessionKey), String(supplied));
}
// Oturum acilmadan once (giris/kurulum) cift gonderim cerezi
export async function preCsrf(env, request) {
  const c = parseCookies(request.headers.get('cookie'))[PRE_COOKIE];
  const val = c && /^[0-9a-f]{32}$/.test(c) ? c : randomHex(16);
  const token = await hmacHex(secret(env), `pre|${val}`);
  return { token, setCookie: c === val ? null : cookie(PRE_COOKIE, val, { path: '/admin', httpOnly: true, secure: true, sameSite: 'Lax', maxAge: 3600 }), val };
}
export async function preCsrfOk(env, request, supplied) {
  const c = parseCookies(request.headers.get('cookie'))[PRE_COOKIE];
  if (!c || !supplied) return false;
  return safeEqual(await hmacHex(secret(env), `pre|${c}`), String(supplied));
}

// ---- Oturum
export async function getSession(env, request) {
  const c = parseCookies(request.headers.get('cookie'))[SESSION_COOKIE];
  if (!c || !/^[0-9a-f]{64}$/.test(c)) return null;
  const key = await sha256Hex(c);
  const row = await first(env, 'SELECT s.expires_at AS expires_at, a.id AS id, a.username AS username, a.display_name AS display_name, a.source AS source, a.active AS active FROM sessions s JOIN admins a ON a.id=s.admin_id WHERE s.token=?', key);
  if (!row || row.expires_at < nowSec() || !row.active) return null;
  return { key, expires_at: row.expires_at, admin: { id: row.id, username: row.username, display_name: row.display_name || row.username, source: row.source } };
}
export async function createSession(env, adminId, ip = '') {
  const s = await getSettings(env);
  const hours = clamp(toInt(s.session_hours, 24), 1, 720);
  const token = randomHex(32);
  const key = await sha256Hex(token);
  await run(env, 'DELETE FROM sessions WHERE expires_at<?', nowSec());
  await run(env, 'INSERT INTO sessions(token,admin_id,expires_at,ip) VALUES(?,?,?,?)', key, adminId, nowSec() + hours * 3600, String(ip).slice(0, 64));
  await run(env, 'UPDATE admins SET last_login=? WHERE id=?', nowIso(), adminId);
  return { token, key, maxAge: hours * 3600 };
}
export const sessionCookie = (token, maxAge) => cookie(SESSION_COOKIE, token, { path: '/admin', httpOnly: true, secure: true, sameSite: 'Lax', maxAge });
export const clearSessionCookie = () => cookie(SESSION_COOKIE, '', { path: '/admin', httpOnly: true, secure: true, sameSite: 'Lax', maxAge: 0 });
export async function destroySession(env, request) {
  const c = parseCookies(request.headers.get('cookie'))[SESSION_COOKIE];
  if (c && /^[0-9a-f]{64}$/.test(c)) await run(env, 'DELETE FROM sessions WHERE token=?', await sha256Hex(c));
}

// ---- Giris deneme siniri (ayarlar tablosunda 'rl:<ip>' / 'rl:u:<kullanici>' anahtarlari)
const rlKeyIp = (ip) => `rl:ip:${ip}`;
const rlKeyUser = (u) => `rl:u:${String(u).toLowerCase().slice(0, 64)}`;
async function rlRead(env, key) { return parseJson(await getRaw(env, key), null); }
export async function rlBlocked(env, ip, user) {
  const now = nowSec();
  let wait = 0;
  for (const key of [rlKeyIp(ip), user ? rlKeyUser(user) : null]) {
    if (!key) continue;
    const o = await rlRead(env, key);
    if (o && o.l && o.l > now) wait = Math.max(wait, o.l - now);
  }
  return wait;
}
export async function rlFail(env, ip, user) {
  const now = nowSec();
  const targets = [[rlKeyIp(ip), RL_MAX_IP]];
  if (user) targets.push([rlKeyUser(user), RL_MAX_USER]);
  for (const [key, max] of targets) {
    let o = await rlRead(env, key);
    if (!o || now - (o.t || 0) > RL_WINDOW) o = { c: 0, t: now, l: 0 };
    o.c++;
    if (o.c >= max) o.l = now + RL_WINDOW;
    await setRaw(env, key, JSON.stringify(o));
  }
}
export async function rlClear(env, user) {
  if (user) await run(env, 'DELETE FROM settings WHERE key=?', rlKeyUser(user));
  const now = nowSec();
  await run(env, "DELETE FROM settings WHERE key LIKE 'rl:%' AND COALESCE(json_extract(value,'$.l'),0)<? AND COALESCE(json_extract(value,'$.t'),0)<?", now, now - RL_WINDOW);
}

// ---- Yoneticiler
export async function adminCount(env) { return Number(await scalar(env, 'SELECT COUNT(*) FROM admins')) || 0; }
export async function createFirstAdmin(env, username, display, pwHash) {
  const r = await run(env, "INSERT INTO admins(username,display_name,pw_hash,source,active,created_at,last_login) SELECT ?,?,?,'local',1,?,'' WHERE NOT EXISTS (SELECT 1 FROM admins)", username, display || username, pwHash, nowIso());
  if (!(r.meta && r.meta.changes === 1)) return null;
  return first(env, 'SELECT id FROM admins WHERE username=?', username);
}
// Yerel parola ile giris denemesi. {ok, admin} | {ok:false, status, error}
export async function tryLogin(env, ip, username, password) {
  const u = String(username || '').trim().toLowerCase().slice(0, 64);
  const row = u ? await first(env, 'SELECT id,username,pw_hash,active FROM admins WHERE username=?', u) : null;
  const wait = await rlBlocked(env, ip, row ? u : '');
  if (wait > 0) return { ok: false, status: 429, error: `Çok fazla hatalı deneme. ${Math.ceil(wait / 60)} dakika sonra tekrar deneyin.` };
  const good = await verifyPassword(String(password ?? ''), row ? row.pw_hash : '');
  if (!row || !good || !row.active || !row.pw_hash) {
    await rlFail(env, ip, row ? u : '');
    return { ok: false, status: 401, error: 'Kullanıcı adı veya parola hatalı.' };
  }
  await rlClear(env, u);
  return { ok: true, admin: row };
}

// ---- SSO (program -> site)
export async function ssoLogin(env, url, ip) {
  const fail = (error, status = 403) => ({ ok: false, error, status });
  if (!env.SITE_TOKEN) return fail('SSO yapılandırılmamış.', 503);
  const wait = await rlBlocked(env, ip, '');
  if (wait > 0) return fail('Çok fazla hatalı deneme.', 429);
  const q = url.searchParams;
  const u = String(q.get('u') || '').trim();
  const n = String(q.get('n') || '').trim().slice(0, 80);
  const exp = String(q.get('exp') || '');
  const sig = String(q.get('sig') || '').toLowerCase();
  if (!u || u.length > 64 || /[\u0000-\u001f|]/.test(u) || !/^\d{1,12}$/.test(exp) || !/^[0-9a-f]{64}$/.test(sig)) { await rlFail(env, ip, ''); return fail('Geçersiz bağlantı.', 400); }
  const want = await hmacHex(env.SITE_TOKEN, `${u}|${exp}`);
  if (!(await safeEqual(want, sig))) { await rlFail(env, ip, ''); return fail('İmza geçersiz.'); }
  const now = nowSec();
  const e = parseInt(exp, 10);
  if (e < now) return fail('Bağlantının süresi dolmuş. Programdan yeniden açın.');
  if (e > now + SSO_MAX_AHEAD) return fail('Bağlantı geçersiz (süre çok ileride).');
  const used = `sso:${sig}`;
  if (await getRaw(env, used)) return fail('Bu bağlantı daha önce kullanılmış. Programdan yeniden açın.');
  await setRaw(env, used, String(e));
  await run(env, "DELETE FROM settings WHERE key LIKE 'sso:%' AND CAST(value AS INTEGER)<?", now);
  let ex = await first(env, 'SELECT id,active,source FROM admins WHERE username=?', u);
  if (ex) {
    if (!ex.active) return fail('Bu hesap pasifleştirilmiş.');
    if (ex.source === 'program' && n) await run(env, 'UPDATE admins SET display_name=? WHERE id=?', n, ex.id);
  } else {
    await run(env, "INSERT INTO admins(username,display_name,pw_hash,source,active,created_at,last_login) VALUES(?,?,'','program',1,?,'')", u, n || u, nowIso());
    ex = await first(env, 'SELECT id FROM admins WHERE username=?', u);
  }
  return { ok: true, adminId: ex.id, username: u };
}
export { all };
