// Yonetim paneli: Blog yazilari, Haberler, Kategoriler, Sayfalar, Bannerlar, Menu, Mesajlar, Medya.
import { html, raw, esc, parseJson, toInt, truthy, nowIso, slugify, fmtDateTime, safeUrl } from './util.js';
import { sanitizeHtml } from './sanitize.js';
import { all, first, run, scalar, logActivity } from './env.js';
import { shell, fld, postForm, btnPost, pageLinks, paging, likeArg, ESC_LIKE, okRedirect, errRedirect, jsonOk, jsonErr, emptyRow, dt, notFoundPage, checkImage, randomName } from './admin_ui.js';
import { uniqueSlug, RESERVED_SLUGS, waNumber } from './admin_util.js';
import { importer } from './admin_import.js';

export async function handle(ctx, parts) {
  switch (parts[0]) {
    case 'yazilar': return posts(ctx, parts, 'blog');
    case 'haberler': return posts(ctx, parts, 'news');
    case 'kategoriler': return categories(ctx, parts);
    case 'sayfalar': return pages(ctx, parts);
    case 'bannerlar': return banners(ctx, parts);
    case 'menu': return menu(ctx, parts);
    case 'mesajlar': return messages(ctx, parts);
    case 'medya': return media(ctx, parts);
    default: return null;
  }
}
const editorField = (name, label, value, lang = 'tr', rows = 14) => html`<div class="fld"><label for="f_${name}">${label}</label><textarea id="f_${name}" name="${name}" data-editor="${lang}" rows="${rows}">${value || ''}</textarea></div>`;
const dtLocal = (iso) => String(iso || '').slice(0, 16);
const toIsoFromLocal = (v, fb) => (v && /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?/.test(v) ? new Date(v.length <= 16 ? v + (v.length === 10 ? 'T00:00' : '') + ':00+03:00' : v).toISOString() : fb);

