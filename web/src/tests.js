// Test surucusu: Worker icinde (admin > Testler) ve `node --test` ile ayni dizileri calistirir.
import { run, all } from './env.js';
import { cleanup, snapshotSettings, restoreSettings } from './testkit.js';

const SOURCES = [
  ['core_tests.js', 'Çekirdek'], ['sync_tests.js', 'Program senkronu'], ['public_tests.js', 'Genel site'],
  ['admin_tests.js', 'Yönetim paneli'],
];
// Dinamik import: yuklenemezse bir "yuklenemedi" testi olarak raporlanir. Yalnizca VAR OLAN dosyalar listelenmeli (bundler olmayani cozemez).
const LOADERS = {
  'core_tests.js': () => import('./core_tests.js'), 'sync_tests.js': () => import('./sync_tests.js'), 'public_tests.js': () => import('./public_tests.js'),
  'admin_tests.js': () => import('./admin_tests.js'),
};
async function loadAll() {
  const out = [];
  for (const [file, label] of SOURCES) {
    try { const m = await LOADERS[file](); for (const t of m.tests || []) out.push({ module: t.module || label, name: t.name, run: t.run }); }
    catch (e) {
      const msg = String(e && e.message || e);
      if (/Cannot find module|Failed to load url|not found|No such module/i.test(msg) && file !== 'core_tests.js' && file !== 'sync_tests.js' && file !== 'public_tests.js' && file !== 'admin_tests.js') continue; // opsiyonel dosya
      out.push({ module: label, name: `${file} yüklenebildi mi`, run: async () => { throw new Error('Modül yüklenemedi: ' + msg); } });
    }
  }
  return out;
}
export async function listModules() { return [...new Set((await loadAll()).map((t) => t.module))]; }

function withTimeout(p, ms) { let h; return Promise.race([p, new Promise((_, rej) => { h = setTimeout(() => rej(new Error(`Zaman aşımı (${ms / 1000} sn)`)), ms); })]).finally(() => clearTimeout(h)); }

export async function runAll(env, { module } = {}) {
  const started = Date.now();
  const snap = await snapshotSettings(env);
  const results = [];
  try {
    await cleanup(env);
    const tests = (await loadAll()).filter((t) => !module || t.module === module);
    for (const tc of tests) {
      const t0 = Date.now();
      try { await withTimeout(Promise.resolve(tc.run(env)), 20000); results.push({ module: tc.module, name: tc.name, ok: true, ms: Date.now() - t0, error: '' }); }
      catch (e) { results.push({ module: tc.module, name: tc.name, ok: false, ms: Date.now() - t0, error: String(e && e.message || e).slice(0, 400) }); }
    }
  } finally {
    try { await cleanup(env); } catch { /* temizlik hatasi raporu bozmasin */ }
    try { await restoreSettings(env, snap); } catch { /* ayar geri yuklenemedi */ }
  }
  const passed = results.filter((r) => r.ok).length;
  const rep = { at: new Date().toISOString(), passed, failed: results.length - passed, ms: Date.now() - started, results };
  try {
    await run(env, 'INSERT INTO test_runs(at,passed,failed,report) VALUES(?,?,?,?)', rep.at, rep.passed, rep.failed, JSON.stringify({ module: module || '', ms: rep.ms, results: rep.results.filter((r) => !r.ok).concat(results.length < 400 ? results.filter((r) => r.ok).map((r) => ({ ...r, error: '' })) : []) }));
    await run(env, 'DELETE FROM test_runs WHERE id NOT IN (SELECT id FROM test_runs ORDER BY id DESC LIMIT 30)');
  } catch { /* kayit kritik degil */ }
  return rep;
}
export async function lastRuns(env, n = 10) { return await all(env, 'SELECT id,at,passed,failed,report FROM test_runs ORDER BY id DESC LIMIT ?', n); }

// ---- Tek tek calistirma (Cloudflare: her test KENDI Worker cagrisinda calisir; istek basina sorgu sinirina takilmaz)
export async function listTests(module) {
  return (await loadAll()).map((t, i) => ({ i, module: t.module, name: t.name })).filter((x) => !module || x.module === module);
}
export async function startRun(env) { await cleanup(env); }
export async function runOne(env, i) {
  const tc = (await loadAll())[i];
  if (!tc) return { module: '?', name: `#${i}`, ok: false, ms: 0, error: 'Test bulunamadı' };
  const snap = await snapshotSettings(env);
  const t0 = Date.now();
  try { await withTimeout(Promise.resolve(tc.run(env)), 25000); return { module: tc.module, name: tc.name, ok: true, ms: Date.now() - t0, error: '' }; }
  catch (e) { return { module: tc.module, name: tc.name, ok: false, ms: Date.now() - t0, error: String(e && e.message || e).slice(0, 400) }; }
  finally { try { await restoreSettings(env, snap); } catch { /* yoksay */ } }
}
export async function endRun(env, results, module, ms) {
  try { await cleanup(env); } catch { /* yoksay */ }
  const list = (Array.isArray(results) ? results : []).slice(0, 500).map((r) => ({ module: String(r.module || '').slice(0, 60), name: String(r.name || '').slice(0, 160), ok: !!r.ok, ms: Number(r.ms) || 0, error: String(r.error || '').slice(0, 400) }));
  const passed = list.filter((r) => r.ok).length;
  const rep = { at: new Date().toISOString(), passed, failed: list.length - passed, ms: Number(ms) || 0, results: list };
  await run(env, 'INSERT INTO test_runs(at,passed,failed,report) VALUES(?,?,?,?)', rep.at, rep.passed, rep.failed, JSON.stringify({ module: module || '', ms: rep.ms, results: list }));
  await run(env, 'DELETE FROM test_runs WHERE id NOT IN (SELECT id FROM test_runs ORDER BY id DESC LIMIT 30)');
  return rep;
}
