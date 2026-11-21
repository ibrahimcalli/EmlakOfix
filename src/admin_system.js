// Yonetim paneli: Panel, Ayarlar (tema/gorunum dahil), Kullanicilar, Yedek, Etkinlik gunlugu, Testler.
import { html, raw, esc, redirect, jsonResponse, htmlResponse, parseJson, fmtDateTime, nowIso, toInt } from './util.js';
import { all, first, run, scalar, logActivity } from './env.js';
import { hashPassword } from './crypto.js';
import { SETTING_DEFS, SETTING_GROUPS, normalizeSetting, setSettings, setSetting, getSettings, FONT_OPTIONS, defaultSettings } from './settings.js';
import { shell, fld, postForm, btnPost, pageLinks, paging, likeArg, ESC_LIKE, okRedirect, errRedirect, jsonOk, jsonErr, secured, emptyRow, dt, notFoundPage } from './admin_ui.js';
import { WIDGET_NAMES } from './admin_util.js';
import { validUsername, passwordProblem } from './auth.js';

const GROUP_SLUGS = { genel: 'Genel', gorunum: 'Görünüm', 'ana-sayfa': 'Ana sayfa', 'kenar-cubugu': 'Kenar çubuğu', ilan: 'İlan', blog: 'Blog', iletisim: 'İletişim', gelismis: 'Gelişmiş' };
const slugOfGroup = (g) => Object.keys(GROUP_SLUGS).find((k) => GROUP_SLUGS[k] === g) || 'genel';

export async function handle(ctx, parts) {
  const sec = parts[0] || '';
  if (sec === '') return dashboard(ctx);
  if (sec === 'ayarlar') return settingsPage(ctx, parts[1] || 'genel');
  if (sec === 'kullanicilar') return users(ctx, parts);
  if (sec === 'yedek') return backup(ctx, parts);
  if (sec === 'gunluk') return activity(ctx);
  if (sec === 'testler') return testsPage(ctx, parts);
  return null;
}

// ------------------------------------------------------------------ Panel
async function dashboard(ctx) {
  const env = ctx.env;
  const n = async (sql, ...a) => Number(await scalar(env, sql, ...a)) || 0;
  const stats = [
    ['İlan (toplam)', await n('SELECT COUNT(*) FROM listings'), '/admin/ilanlar'],
    ['Sitede görünen', await n("SELECT COUNT(*) FROM listings WHERE visible=1 AND prog_status IN ('Yayında','Kiralandı','Satıldı')"), '/admin/ilanlar?gorunur=1'],
    ['Gizlenen', await n('SELECT COUNT(*) FROM listings WHERE visible=0'), '/admin/ilanlar?gorunur=0'],
    ['Blog yazısı', await n("SELECT COUNT(*) FROM posts"), '/admin/yazilar'],
    ['Okunmamış mesaj', await n('SELECT COUNT(*) FROM messages WHERE is_read=0'), '/admin/mesajlar'],
    ['Etiket', await n('SELECT COUNT(*) FROM tags'), '/admin/etiketler'],
  ];
  const msgs = await all(env, 'SELECT id,name,phone,body,created_at,is_read FROM messages ORDER BY id DESC LIMIT 5');
  const acts = await all(env, 'SELECT at,who,what FROM activity ORDER BY id DESC LIMIT 8');
  const last = await first(env, 'SELECT at,passed,failed FROM test_runs ORDER BY id DESC LIMIT 1');
  const body = html`<div class="grid stats">${stats.map(([l, v, h]) => html`<a class="card stat" href="${h}"><b>${v}</b><span>${l}</span></a>`)}</div>
<div class="grid two"><div class="card"><h2>Son mesajlar</h2><table class="tbl"><tbody>${msgs.length ? msgs.map((m) => html`<tr><td>${m.is_read ? '' : html`<span class="badge">yeni</span> `}<a href="/admin/mesajlar/${m.id}">${m.name || '(adsız)'}</a><div class="muted">${String(m.body).slice(0, 80)}</div></td><td class="muted nowrap">${dt(m.created_at)}</td></tr>`) : emptyRow(2, 'Mesaj yok.')}</tbody></table></div>
<div class="card"><h2>Son etkinlik</h2><table class="tbl"><tbody>${acts.length ? acts.map((a) => html`<tr><td>${a.what}<div class="muted">${a.who}</div></td><td class="muted nowrap">${dt(a.at)}</td></tr>`) : emptyRow(2, 'Kayıt yok.')}</tbody></table></div></div>
<div class="card"><h2>Hızlı bağlantılar</h2><p class="row"><a class="btn" href="/admin/yazilar/yeni">Yeni blog yazısı</a> <a class="btn" href="/admin/haberler/yeni">Yeni haber</a> <a class="btn" href="/admin/ilanlar">İlanlar</a> <a class="btn" href="/admin/menu">Menü</a> <a class="btn" href="/admin/ayarlar/gorunum">Tema</a> <a class="btn" href="/admin/testler">Testler</a></p>
<p class="muted">${last ? html`Son test: ${dt(last.at)} — ${last.passed} geçti, ${last.failed} başarısız.` : 'Testler henüz çalıştırılmadı.'}</p></div>`;
  return shell(ctx, { title: 'Panel', body });
}

