// Cloudflare baglamalarinin (D1 = env.DB, R2 = env.MEDIA, statik = env.ASSETS) ince katmani.
// Testlerde ayni arayuzu node:sqlite tabanli taklit (test/shim.js) saglar.
export async function all(env, sql, ...args) {
  const r = await env.DB.prepare(sql).bind(...args).all();
  return r.results || [];
}
export async function first(env, sql, ...args) {
  return (await env.DB.prepare(sql).bind(...args).first()) || null;
}
export async function run(env, sql, ...args) {
  return env.DB.prepare(sql).bind(...args).run();
}
// Birden cok deyim: [[sql, ...args], ...]
export async function batch(env, stmts) {
  return env.DB.batch(stmts.map(([sql, ...a]) => env.DB.prepare(sql).bind(...a)));
}
export async function scalar(env, sql, ...args) {
  const r = await first(env, sql, ...args);
  return r ? Object.values(r)[0] : null;
}
// Etkinlik gunlugu
export async function logActivity(env, who, what) {
  try { await run(env, 'INSERT INTO activity(at,who,what) VALUES(?,?,?)', new Date().toISOString(), String(who || ''), String(what || '').slice(0, 500)); } catch { /* gunluk kritik degil */ }
}