// ------------------------------------------------------------------ Yazilar / Haberler
async function posts(ctx, parts, kind) {
  const env = ctx.env; const base = kind === 'news' ? '/admin/haberler' : '/admin/yazilar'; const label = kind === 'news' ? 'Haber' : 'Yazı';
  const cats = await all(env, 'SELECT id,name,slug FROM blog_categories WHERE kind=? ORDER BY sort_key,name', kind);
  const catIds = cats.map((c) => c.id);
  const sub = parts[1] || '';
  if (sub === 'ice-aktar') return importer(ctx, base, cats, kind);
  if (ctx.method === 'POST') {
    if (sub === 'kaydet') {
      const id = toInt(ctx.s('id'), 0);
      const title = ctx.s('title').slice(0, 200);
      if (!title) return errRedirect(ctx, id ? `${base}/${id}` : `${base}/yeni`, 'Başlık gerekli.');
      const old = id ? await first(env, 'SELECT * FROM posts WHERE id=?', id) : null;
      if (id && !old) return errRedirect(ctx, base, `${label} bulunamadı.`);
      const cat = toInt(ctx.s('category_id'), 0);
      if (cat && !(await first(env, 'SELECT id FROM blog_categories WHERE id=?', cat))) return errRedirect(ctx, base, 'Kategori bulunamadı.');
      let slugBase = ctx.s('slug') || title;
      let slug = await uniqueSlug(env, 'posts', slugBase, id, 'yazi');
      if (RESERVED_SLUGS.has(slug)) slug += '-yazi';
      const f = {
        slug, category_id: cat, title, title_en: ctx.s('title_en').slice(0, 200), title_ru: ctx.s('title_ru').slice(0, 200),
        excerpt: ctx.s('excerpt').slice(0, 500), body: sanitizeHtml(ctx.r('body')), body_en: sanitizeHtml(ctx.r('body_en')), body_ru: sanitizeHtml(ctx.r('body_ru')),
        image: ctx.s('image').slice(0, 300), status: ctx.s('status') === 'taslak' ? 'taslak' : 'yayinda', author: ctx.s('author').slice(0, 80) || ctx.admin.display_name,
        published_at: toIsoFromLocal(ctx.s('published_at'), old ? old.published_at || nowIso() : nowIso()), updated_at: nowIso(),
      };
      if (f.image && !safeUrl(f.image)) f.image = '';
      const keys = Object.keys(f);
      if (old) await run(env, `UPDATE posts SET ${keys.map((k) => `${k}=?`).join(',')} WHERE id=?`, ...keys.map((k) => f[k]), id);
      else { const r = await run(env, `INSERT INTO posts(${keys.join(',')},source) VALUES(${keys.map(() => '?').join(',')},'admin')`, ...keys.map((k) => f[k])); ctx._newId = r.meta && r.meta.last_row_id; }
      const nid = id || (await first(env, 'SELECT id FROM posts WHERE slug=?', slug)).id;
      await logActivity(env, ctx.who, `${label} ${old ? 'düzenlendi' : 'eklendi'}: ${title}`);
      return okRedirect(ctx, `${base}/${nid}`, `${label} kaydedildi.`);
    }
    if (sub === 'sil') { const id = toInt(ctx.s('id'), 0); const p = await first(env, 'SELECT title FROM posts WHERE id=?', id); if (p) { await run(env, 'DELETE FROM posts WHERE id=?', id); await logActivity(env, ctx.who, `${label} silindi: ${p.title}`); } return okRedirect(ctx, base, `${label} silindi.`); }
    if (sub === 'durum') { const id = toInt(ctx.s('id'), 0); const p = await first(env, 'SELECT status FROM posts WHERE id=?', id); if (p) await run(env, 'UPDATE posts SET status=?,updated_at=? WHERE id=?', p.status === 'yayinda' ? 'taslak' : 'yayinda', nowIso(), id); return okRedirect(ctx, base, 'Durum değişti.'); }
    return errRedirect(ctx, base, 'Geçersiz işlem.');
  }
  if (sub === 'yeni' || toInt(sub, 0)) {
    const id = sub === 'yeni' ? 0 : toInt(sub, 0);
    const p = id ? await first(env, 'SELECT * FROM posts WHERE id=?', id) : { title: '', title_en: '', title_ru: '', slug: '', category_id: catIds[0] || 0, excerpt: '', body: '', body_en: '', body_ru: '', image: '', status: 'yayinda', author: ctx.admin.display_name, published_at: nowIso() };
    if (!p) return notFoundPage(ctx, label);
    const catOpts = [[0, '— kategorisiz —'], ...cats.map((c) => [c.id, c.name])];
    const body = html`<p><a class="btn ghost" href="${base}">← ${label} listesi</a>${id && p.status === 'yayinda' ? html` <a class="btn ghost" target="_blank" rel="noopener" href="/yazi/${encodeURIComponent(p.slug)}">Sitede aç</a>` : ''}</p>
${postForm(ctx, `${base}/kaydet`, html`<input type="hidden" name="id" value="${id}"><div class="card form">
${fld({ name: 'title', label: 'Başlık (TR)', value: p.title, req: true })}<div class="grid two">${fld({ name: 'category_id', label: 'Kategori', type: 'select', options: catOpts, value: p.category_id })}${fld({ name: 'status', label: 'Durum', type: 'select', options: [['yayinda', 'Yayında'], ['taslak', 'Taslak']], value: p.status })}${fld({ name: 'slug', label: 'Bağlantı adı (boşsa başlıktan)', value: p.slug })}${fld({ name: 'published_at', label: 'Yayın tarihi', type: 'datetime-local', value: dtLocal(p.published_at) })}${fld({ name: 'author', label: 'Yazar', value: p.author })}</div>
${fld({ name: 'image', label: 'Öne çıkan görsel', type: 'image', value: p.image })}${fld({ name: 'excerpt', label: 'Özet (listelerde ve paylaşımda görünür)', type: 'textarea', rows: 3, value: p.excerpt })}</div>
<div class="card form"><h2>İçerik (Türkçe)</h2>${editorField('body', 'Metin', p.body, 'tr', 18)}</div>
<div class="card form"><h2>English</h2>${fld({ name: 'title_en', label: 'Title', value: p.title_en })}${editorField('body_en', 'Body', p.body_en, 'en', 10)}</div>
<div class="card form"><h2>Русский</h2>${fld({ name: 'title_ru', label: 'Заголовок', value: p.title_ru })}${editorField('body_ru', 'Текст', p.body_ru, 'ru', 10)}</div>
<div class="sticky-actions"><button class="btn primary" type="submit">Kaydet</button></div>`, { id: 'postform' })}
${id ? html`<div class="card">${btnPost(ctx, `${base}/sil`, `${label} sil`, { cls: 'btn danger', confirm: `Bu ${label.toLowerCase()} silinsin mi?`, fields: { id } })}</div>` : ''}`;
    return shell(ctx, { title: id ? `${label} düzenle` : `Yeni ${label.toLowerCase()}`, active: base, body });
  }
  const u = ctx.url.searchParams; const q = (u.get('q') || '').trim(); const st = u.get('durum') || ''; const cat = toInt(u.get('kategori'), 0);
  const w = [`(category_id IN (${catIds.length ? catIds.map(() => '?').join(',') : 'NULL'})${kind === 'blog' ? ' OR category_id=0' : ''})`]; const a = [...catIds];
  if (q) { w.push(`title LIKE ? ${ESC_LIKE}`); a.push(likeArg(q)); }
  if (st) { w.push('status=?'); a.push(st); }
  if (cat) { w.push('category_id=?'); a.push(cat); }
  const total = Number(await scalar(env, `SELECT COUNT(*) FROM posts WHERE ${w.join(' AND ')}`, ...a));
  const pg = paging(ctx.url, total, 25);
  const rows = await all(env, `SELECT id,slug,title,category_id,status,source,published_at FROM posts WHERE ${w.join(' AND ')} ORDER BY published_at DESC, id DESC LIMIT ? OFFSET ?`, ...a, pg.per, pg.offset);
  const cn = Object.fromEntries(cats.map((c) => [c.id, c.name]));
  const body = html`<form class="row filters" method="get"><input type="search" name="q" value="${q}" placeholder="Başlıkta ara"><select name="kategori"><option value="">Kategori: hepsi</option>${cats.map((c) => html`<option value="${c.id}"${cat === c.id ? raw(' selected') : ''}>${c.name}</option>`)}</select><select name="durum"><option value="">Durum: hepsi</option><option value="yayinda"${st === 'yayinda' ? raw(' selected') : ''}>Yayında</option><option value="taslak"${st === 'taslak' ? raw(' selected') : ''}>Taslak</option></select><button class="btn" type="submit">Süz</button> <a class="btn primary" href="${base}/yeni">+ Yeni ${label.toLowerCase()}</a> <a class="btn" href="${base}/ice-aktar">Siteden içe aktar</a></form>
<div class="card"><div class="tablewrap"><table class="tbl"><thead><tr><th>Başlık</th><th>Kategori</th><th>Durum</th><th>Tarih</th><th></th></tr></thead><tbody>${rows.length ? rows.map((p) => html`<tr><td><a href="${base}/${p.id}"><strong>${p.title}</strong></a>${p.source !== 'admin' ? html` <span class="badge">${p.source}</span>` : ''}</td><td>${cn[p.category_id] || '—'}</td><td>${p.status === 'yayinda' ? html`<span class="badge ok">yayında</span>` : html`<span class="badge off">taslak</span>`}</td><td class="muted nowrap">${dt(p.published_at)}</td><td class="actions"><a class="btn sm" href="${base}/${p.id}">Düzenle</a> ${btnPost(ctx, `${base}/durum`, p.status === 'yayinda' ? 'Taslağa al' : 'Yayınla', { fields: { id: p.id } })}</td></tr>`) : emptyRow(5, 'Kayıt yok.')}</tbody></table></div>${pageLinks(ctx.url, pg.page, pg.pages)}</div>`;
  return shell(ctx, { title: kind === 'news' ? 'Haberler' : 'Blog yazıları', active: base, body });
}