// ------------------------------------------------------------------ Ayarlar
function settingField(d, v) {
  const base = { name: `s_${d.key}`, label: d.label, value: v, help: d.key };
  if (d.type === 'bool') return fld({ ...base, type: 'bool' });
  if (d.type === 'select') return fld({ ...base, type: 'select', options: d.options });
  if (d.type === 'number') return fld({ ...base, type: 'number', min: d.min, max: d.max, step: 'any' });
  if (d.type === 'color') return fld({ ...base, type: 'color' });
  if (d.type === 'image') return fld({ ...base, type: 'image' });
  if (d.type === 'textarea') return fld({ ...base, type: 'textarea', rows: 3 });
  if (d.key === 'sidebar_widgets') return widgetsField(d, v);
  if (d.type === 'json' || d.type === 'code') return fld({ ...base, type: 'code', rows: d.type === 'json' ? 2 : 6 });
  return fld(base);
}
function widgetsField(d, v) {
  const cur = parseJson(v, []).filter((k) => k in WIDGET_NAMES);
  const rest = Object.keys(WIDGET_NAMES).filter((k) => !cur.includes(k));
  return html`<div class="fld"><label>${d.label}</label><input type="hidden" name="s_${d.key}" id="widgets_json" value="${JSON.stringify(cur)}"><div class="widgets" data-widgets="widgets_json">${[...cur, ...rest].map((k) => html`<div class="wrow" data-k="${k}"><input type="checkbox"${cur.includes(k) ? raw(' checked') : ''} aria-label="Göster"> <span>${WIDGET_NAMES[k]}</span> <button type="button" class="btn sm" data-up>↑</button><button type="button" class="btn sm" data-down>↓</button></div>`)}</div><small>İşaretli parçalar kenar çubuğunda bu sırayla görünür.</small></div>`;
}
async function settingsPage(ctx, slug) {
  const group = GROUP_SLUGS[slug];
  if (!group) return notFoundPage(ctx, 'Ayar grubu');
  const defs = SETTING_DEFS.filter((d) => d.group === group);
  if (ctx.method === 'POST') {
    const action = ctx.s('_action');
    if (action === 'reset') {
      for (const d of defs) await setSetting(ctx.env, d.key, d.def);
      await logActivity(ctx.env, ctx.who, `Ayarlar varsayılana döndü: ${group}`);
      return okRedirect(ctx, `/admin/ayarlar/${slug}`, 'Bu gruptaki ayarlar varsayılana döndürüldü.');
    }
    const obj = {};
    for (const d of defs) {
      const k = `s_${d.key}`;
      if (d.type === 'bool') obj[d.key] = ctx.s(k) === '1' ? '1' : '0';
      else if (ctx.form[k]) obj[d.key] = d.type === 'code' || d.type === 'textarea' ? ctx.r(k) : ctx.s(k);
    }
    const n = await setSettings(ctx.env, obj);
    await logActivity(ctx.env, ctx.who, `Ayarlar kaydedildi: ${group} (${n})`);
    return okRedirect(ctx, `/admin/ayarlar/${slug}`, 'Ayarlar kaydedildi.');
  }
  const s = await getSettings(ctx.env, true);
  const tabs = SETTING_GROUPS.map((g) => { const sl = slugOfGroup(g); return html`<a class="tab ${sl === slug ? 'on' : ''}" href="/admin/ayarlar/${sl}">${g}</a>`; });
  const preview = group === 'Görünüm' || group === 'Ana sayfa' || group === 'Kenar çubuğu' ? html`<div class="card"><h2>Canlı önizleme</h2><p class="muted">Kaydettikten sonra önizleme yenilenir.</p><iframe class="preview" src="/?onizleme=${Date.now()}" title="Site önizleme" loading="lazy"></iframe></div>` : '';
  const body = html`<div class="tabs">${tabs}</div>
${postForm(ctx, `/admin/ayarlar/${slug}`, html`<div class="card form">${defs.map((d) => settingField(d, s[d.key]))}<div class="row"><button class="btn primary" type="submit">Kaydet</button><button class="btn danger" type="submit" name="_action" value="reset" data-confirmbtn="Bu gruptaki tüm ayarlar varsayılana dönsün mü?">Varsayılana dön</button></div></div>`, { id: 'settingsform' })}${preview}`;
  return shell(ctx, { title: `Ayarlar — ${group}`, active: slug === 'gorunum' ? '/admin/ayarlar/gorunum' : slug === 'genel' ? '/admin/ayarlar/genel' : '', body });
}

