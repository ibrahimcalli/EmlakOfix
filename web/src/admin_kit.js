// Yonetim paneli testleri icin oturum yardimcisi: __test__admin yoneticisi + cerez + CSRF.
import { run, first } from './env.js';
import { call, TP } from './testkit.js';
import { createSession, csrfFor, SESSION_COOKIE } from './auth.js';
import { hashPassword } from './crypto.js';

export async function adminSession(env, name = `${TP}admin`) {
  let a = await first(env, 'SELECT id FROM admins WHERE username=?', name);
  if (!a) {
    await run(env, "INSERT INTO admins(username,display_name,pw_hash,source,active,created_at) VALUES(?,?,?,?,1,?)", name, 'Test Yönetici', await hashPassword('Test-parola-123'), 'local', new Date().toISOString());
    a = await first(env, 'SELECT id FROM admins WHERE username=?', name);
  }
  const s = await createSession(env, a.id, '127.0.0.1');
  const csrf = await csrfFor(env, s.key);
  const cookie = `${SESSION_COOKIE}=${s.token}`;
  const get = (path, headers = {}) => call(env, path, { headers: { cookie, ...headers } });
  // fields: nesne (dizi degerleri tekrar eder); File/Blob degerleri multipart olur
  const post = (path, fields = {}, { csrfToken = csrf, ajax = false, multipart = false } = {}) => {
    const hasFile = multipart || Object.values(fields).some((v) => v instanceof Blob);
    const headers = { cookie }; if (ajax) headers['x-requested-with'] = 'fetch';
    let body;
    if (hasFile) { body = new FormData(); body.append('_csrf', csrfToken); for (const [k, v] of Object.entries(fields)) for (const x of Array.isArray(v) ? v : [v]) body.append(k, x); }
    else { body = new URLSearchParams({ _csrf: csrfToken }); for (const [k, v] of Object.entries(fields)) for (const x of Array.isArray(v) ? v : [v]) body.append(k, String(x)); headers['content-type'] = 'application/x-www-form-urlencoded'; }
    return call(env, path, { method: 'POST', headers, body });
  };
  return { id: a.id, token: s.token, csrf, cookie, get, post, name };
}