// ------------------------------------------------------------------ Kategoriler
async function categories(ctx, parts) {
  const env = ctx.env; const act = parts[1] || '';
  if (ctx.method === 'POST') {
    const name = ctx.s('name').slice(0, 80); const kind = ctx.s('kind') === 'news' ? 'news' : 'blog';
    if (act === 'ekle') {
      if (!name) return errRedirect(ctx, '/admin/kategoriler', 'Ad gerekli.');
      const slug = await uniqueSlug(env, 'blog_categories', ctx.s('slug') || name, 0, 'kategori');
      await run(env, 'INSERT INTO blog_categories(slug,name,name_en,name_ru,kind,sort_key) VALUES(?,?,?,?,?,?)', slug, name, ctx.s('name_en').slice(0, 80), ctx.s('name_ru').slice(0, 80), kind, toInt(ctx.s('sort_key'), 0));
      await logActivity(env, ctx.who, `Kategori eklendi: ${name}`);
      return okRedirect(ctx, '/admin/kategoriler', 'Kategori eklendi.');
    }
    const id = toInt(parts[2], 0); const c = id ? await first(env, 'SELECT * FROM blog_categories WHERE id=?', id) : null;
    if (!c) return errRedirect(ctx, '/admin/kategoriler', 'Kategori bulunamadı.');
    if (act === 'kaydet') {
      if (!name) return errRedirect(ctx, '/admin/kategoriler', 'Ad gerekli.');
      const slug = await uniqueSlug(env, 'blog_categories', ctx.s('slug') || name, id, 'kategori');
      await run(env, 'UPDATE blog_categories SET slug=?,name=?,name_en=?,name_ru=?,kind=?,sort_key=? WHERE id=?', slug, name, ctx.s('name_en').slice(0, 80), ctx.s('name_ru').slice(0, 80), kind, toInt(ctx.s('sort_key'), 0), id);
      return okRedirect(ctx, '/admin/kategoriler', 'Kategori güncellendi.');
    }
    if (act === 'sil') {
      const n = Number(await scalar(env, 'SELECT COUNT(*) FROM posts WHERE category_id=?', id));
      if (n && ctx.s('onay') !== '1') return errRedirect(ctx, '/admin/kategoriler', `Bu kategoride ${n} yazı var; silerseniz yazılar kategorisiz kalır. Yine de silmek için tekrar “Sil”e basıp onaylayın.`);
      await run(env, 'UPDATE posts SET category_id=0 WHERE category_id=?', id); await run(env, 'DELETE FROM blog_categories WHERE id=?', id);
      await logActivity(env, ctx.who, `Kategori silindi: ${c.name}`);
      return okRedirect(ctx, '/admin/kategoriler', 'Kategori silindi.');
    }
    return errRedirect(ctx, '/admin/kategoriler', 'Geçersiz işlem.');
  }
  const rows = await all(env, 'SELECT c.*, (SELECT COUNT(*) FROM posts p WHERE p.category_id=c.id) AS n FROM blog_categories c ORDER BY kind,sort_key,name');
  const kindSel = (v) => html`<select name="kind"><option value="blog"${v === 'blog' ? raw(' selected') : ''}>Blog</option><option value="news"${v === 'news' ? raw(' selected') : ''}>Haber</option></select>`;
  const body = html`<div class="card"><p class="muted">“Haber” türündeki kategoriler sitede kenar çubuğunda ayrı başlık olarak, “Blog” türündekiler Blog sayfasında listelenir.</p>${rows.map((c) => html`<div class="rowform"><form method="post" action="/admin/kategoriler/kaydet/${c.id}" class="tagrow"><input type="hidden" name="_csrf" value="${ctx.csrf}"><input name="name" value="${c.name}" aria-label="Ad"><input name="name_en" value="${c.name_en}" placeholder="EN" aria-label="EN"><input name="name_ru" value="${c.name_ru}" placeholder="RU" aria-label="RU"><input name="slug" value="${c.slug}" aria-label="Bağlantı adı">${kindSel(c.kind)}<input type="number" name="sort_key" value="${c.sort_key}" style="width:5em" aria-label="Sıra"><span class="muted">${c.n} yazı</span><button class="btn sm primary" type="submit">Kaydet</button></form>${btnPost(ctx, `/admin/kategoriler/sil/${c.id}`, 'Sil', { cls: 'btn sm danger', confirm: `“${c.name}” kategorisi silinsin mi?`, fields: { onay: 1 } })}</div>`)}</div>
<div class="card"><h2>Yeni kategori</h2>${postForm(ctx, '/admin/kategoriler/ekle', html`<div class="grid two">${fld({ name: 'name', label: 'Ad (TR)', req: true })}${fld({ name: 'name_en', label: 'Name (EN)' })}${fld({ name: 'name_ru', label: 'Название (RU)' })}${fld({ name: 'kind', label: 'Tür', type: 'select', options: [['blog', 'Blog'], ['news', 'Haber']] })}</div><button class="btn primary" type="submit">Ekle</button>`)}</div>`;
  return shell(ctx, { title: 'Blog kategorileri', body });
}