// ------------------------------------------------------------------ Kullanicilar
async function users(ctx, parts) {
  const env = ctx.env;
  if (ctx.method === 'POST') {
    const act = parts[1] || '';
    if (act === 'ekle') {
      const u = ctx.s('username').toLowerCase();
      if (!validUsername(u)) return errRedirect(ctx, '/admin/kullanicilar', 'Kullanıcı adı 3-32 karakter olmalı (küçük harf, rakam, . _ -).');
      const pw = ctx.r('password'); const pr = passwordProblem(pw);
      if (pr) return errRedirect(ctx, '/admin/kullanicilar', pr);
      if (await first(env, 'SELECT id FROM admins WHERE username=?', u)) return errRedirect(ctx, '/admin/kullanicilar', 'Bu kullanıcı adı zaten var.');
      await run(env, "INSERT INTO admins(username,display_name,pw_hash,source,active,created_at) VALUES(?,?,?,'local',1,?)", u, ctx.s('display_name').slice(0, 80) || u, await hashPassword(pw), nowIso());
      await logActivity(env, ctx.who, `Yönetici eklendi: ${u}`);
      return okRedirect(ctx, '/admin/kullanicilar', 'Yönetici eklendi.');
    }
    const id = toInt(parts[2], 0);
    const a = id ? await first(env, 'SELECT * FROM admins WHERE id=?', id) : null;
    if (!a) return errRedirect(ctx, '/admin/kullanicilar', 'Kullanıcı bulunamadı.');
    const activeCount = Number(await scalar(env, 'SELECT COUNT(*) FROM admins WHERE active=1'));
    if (act === 'parola') {
      const pw = ctx.r('password'); const pr = passwordProblem(pw);
      if (pr) return errRedirect(ctx, '/admin/kullanicilar', pr);
      await run(env, 'UPDATE admins SET pw_hash=? WHERE id=?', await hashPassword(pw), id);
      await logActivity(env, ctx.who, `Parola değişti: ${a.username}`);
      return okRedirect(ctx, '/admin/kullanicilar', 'Parola güncellendi.');
    }
    if (act === 'ad') {
      await run(env, 'UPDATE admins SET display_name=? WHERE id=?', ctx.s('display_name').slice(0, 80), id);
      return okRedirect(ctx, '/admin/kullanicilar', 'Ad güncellendi.');
    }
    if (act === 'aktif') {
      const to = a.active ? 0 : 1;
      if (!to && a.active && activeCount <= 1) return errRedirect(ctx, '/admin/kullanicilar', 'Son etkin yönetici pasifleştirilemez.');
      await run(env, 'UPDATE admins SET active=? WHERE id=?', to, id);
      if (!to) await run(env, 'DELETE FROM sessions WHERE admin_id=?', id);
      await logActivity(env, ctx.who, `${to ? 'Etkinleştirildi' : 'Pasifleştirildi'}: ${a.username}`);
      return okRedirect(ctx, '/admin/kullanicilar', to ? 'Etkinleştirildi.' : 'Pasifleştirildi.');
    }
    if (act === 'sil') {
      const total = Number(await scalar(env, 'SELECT COUNT(*) FROM admins'));
      if (total <= 1 || (a.active && activeCount <= 1)) return errRedirect(ctx, '/admin/kullanicilar', 'Son yönetici silinemez.');
      if (a.id === ctx.admin.id) return errRedirect(ctx, '/admin/kullanicilar', 'Kendinizi silemezsiniz.');
      await run(env, 'DELETE FROM sessions WHERE admin_id=?', id);
      await run(env, 'DELETE FROM admins WHERE id=?', id);
      await logActivity(env, ctx.who, `Yönetici silindi: ${a.username}`);
      return okRedirect(ctx, '/admin/kullanicilar', 'Silindi.');
    }
    return errRedirect(ctx, '/admin/kullanicilar', 'Geçersiz işlem.');
  }
  const rows = await all(env, 'SELECT id,username,display_name,source,active,created_at,last_login FROM admins ORDER BY id');
  const body = html`<div class="card"><div class="tablewrap"><table class="tbl"><thead><tr><th>Kullanıcı</th><th>Tür</th><th>Son giriş</th><th>Durum</th><th>İşlemler</th></tr></thead><tbody>${rows.map((a) => html`<tr><td><strong>${a.display_name || a.username}</strong><div class="muted">${a.username}</div></td><td>${a.source === 'program' ? html`<span class="badge prog">program yöneticisi</span>` : html`<span class="badge">yerel</span>`}</td><td class="muted">${dt(a.last_login) || '—'}</td><td>${a.active ? 'Etkin' : html`<span class="badge off">Pasif</span>`}</td>
<td class="actions">${postForm(ctx, `/admin/kullanicilar/parola/${a.id}`, html`<input type="password" name="password" placeholder="Yeni parola" autocomplete="new-password" required><button class="btn sm" type="submit">Parolayı değiştir</button>`, { cls: 'inline' })}
${btnPost(ctx, `/admin/kullanicilar/aktif/${a.id}`, a.active ? 'Pasifleştir' : 'Etkinleştir')}${btnPost(ctx, `/admin/kullanicilar/sil/${a.id}`, 'Sil', { cls: 'btn sm danger', confirm: `${a.username} silinsin mi?` })}</td></tr>`)}</tbody></table></div>
<p class="muted">Programdaki yönetici, programdan “Web yönetim girişi”ne basınca otomatik “program yöneticisi” olarak eklenir; parolası gerekmez.</p></div>
<div class="card"><h2>Yeni yönetici</h2>${postForm(ctx, '/admin/kullanicilar/ekle', html`${fld({ name: 'username', label: 'Kullanıcı adı', req: true })}${fld({ name: 'display_name', label: 'Görünen ad' })}${fld({ name: 'password', label: 'Parola (en az 8)', type: 'password', req: true })}<button class="btn primary" type="submit">Ekle</button>`)}</div>`;
  return shell(ctx, { title: 'Kullanıcılar', body });
}

// ------------------------------------------------------------------ Yedek
const BACKUP_TABLES = ['settings', 'listings', 'posts', 'pages', 'menu_items', 'tags', 'blog_categories', 'banners'];
export async function exportAll(env) {
  const out = { version: 1, at: nowIso(), tables: {} };
  for (const t of BACKUP_TABLES) out.tables[t] = await all(env, `SELECT * FROM ${t}`);
  return out;
}
export async function importAll(env, data) {
  if (!data || typeof data !== 'object' || !data.tables) throw new Error('Yedek biçimi geçersiz.');
  let n = 0;
  for (const t of BACKUP_TABLES) {
    const rows = data.tables[t];
    if (!Array.isArray(rows) || !rows.length) continue;
    const cols = (await all(env, `PRAGMA table_info(${t})`)).map((c) => c.name);
    for (const r of rows) {
      const keys = Object.keys(r).filter((k) => cols.includes(k));
      if (!keys.length) continue;
      await run(env, `INSERT OR REPLACE INTO ${t}(${keys.join(',')}) VALUES(${keys.map(() => '?').join(',')})`, ...keys.map((k) => r[k]));
      n++;
    }
  }
  return n;
}
async function backup(ctx, parts) {
  if (parts[1] === 'indir') {
    const json = JSON.stringify(await exportAll(ctx.env), null, 1);
    return secured(new Response(json, { headers: { 'content-type': 'application/json; charset=utf-8', 'content-disposition': `attachment; filename="site-yedek-${new Date().toISOString().slice(0, 10)}.json"` } }));
  }
  if (ctx.method === 'POST' && parts[1] === 'yukle') {
    const f = ctx.file('file');
    if (!f) return errRedirect(ctx, '/admin/yedek', 'Dosya seçilmedi.');
    try {
      const n = await importAll(ctx.env, JSON.parse(await f.text()));
      await logActivity(ctx.env, ctx.who, `Yedek içe aktarıldı (${n} kayıt)`);
      return okRedirect(ctx, '/admin/yedek', `${n} kayıt içe aktarıldı.`);
    } catch (e) { return errRedirect(ctx, '/admin/yedek', 'İçe aktarılamadı: ' + String(e.message || e).slice(0, 120)); }
  }
  const body = html`<div class="card"><h2>Dışa aktar</h2><p>Ayarlar, ilanlar, yazılar, sayfalar, menü, etiketler, kategoriler ve bannerlar tek JSON dosyasına yazılır. (Fotoğraf ve medya dosyaları R2'de durur; ilan fotoğrafları program tarafından yeniden gönderilebilir.)</p><p><a class="btn primary" href="/admin/yedek/indir">Yedeği indir (.json)</a></p></div>
<div class="card"><h2>İçe aktar</h2><p>Aynı kimlikli kayıtların üzerine yazılır. Önce yedek alın.</p>${postForm(ctx, '/admin/yedek/yukle', html`<input type="file" name="file" accept="application/json,.json" required> <button class="btn" type="submit">İçe aktar</button>`, { multipart: true })}</div>`;
  return shell(ctx, { title: 'Yedek', body });
}