// ------------------------------------------------------------------ Sayfalar
async function pages(ctx, parts) {
  const env = ctx.env; const sub = parts[1] || '';
  if (ctx.method === 'POST') {
    if (sub === 'kaydet') {
      const id = toInt(ctx.s('id'), 0); const title = ctx.s('title').slice(0, 200);
      if (!title) return errRedirect(ctx, '/admin/sayfalar', 'Başlık gerekli.');
      let slug = await uniqueSlug(env, 'pages', ctx.s('slug') || title, id, 'sayfa');
      if (RESERVED_SLUGS.has(slug)) slug += '-sayfa';
      const f = { slug, title, title_en: ctx.s('title_en').slice(0, 200), title_ru: ctx.s('title_ru').slice(0, 200), body: sanitizeHtml(ctx.r('body')), body_en: sanitizeHtml(ctx.r('body_en')), body_ru: sanitizeHtml(ctx.r('body_ru')), visible: ctx.s('visible') === '1' ? 1 : 0, updated_at: nowIso() };
      const keys = Object.keys(f);
      if (id) await run(env, `UPDATE pages SET ${keys.map((k) => `${k}=?`).join(',')} WHERE id=?`, ...keys.map((k) => f[k]), id);
      else await run(env, `INSERT INTO pages(${keys.join(',')}) VALUES(${keys.map(() => '?').join(',')})`, ...keys.map((k) => f[k]));
      const nid = id || (await first(env, 'SELECT id FROM pages WHERE slug=?', slug)).id;
      await logActivity(env, ctx.who, `Sayfa kaydedildi: ${title}`);
      return okRedirect(ctx, `/admin/sayfalar/${nid}`, 'Sayfa kaydedildi.');
    }
    if (sub === 'sil') { const id = toInt(ctx.s('id'), 0); await run(env, 'DELETE FROM pages WHERE id=?', id); await logActivity(env, ctx.who, `Sayfa silindi #${id}`); return okRedirect(ctx, '/admin/sayfalar', 'Sayfa silindi.'); }
    if (sub === 'gorunur') { const id = toInt(ctx.s('id'), 0); await run(env, 'UPDATE pages SET visible=1-visible WHERE id=?', id); return okRedirect(ctx, '/admin/sayfalar', 'Görünürlük değişti.'); }
    return errRedirect(ctx, '/admin/sayfalar', 'Geçersiz işlem.');
  }
  if (sub === 'yeni' || toInt(sub, 0)) {
    const id = sub === 'yeni' ? 0 : toInt(sub, 0);
    const p = id ? await first(env, 'SELECT * FROM pages WHERE id=?', id) : { title: '', title_en: '', title_ru: '', slug: '', body: '', body_en: '', body_ru: '', visible: 1 };
    if (!p) return notFoundPage(ctx, 'Sayfa');
    const body = html`<p><a class="btn ghost" href="/admin/sayfalar">← Sayfalar</a>${id ? html` <a class="btn ghost" target="_blank" rel="noopener" href="/${encodeURIComponent(p.slug)}">Sitede aç</a>` : ''}</p>${postForm(ctx, '/admin/sayfalar/kaydet', html`<input type="hidden" name="id" value="${id}"><div class="card form">${fld({ name: 'title', label: 'Başlık (TR)', value: p.title, req: true })}<div class="grid two">${fld({ name: 'slug', label: 'Bağlantı adı', value: p.slug })}${fld({ name: 'visible', label: 'Sitede görünür', type: 'bool', value: p.visible })}</div>${editorField('body', 'İçerik (TR)', p.body, 'tr', 14)}</div><div class="card form"><h2>English</h2>${fld({ name: 'title_en', label: 'Title', value: p.title_en })}${editorField('body_en', 'Body', p.body_en, 'en', 8)}</div><div class="card form"><h2>Русский</h2>${fld({ name: 'title_ru', label: 'Заголовок', value: p.title_ru })}${editorField('body_ru', 'Текст', p.body_ru, 'ru', 8)}</div><div class="sticky-actions"><button class="btn primary" type="submit">Kaydet</button></div>`, { id: 'pageform' })}`;
    return shell(ctx, { title: id ? 'Sayfa düzenle' : 'Yeni sayfa', active: '/admin/sayfalar', body });
  }
  const rows = await all(env, 'SELECT id,slug,title,visible,updated_at FROM pages ORDER BY title');
  const body = html`<p><a class="btn primary" href="/admin/sayfalar/yeni">+ Yeni sayfa</a></p><div class="card"><div class="tablewrap"><table class="tbl"><thead><tr><th>Başlık</th><th>Adres</th><th>Site</th><th></th></tr></thead><tbody>${rows.length ? rows.map((p) => html`<tr><td><a href="/admin/sayfalar/${p.id}"><strong>${p.title}</strong></a></td><td class="muted">/${p.slug}</td><td>${p.visible ? html`<span class="badge ok">görünür</span>` : html`<span class="badge off">gizli</span>`}</td><td class="actions"><a class="btn sm" href="/admin/sayfalar/${p.id}">Düzenle</a> ${btnPost(ctx, '/admin/sayfalar/gorunur', p.visible ? 'Gizle' : 'Göster', { fields: { id: p.id } })}${btnPost(ctx, '/admin/sayfalar/sil', 'Sil', { cls: 'btn sm danger', confirm: 'Sayfa silinsin mi?', fields: { id: p.id } })}</td></tr>`) : emptyRow(4)}</tbody></table></div></div>`;
  return shell(ctx, { title: 'Sayfalar', body });
}

// ------------------------------------------------------------------ Bannerlar
async function banners(ctx, parts) {
  const env = ctx.env; const act = parts[1] || '';
  if (ctx.method === 'POST') {
    const vals = () => [ctx.s('title').slice(0, 120), ctx.s('subtitle').slice(0, 240), safeUrl(ctx.s('image')), safeUrl(ctx.s('link')), ctx.s('button').slice(0, 40), toInt(ctx.s('sort_key'), 0), ctx.s('visible') === '0' ? 0 : 1];
    if (act === 'ekle') { await run(env, 'INSERT INTO banners(title,subtitle,image,link,button,sort_key,visible) VALUES(?,?,?,?,?,?,?)', ...vals()); await logActivity(env, ctx.who, 'Banner eklendi'); return okRedirect(ctx, '/admin/bannerlar', 'Banner eklendi.'); }
    const id = toInt(parts[2], 0); const b = id ? await first(env, 'SELECT * FROM banners WHERE id=?', id) : null;
    if (!b) return errRedirect(ctx, '/admin/bannerlar', 'Banner bulunamadı.');
    if (act === 'kaydet') { await run(env, 'UPDATE banners SET title=?,subtitle=?,image=?,link=?,button=?,sort_key=?,visible=? WHERE id=?', ...vals(), id); return okRedirect(ctx, '/admin/bannerlar', 'Banner güncellendi.'); }
    if (act === 'gorunur') { await run(env, 'UPDATE banners SET visible=1-visible WHERE id=?', id); return okRedirect(ctx, '/admin/bannerlar', 'Görünürlük değişti.'); }
    if (act === 'sil') { await run(env, 'DELETE FROM banners WHERE id=?', id); return okRedirect(ctx, '/admin/bannerlar', 'Banner silindi.'); }
    return errRedirect(ctx, '/admin/bannerlar', 'Geçersiz işlem.');
  }
  const rows = await all(env, 'SELECT * FROM banners ORDER BY sort_key,id');
  const form = (b, action, btn) => postForm(ctx, action, html`<div class="grid two">${fld({ name: 'title', label: 'Başlık', value: b.title })}${fld({ name: 'subtitle', label: 'Alt başlık', value: b.subtitle })}${fld({ name: 'image', label: 'Görsel', type: 'image', value: b.image })}${fld({ name: 'link', label: 'Bağlantı', value: b.link, ph: '/satilik-villa' })}${fld({ name: 'button', label: 'Düğme yazısı', value: b.button })}${fld({ name: 'sort_key', label: 'Sıra', type: 'number', value: b.sort_key })}${fld({ name: 'visible', label: 'Görünür', type: 'select', options: [['1', 'Evet'], ['0', 'Hayır']], value: String(b.visible) })}</div><button class="btn primary" type="submit">${btn}</button>`);
  const body = html`<p class="muted">Ana sayfanın üstündeki kayan bannerlar. Hiç banner yoksa varsayılan marka bandı gösterilir.</p>${rows.map((b) => html`<div class="card"><h2>${b.title || '(başlıksız)'} ${b.visible ? '' : html`<span class="badge off">gizli</span>`}</h2>${b.image ? html`<img class="pv" src="${b.image}" alt="">` : ''}${form(b, `/admin/bannerlar/kaydet/${b.id}`, 'Kaydet')}<p class="row">${btnPost(ctx, `/admin/bannerlar/gorunur/${b.id}`, b.visible ? 'Gizle' : 'Göster')}${btnPost(ctx, `/admin/bannerlar/sil/${b.id}`, 'Sil', { cls: 'btn sm danger', confirm: 'Banner silinsin mi?' })}</p></div>`)}
<div class="card"><h2>Yeni banner</h2>${form({ title: '', subtitle: '', image: '', link: '', button: '', sort_key: rows.length, visible: 1 }, '/admin/bannerlar/ekle', 'Ekle')}</div>`;
  return shell(ctx, { title: 'Bannerlar', body });
}