// ------------------------------------------------------------------ Gunluk
async function activity(ctx) {
  const q = ctx.url.searchParams.get('q') || '';
  const where = q ? `WHERE who LIKE ? ${ESC_LIKE} OR what LIKE ? ${ESC_LIKE}` : '';
  const args = q ? [likeArg(q), likeArg(q)] : [];
  const total = Number(await scalar(ctx.env, `SELECT COUNT(*) FROM activity ${where}`, ...args));
  const pg = paging(ctx.url, total, 50);
  const rows = await all(ctx.env, `SELECT at,who,what FROM activity ${where} ORDER BY id DESC LIMIT ? OFFSET ?`, ...args, pg.per, pg.offset);
  const body = html`<form class="row" method="get"><input type="search" name="q" value="${q}" placeholder="Ara…"><button class="btn" type="submit">Ara</button></form>
<div class="card"><div class="tablewrap"><table class="tbl"><thead><tr><th>Zaman</th><th>Kim</th><th>İşlem</th></tr></thead><tbody>${rows.length ? rows.map((r) => html`<tr><td class="nowrap muted">${dt(r.at)}</td><td>${r.who}</td><td>${r.what}</td></tr>`) : emptyRow(3)}</tbody></table></div>${pageLinks(ctx.url, pg.page, pg.pages)}</div>`;
  return shell(ctx, { title: 'Etkinlik günlüğü', body });
}