// ------------------------------------------------------------------ Menu
const KINDS = [['home', 'Ana sayfa'], ['listings', 'İlan listesi (filtre)'], ['page', 'Sayfa'], ['blogcat', 'Blog kategorisi'], ['blog', 'Blog'], ['link', 'Bağlantı']];
async function menu(ctx, parts) {
  const env = ctx.env; const act = parts[1] || '';
  if (ctx.method === 'POST') {
    const grab = () => {
      const kind = KINDS.some(([k]) => k === ctx.s('kind')) ? ctx.s('kind') : 'link';
      let target = ctx.s('target').slice(0, 200); if (kind === 'link') target = safeUrl(target);
      return { label: ctx.s('label').slice(0, 60), label_en: ctx.s('label_en').slice(0, 60), label_ru: ctx.s('label_ru').slice(0, 60), kind, target, slug: slugify(ctx.s('slug') || ctx.s('label') || target.replace('|', '-')), parent_id: toInt(ctx.s('parent_id'), 0), sort_key: toInt(ctx.s('sort_key'), 0), visible: ctx.s('visible') === '0' ? 0 : 1 };
    };
    if (act === 'ekle') {
      const m = grab(); if (!m.label) return errRedirect(ctx, '/admin/menu', 'Menü adı gerekli.');
      const mx = Number(await scalar(env, 'SELECT COALESCE(MAX(sort_key),0) FROM menu_items WHERE parent_id=?', m.parent_id)) + 1; if (!ctx.s('sort_key')) m.sort_key = mx;
      await run(env, "INSERT INTO menu_items(menu,parent_id,label,label_en,label_ru,kind,target,slug,sort_key,visible) VALUES('main',?,?,?,?,?,?,?,?,?)", m.parent_id, m.label, m.label_en, m.label_ru, m.kind, m.target, m.slug, m.sort_key, m.visible);
      await logActivity(env, ctx.who, `Menü öğesi eklendi: ${m.label}`); return okRedirect(ctx, '/admin/menu', 'Menü öğesi eklendi.');
    }
    const id = toInt(parts[2], 0); const it = id ? await first(env, 'SELECT * FROM menu_items WHERE id=?', id) : null;
    if (!it) return errRedirect(ctx, '/admin/menu', 'Menü öğesi bulunamadı.');
    if (act === 'kaydet') {
      const m = grab(); if (!m.label) return errRedirect(ctx, '/admin/menu', 'Menü adı gerekli.');
      if (m.parent_id === id) m.parent_id = 0;
      await run(env, 'UPDATE menu_items SET parent_id=?,label=?,label_en=?,label_ru=?,kind=?,target=?,slug=?,sort_key=?,visible=? WHERE id=?', m.parent_id, m.label, m.label_en, m.label_ru, m.kind, m.target, m.slug, m.sort_key, m.visible, id);
      return okRedirect(ctx, '/admin/menu', 'Menü öğesi güncellendi.');
    }
    if (act === 'gizle') { await run(env, 'UPDATE menu_items SET visible=1-visible WHERE id=?', id); return okRedirect(ctx, '/admin/menu', 'Görünürlük değişti.'); }
    if (act === 'sil') { await run(env, 'UPDATE menu_items SET parent_id=? WHERE parent_id=?', it.parent_id, id); await run(env, 'DELETE FROM menu_items WHERE id=?', id); await logActivity(env, ctx.who, `Menü öğesi silindi: ${it.label}`); return okRedirect(ctx, '/admin/menu', 'Silindi.'); }
    if (act === 'yukari' || act === 'asagi') {
      const sibs = await all(env, 'SELECT id FROM menu_items WHERE menu=? AND parent_id=? ORDER BY sort_key,id', it.menu, it.parent_id);
      const ids = sibs.map((s) => s.id); const i = ids.indexOf(id); const j = act === 'yukari' ? i - 1 : i + 1;
      if (j >= 0 && j < ids.length) { [ids[i], ids[j]] = [ids[j], ids[i]]; for (let k = 0; k < ids.length; k++) await run(env, 'UPDATE menu_items SET sort_key=? WHERE id=?', k + 1, ids[k]); }
      return okRedirect(ctx, '/admin/menu', 'Sıra değişti.');
    }
    return errRedirect(ctx, '/admin/menu', 'Geçersiz işlem.');
  }
  const items = await all(env, "SELECT * FROM menu_items WHERE menu='main' ORDER BY parent_id,sort_key,id");
  const combos = new Set(['Satılık|Villa', 'Satılık|Daire', 'Satılık|Arazi', 'Kiralık|Villa', 'Kiralık|Daire', '*|*']);
  for (const r of await all(env, "SELECT DISTINCT listing_type t, category c FROM listings WHERE category<>''")) combos.add(`${r.t}|${r.c}`);
  const pagesL = await all(env, 'SELECT slug,title FROM pages ORDER BY title'); const cats = await all(env, 'SELECT slug,name FROM blog_categories ORDER BY name');
  const tops = items.filter((i) => !i.parent_id);
  const tree = []; for (const t of tops) { tree.push([t, 0]); for (const c of items.filter((x) => x.parent_id === t.id)) tree.push([c, 1]); }
  const known = new Set(tree.map(([i]) => i.id)); for (const i of items) if (!known.has(i.id)) tree.push([i, 1]);
  const targetField = (it) => html`<input name="target" value="${it.target}" list="tgt" placeholder="hedef" aria-label="Hedef">`;
  const row = (it, depth) => html`<div class="rowform ${depth ? 'child' : ''}"><form method="post" action="/admin/menu/kaydet/${it.id}" class="menurow"><input type="hidden" name="_csrf" value="${ctx.csrf}"><input name="label" value="${it.label}" aria-label="Ad (TR)"><input name="label_en" value="${it.label_en}" placeholder="EN" aria-label="EN"><input name="label_ru" value="${it.label_ru}" placeholder="RU" aria-label="RU"><select name="kind">${KINDS.map(([k, l]) => html`<option value="${k}"${k === it.kind ? raw(' selected') : ''}>${l}</option>`)}</select>${targetField(it)}<input name="slug" value="${it.slug}" placeholder="adres" aria-label="Adres"><select name="parent_id"><option value="0">Üst düzey</option>${tops.filter((t) => t.id !== it.id).map((t) => html`<option value="${t.id}"${t.id === it.parent_id ? raw(' selected') : ''}>↳ ${t.label}</option>`)}</select><input type="number" name="sort_key" value="${it.sort_key}" style="width:4.5em" aria-label="Sıra"><select name="visible"><option value="1"${it.visible ? raw(' selected') : ''}>Görünür</option><option value="0"${it.visible ? '' : raw(' selected')}>Gizli</option></select><button class="btn sm primary" type="submit">Kaydet</button></form><span class="row">${btnPost(ctx, `/admin/menu/yukari/${it.id}`, '↑')}${btnPost(ctx, `/admin/menu/asagi/${it.id}`, '↓')}${btnPost(ctx, `/admin/menu/sil/${it.id}`, 'Sil', { cls: 'btn sm danger', confirm: `“${it.label}” menüden silinsin mi?` })}</span></div>`;
  const body = html`<div class="card"><p class="muted">Üst menü bu listedir. <b>İlan listesi</b> hedefi “Tür|Kategori” biçimindedir (örn. Satılık|Villa); <b>*|*</b> menüdeki diğer ilan filtrelerine uymayan ilanları gösterir (DİĞER İLANLAR). Sayfa/blog kategorisi için slug seçin; bağlantı için tam adres yazın.</p>${tree.map(([it, d]) => row(it, d))}</div>
<div class="card"><h2>Yeni menü öğesi</h2>${postForm(ctx, '/admin/menu/ekle', html`<div class="grid two">${fld({ name: 'label', label: 'Ad (TR)', req: true })}${fld({ name: 'label_en', label: 'Name (EN)' })}${fld({ name: 'label_ru', label: 'Название (RU)' })}${fld({ name: 'kind', label: 'Tür', type: 'select', options: KINDS })}${fld({ name: 'target', label: 'Hedef', attrs: { list: 'tgt' }, help: 'İlan: Satılık|Villa • Sayfa/Blog kategorisi: slug • Bağlantı: adres' })}${fld({ name: 'slug', label: 'Site adresi (boşsa addan)' })}${fld({ name: 'parent_id', label: 'Üst öğe', type: 'select', options: [[0, 'Üst düzey'], ...tops.map((t) => [t.id, t.label])] })}</div><button class="btn primary" type="submit">Ekle</button>`)}</div>
<datalist id="tgt">${[...combos].map((c) => html`<option value="${c}">`)}${pagesL.map((p) => html`<option value="${p.slug}">${p.title}</option>`)}${cats.map((c) => html`<option value="${c.slug}">${c.name}</option>`)}</datalist>`;
  return shell(ctx, { title: 'Menü', body });
}