// ------------------------------------------------------------------ Testler
async function testsPage(ctx, parts) {
  const { runAll, listModules, listTests, runOne, startRun, endRun } = await import('./tests.js');
  if (ctx.method === 'POST' && parts[1] === 'calistir') {
    const module = ctx.s('module');
    const rep = await runAll(ctx.env, module ? { module } : {});
    await logActivity(ctx.env, ctx.who, `Testler çalıştırıldı${module ? ` (${module})` : ''}: ${rep.passed} geçti, ${rep.failed} başarısız`);
    return secured(jsonResponse({ ok: true, report: rep }));
  }
  if (parts[1] === 'liste') return secured(jsonResponse({ ok: true, tests: await listTests(ctx.url.searchParams.get('module') || '') }));
  if (ctx.method === 'POST' && parts[1] === 'baslat') { await startRun(ctx.env); return secured(jsonResponse({ ok: true })); }
  if (ctx.method === 'POST' && parts[1] === 'tek') return secured(jsonResponse({ ok: true, result: await runOne(ctx.env, Number(ctx.s('i'))) }));
  if (ctx.method === 'POST' && parts[1] === 'bitir') {
    let results = []; try { results = JSON.parse(ctx.s('results') || '[]'); } catch { /* bos */ }
    const rep = await endRun(ctx.env, results, ctx.s('module'), Number(ctx.s('ms')));
    await logActivity(ctx.env, ctx.who, `Testler çalıştırıldı${ctx.s('module') ? ` (${ctx.s('module')})` : ''}: ${rep.passed} geçti, ${rep.failed} başarısız`);
    return secured(jsonResponse({ ok: true, report: rep }));
  }
  if (parts[1] === 'rapor') {
    const row = await first(ctx.env, 'SELECT id,at,passed,failed,report FROM test_runs ORDER BY id DESC LIMIT 1');
    return secured(new Response(JSON.stringify(row ? { id: row.id, at: row.at, passed: row.passed, failed: row.failed, ...parseJson(row.report, {}) } : {}, null, 1), { headers: { 'content-type': 'application/json; charset=utf-8', 'content-disposition': 'attachment; filename="test-raporu.json"' } }));
  }
  const modules = await listModules();
  const runs = await all(ctx.env, 'SELECT id,at,passed,failed FROM test_runs ORDER BY id DESC LIMIT 10');
  const body = html`<div class="card"><p>Bu düğmeler sitenin <b>canlı</b> sistemi üzerinde tüm modülleri sınar (veritabanı, API, genel site, yönetim paneli, medya). Test verileri “__test__” önekiyle yazılır ve işlem sonunda silinir; ayarlarınız korunur.</p>
<div class="row"><button class="btn primary" id="runall" data-run="">▶ Tüm testleri çalıştır</button>${modules.map((m) => html`<button class="btn" data-run="${m}">${m}</button>`)}<a class="btn ghost" href="/admin/testler/rapor">Son raporu indir</a></div>
<div id="testout" class="testout" aria-live="polite"></div></div>
<div class="card"><h2>Geçmiş çalıştırmalar</h2><table class="tbl"><tbody>${runs.length ? runs.map((r) => html`<tr><td>${dt(r.at)}</td><td>${r.passed} geçti</td><td>${r.failed ? html`<span class="badge off">${r.failed} başarısız</span>` : html`<span class="badge ok">hepsi geçti</span>`}</td></tr>`) : emptyRow(3, 'Henüz çalıştırılmadı.')}</tbody></table></div>`;
  return shell(ctx, { title: 'Testler', body });
}