// ------------------------------------------------------------------ Mesajlar
async function messages(ctx, parts) {
  const env = ctx.env; const sub = parts[1] || '';
  if (ctx.method === 'POST') {
    const id = toInt(ctx.s('id') || sub, 0);
    if (parts[1] === 'oku') { await run(env, 'UPDATE messages SET is_read=1 WHERE id=?', id); return okRedirect(ctx, '/admin/mesajlar', 'Okundu.'); }
    if (parts[1] === 'okunmadi') { await run(env, 'UPDATE messages SET is_read=0 WHERE id=?', id); return okRedirect(ctx, '/admin/mesajlar', 'Okunmadı işaretlendi.'); }
    if (parts[1] === 'hepsi') { await run(env, 'UPDATE messages SET is_read=1'); return okRedirect(ctx, '/admin/mesajlar', 'Hepsi okundu.'); }
    if (parts[1] === 'sil') { await run(env, 'DELETE FROM messages WHERE id=?', id); await logActivity(env, ctx.who, `Mesaj silindi #${id}`); return okRedirect(ctx, '/admin/mesajlar', 'Mesaj silindi.'); }
    return errRedirect(ctx, '/admin/mesajlar', 'Geçersiz işlem.');
  }
  if (toInt(sub, 0)) {
    const m = await first(env, 'SELECT * FROM messages WHERE id=?', toInt(sub, 0)); if (!m) return notFoundPage(ctx, 'Mesaj');
    await run(env, 'UPDATE messages SET is_read=1 WHERE id=?', m.id);
    const l = m.listing_id ? await first(env, 'SELECT id,title,slug FROM listings WHERE id=?', m.listing_id) : null;
    const wa = waNumber(m.phone);
    const body = html`<p><a class="btn ghost" href="/admin/mesajlar">← Gelen kutusu</a></p><div class="card"><h2>${m.name || '(adsız)'}</h2><p class="muted">${dt(m.created_at)} · IP ${m.ip}</p><p>${m.phone ? html`Telefon: <a href="tel:${m.phone}">${m.phone}</a><br>` : ''}${m.email ? html`E-posta: <a href="mailto:${m.email}">${m.email}</a><br>` : ''}${l ? html`İlan: <a href="/admin/ilanlar/${l.id}">${l.title}</a>` : ''}</p><blockquote class="msg">${m.body}</blockquote><p class="row">${m.email ? html`<a class="btn primary" href="mailto:${m.email}?subject=${encodeURIComponent('Mesajınız hk.')}">E-postayla yanıtla</a>` : ''}${wa ? html`<a class="btn" target="_blank" rel="noopener" href="https://wa.me/${wa}">WhatsApp'tan yaz</a>` : ''}${btnPost(ctx, '/admin/mesajlar/okunmadi', 'Okunmadı yap', { fields: { id: m.id } })}${btnPost(ctx, '/admin/mesajlar/sil', 'Sil', { cls: 'btn danger', confirm: 'Mesaj silinsin mi?', fields: { id: m.id } })}</p></div>`;
    return shell(ctx, { title: 'Mesaj', active: '/admin/mesajlar', body });
  }
  const only = ctx.url.searchParams.get('durum') === 'okunmamis';
  const total = Number(await scalar(env, `SELECT COUNT(*) FROM messages ${only ? 'WHERE is_read=0' : ''}`)); const pg = paging(ctx.url, total, 30);
  const rows = await all(env, `SELECT id,name,phone,email,body,created_at,is_read FROM messages ${only ? 'WHERE is_read=0' : ''} ORDER BY id DESC LIMIT ? OFFSET ?`, pg.per, pg.offset);
  const body = html`<p class="row"><a class="btn ${only ? '' : 'primary'}" href="/admin/mesajlar">Hepsi</a><a class="btn ${only ? 'primary' : ''}" href="/admin/mesajlar?durum=okunmamis">Okunmamış</a>${btnPost(ctx, '/admin/mesajlar/hepsi', 'Hepsini okundu yap', { cls: 'btn' })}</p><div class="card"><div class="tablewrap"><table class="tbl"><tbody>${rows.length ? rows.map((m) => html`<tr class="${m.is_read ? '' : 'unread'}"><td>${m.is_read ? '' : html`<span class="badge">yeni</span> `}<a href="/admin/mesajlar/${m.id}"><strong>${m.name || '(adsız)'}</strong></a> <span class="muted">${m.phone} ${m.email}</span><div class="muted">${String(m.body).slice(0, 140)}</div></td><td class="muted nowrap">${dt(m.created_at)}</td><td class="actions">${btnPost(ctx, '/admin/mesajlar/sil', 'Sil', { cls: 'btn sm danger', confirm: 'Mesaj silinsin mi?', fields: { id: m.id } })}</td></tr>`) : emptyRow(3, 'Mesaj yok.')}</tbody></table></div>${pageLinks(ctx.url, pg.page, pg.pages)}</div>`;
  return shell(ctx, { title: 'Mesajlar', body });
}

// ------------------------------------------------------------------ Medya
async function media(ctx, parts) {
  const env = ctx.env; const sub = parts[1] || '';
  if (sub === 'yukle' && ctx.method === 'POST') {
    const f = ctx.file('file') || ctx.file('files');
    const c = await checkImage(f, { allowGif: true });
    if (!c.ok) return jsonErr(c.error, 400);
    const name = `${randomName()}.${c.kind.ext}`;
    await env.MEDIA.put(`m/${name}`, c.bytes, { httpMetadata: { contentType: c.kind.type } });
    await logActivity(env, ctx.who, `Medya yüklendi: ${name}`);
    return jsonOk({ url: `/media/${name}`, name });
  }
  if (sub === 'sil' && ctx.method === 'POST') {
    const name = ctx.s('name'); if (!/^[A-Za-z0-9._-]{1,120}$/.test(name)) return errRedirect(ctx, '/admin/medya', 'Geçersiz dosya adı.');
    await env.MEDIA.delete(`m/${name}`); await logActivity(env, ctx.who, `Medya silindi: ${name}`);
    return okRedirect(ctx, '/admin/medya', 'Dosya silindi.');
  }
  if (ctx.method === 'POST') return errRedirect(ctx, '/admin/medya', 'Geçersiz işlem.');
  const cursor = ctx.url.searchParams.get('c') || undefined;
  const r = await env.MEDIA.list({ prefix: 'm/', cursor, limit: 48 });
  const items = r.objects.map((o) => ({ name: o.key.slice(2), size: o.size }));
  const body = html`<div class="card"><h2>Görsel yükle</h2><p class="row"><input type="file" id="mediafile" accept="image/jpeg,image/png,image/webp,image/gif" multiple> <button class="btn primary" type="button" id="mediaup">Yükle</button> <span id="mediastatus" class="muted"></span></p></div>
<div class="card"><div class="mediagrid">${items.length ? items.map((m) => html`<figure><img src="/media/${m.name}" alt="" loading="lazy"><figcaption><code>/media/${m.name}</code><div class="row"><button type="button" class="btn sm" data-copy="/media/${m.name}">Adresi kopyala</button>${btnPost(ctx, '/admin/medya/sil', 'Sil', { cls: 'btn sm danger', confirm: 'Dosya silinsin mi? (Kullanıldığı yerlerde görsel kırılır.)', fields: { name: m.name } })}</div></figcaption></figure>`) : html`<p class="muted">Henüz yüklenmiş görsel yok.</p>`}</div>${r.truncated ? html`<p><a class="btn" href="/admin/medya?c=${encodeURIComponent(r.cursor)}">Sonraki ›</a></p>` : ''}</div>`;
  return shell(ctx, { title: 'Medya kitaplığı', body });
}
