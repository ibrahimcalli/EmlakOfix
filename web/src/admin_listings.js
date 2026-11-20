// Yonetim paneli: Ilanlar (liste, duzenleme, fotograf galerisi, gorunurluk) ve Etiketler.
import { html, raw, esc, parseJson, toNum, toInt, truthy, nowIso, slugify, fmtPrice } from './util.js';
import { all, first, run, scalar, logActivity } from './env.js';
import { shell, fld, postForm, btnPost, pageLinks, paging, likeArg, ESC_LIKE, okRedirect, errRedirect, emptyRow, dt, notFoundPage, checkImage } from './admin_ui.js';

const TEXT_FIELDS = ['title', 'title_en', 'title_ru', 'listing_type', 'category', 'district', 'neighborhood', 'address', 'currency', 'price_note', 'rooms', 'bathrooms', 'building_age', 'floor', 'total_floors', 'heating', 'deed_status', 'desc_tr', 'desc_en', 'desc_ru', 'agent_name', 'agent_phone', 'prog_status'];
const NUM_FIELDS = ['lat', 'lon', 'price', 'gross_m2', 'net_m2'];
const BOOL_FIELDS = ['hide_exact_location', 'furnished', 'swap_ok', 'credit_ok'];
const ADMIN_ONLY = ['visible', 'featured', 'sort_key'];
const STATUSES = ['Yayında', 'Kiralandı', 'Satıldı', 'Pasif', 'Taslak'];
const MAXLEN = { title: 200, title_en: 200, title_ru: 200, desc_tr: 20000, desc_en: 20000, desc_ru: 20000, address: 300 };

export async function handle(ctx, parts) {
  const sec = parts[0];
  if (sec === 'ilanlar') return parts[1] ? await listingAction(ctx, parts) : await listingList(ctx);
  if (sec === 'etiketler') return await tagsSection(ctx, parts);
  return null;
}

const jarr = (s) => { const a = parseJson(s, []); return Array.isArray(a) ? a : []; };
export function photoNums(l) {
  const o = jarr(l.photo_order).map((x) => toInt(x, -1)).filter((x) => x >= 0);
  return o.length ? o : Array.from({ length: Math.max(0, toInt(l.photo_count, 0)) }, (_, i) => i);
}

// ------------------------------------------------------------------ liste
async function listingList(ctx) {
  const u = ctx.url.searchParams;
  const q = (u.get('q') || '').trim(), tur = u.get('tur') || '', kat = u.get('kategori') || '', durum = u.get('durum') || '', gor = u.get('gorunur') || '', et = u.get('etiket') || '';
  const w = []; const a = [];
  if (q) { w.push(`(title LIKE ? ${ESC_LIKE} OR district LIKE ? ${ESC_LIKE} OR neighborhood LIKE ? ${ESC_LIKE} OR CAST(id AS TEXT)=?)`); a.push(likeArg(q), likeArg(q), likeArg(q), q); }
  if (tur) { w.push('listing_type=?'); a.push(tur); }
  if (kat) { w.push('category=?'); a.push(kat); }
  if (durum) { w.push('prog_status=?'); a.push(durum); }
  if (gor === '0' || gor === '1') { w.push('visible=?'); a.push(Number(gor)); }
  if (et) { w.push(`tags LIKE ? ${ESC_LIKE}`); a.push(likeArg(JSON.stringify(et).slice(1, -1))); }
  const where = w.length ? 'WHERE ' + w.join(' AND ') : '';
  const total = Number(await scalar(ctx.env, `SELECT COUNT(*) FROM listings ${where}`, ...a));
  const pg = paging(ctx.url, total, 25);
  const rows = await all(ctx.env, `SELECT id,slug,title,listing_type,category,district,price,currency,prog_status,visible,featured,photo_count,photo_order,updated_at FROM listings ${where} ORDER BY featured DESC, sort_key DESC, id DESC LIMIT ? OFFSET ?`, ...a, pg.per, pg.offset);
  const cats = (await all(ctx.env, "SELECT DISTINCT category FROM listings WHERE category<>'' ORDER BY category")).map((r) => r.category);
  const tags = (await all(ctx.env, 'SELECT name FROM tags ORDER BY sort_key,name')).map((r) => r.name);
  const sel = (name, val, opts, all_) => html`<select name="${name}"><option value="">${all_}</option>${opts.map((o) => html`<option${o === val ? raw(' selected') : ''}>${o}</option>`)}</select>`;
  const body = html`<form class="row filters" method="get"><input type="search" name="q" value="${q}" placeholder="Başlık, ilçe, mahalle veya no">${sel('tur', tur, ['Satılık', 'Kiralık'], 'Tür: hepsi')}${sel('kategori', kat, cats, 'Kategori: hepsi')}${sel('durum', durum, STATUSES, 'Durum: hepsi')}<select name="gorunur"><option value="">Görünürlük: hepsi</option><option value="1"${gor === '1' ? raw(' selected') : ''}>Görünen</option><option value="0"${gor === '0' ? raw(' selected') : ''}>Gizli</option></select>${sel('etiket', et, tags, 'Etiket: hepsi')}<button class="btn" type="submit">Süz</button> <a class="btn ghost" href="/admin/ilanlar">Temizle</a> <a class="btn primary" href="/admin/ilanlar/yeni">+ Yeni ilan (elle)</a></form>
<div class="card"><p class="muted">${total} ilan. İlanlar ofis programından otomatik gelir; burada her alanı elle değiştirebilir, gizleyebilir veya silebilirsiniz.</p><div class="tablewrap"><table class="tbl"><thead><tr><th>No</th><th>İlan</th><th>Durum</th><th>Fiyat</th><th>Foto</th><th>Site</th><th></th></tr></thead><tbody>${rows.length ? rows.map((l) => html`<tr><td>${l.id}</td><td><a href="/admin/ilanlar/${l.id}"><strong>${l.title}</strong></a><div class="muted">${l.listing_type} ${l.category} · ${l.district}</div></td><td>${l.prog_status}</td><td class="nowrap">${fmtPrice(l.price, l.currency)}</td><td>${photoNums(l).length}</td><td>${l.visible ? html`<span class="badge ok">görünür</span>` : html`<span class="badge off">gizli</span>`}${l.featured ? html` <span class="badge">öne çıkan</span>` : ''}</td><td class="actions"><a class="btn sm" href="/admin/ilanlar/${l.id}">Düzenle</a> <a class="btn sm ghost" href="/ilan/${encodeURIComponent(l.slug)}" target="_blank" rel="noopener">Siteyi aç</a></td></tr>`) : emptyRow(7, 'Eşleşen ilan yok.')}</tbody></table></div>${pageLinks(ctx.url, pg.page, pg.pages)}</div>`;
  return shell(ctx, { title: 'İlanlar', body });
}

// ------------------------------------------------------------------ islemler
async function newListing(ctx) {
  const env = ctx.env;
  const mx = Number(await scalar(env, 'SELECT COALESCE(MAX(id),0) FROM listings')) || 0;
  const id = Math.max(mx, 1000000) + 1;
  const title = ctx.s('title') || 'Yeni ilan';
  const slug = `${slugify(title) || 'ilan'}-${id}`;
  await run(env, "INSERT INTO listings(id,slug,title,listing_type,category,prog_status,visible,published_at,updated_at,overrides) VALUES(?,?,?,?,?,'Taslak',0,?,?,'{}')", id, slug, title.slice(0, 200), 'Satılık', 'Villa', nowIso(), nowIso());
  await logActivity(env, ctx.who, `İlan elle oluşturuldu: #${id}`);
  return okRedirect(ctx, `/admin/ilanlar/${id}`, 'İlan oluşturuldu (gizli/taslak). Alanları doldurup durumu “Yayında” yapın.');
}
async function listingAction(ctx, parts) {
  const env = ctx.env;
  if (parts[1] === 'yeni') {
    if (ctx.method === 'POST') return newListing(ctx);
    const body = postForm(ctx, '/admin/ilanlar/yeni', html`<div class="card form">${fld({ name: 'title', label: 'İlan başlığı', req: true })}<button class="btn primary" type="submit">Oluştur</button></div>`);
    return shell(ctx, { title: 'Yeni ilan', active: '/admin/ilanlar', body });
  }
  const id = toInt(parts[1], 0);
  const l = id ? await first(env, 'SELECT * FROM listings WHERE id=?', id) : null;
  if (!l) return notFoundPage(ctx, 'İlan');
  const act = parts[2] || '';
  const back = `/admin/ilanlar/${id}`;
  if (ctx.method === 'POST') {
    if (act === 'kaydet') return saveListing(ctx, l);
    if (act === 'gorunur') { await run(env, 'UPDATE listings SET visible=? WHERE id=?', l.visible ? 0 : 1, id); await logActivity(env, ctx.who, `İlan ${l.visible ? 'gizlendi' : 'gösterildi'}: #${id}`); return okRedirect(ctx, ctx.s('back') || back, l.visible ? 'İlan sitede gizlendi.' : 'İlan sitede görünür.'); }
    if (act === 'one-cikar') { await run(env, 'UPDATE listings SET featured=? WHERE id=?', l.featured ? 0 : 1, id); return okRedirect(ctx, back, l.featured ? 'Öne çıkarma kaldırıldı.' : 'İlan öne çıkarıldı.'); }
    if (act === 'override-kaldir') {
      const o = parseJson(l.overrides, {}); delete o[ctx.s('field')];
      await run(env, 'UPDATE listings SET overrides=? WHERE id=?', JSON.stringify(o), id);
      return okRedirect(ctx, back, 'Elle değişiklik kaldırıldı; bir sonraki program eşitlemesinde alan programdan gelen değerle güncellenir.');
    }
    if (act === 'sil') {
      await deleteListingR2(env, id); await run(env, 'DELETE FROM listings WHERE id=?', id);
      await logActivity(env, ctx.who, `İlan silindi: #${id} ${l.title}`);
      return okRedirect(ctx, '/admin/ilanlar', 'İlan silindi.');
    }
    if (act === 'foto-yukle') {
      const files = ctx.fileList('photos'); let added = 0; let errs = [];
      const order = photoNums(l);
      let next = order.length ? Math.max(...order) + 1 : 0;
      for (const f of files) {
        const c = await checkImage(f); if (!c.ok) { errs.push(c.error); continue; }
        await env.MEDIA.put(`l/${id}/${next}.jpg`, c.bytes, { httpMetadata: { contentType: c.kind.type } });
        order.push(next++); added++;
      }
      await run(env, 'UPDATE listings SET photo_order=?, photo_count=?, updated_at=? WHERE id=?', JSON.stringify(order), order.length, nowIso(), id);
      if (added) await logActivity(env, ctx.who, `Fotoğraf yüklendi: #${id} (${added})`);
      return added ? okRedirect(ctx, back, `${added} fotoğraf yüklendi.` + (errs.length ? ' Bazıları reddedildi: ' + errs[0] : '')) : errRedirect(ctx, back, errs[0] || 'Dosya seçilmedi.');
    }
    if (act === 'foto-sil' || act === 'foto-yukari' || act === 'foto-asagi' || act === 'foto-kapak') {
      const n = toInt(ctx.s('n'), -1); const order = photoNums(l); const i = order.indexOf(n);
      if (i < 0) return errRedirect(ctx, back, 'Fotoğraf bulunamadı.');
      if (act === 'foto-sil') { order.splice(i, 1); await env.MEDIA.delete(`l/${id}/${n}.jpg`); }
      else if (act === 'foto-yukari' && i > 0) [order[i - 1], order[i]] = [order[i], order[i - 1]];
      else if (act === 'foto-asagi' && i < order.length - 1) [order[i + 1], order[i]] = [order[i], order[i + 1]];
      else if (act === 'foto-kapak') { order.splice(i, 1); order.unshift(n); }
      await run(env, 'UPDATE listings SET photo_order=?, photo_count=?, updated_at=? WHERE id=?', JSON.stringify(order), order.length, nowIso(), id);
      return okRedirect(ctx, back, 'Fotoğraflar güncellendi.');
    }
    return errRedirect(ctx, back, 'Geçersiz işlem.');
  }
  return editPage(ctx, l);
}
async function deleteListingR2(env, id) {
  let cursor;
  do { const r = await env.MEDIA.list({ prefix: `l/${id}/`, cursor, limit: 500 }); if (r.objects.length) await env.MEDIA.delete(r.objects.map((o) => o.key)); cursor = r.truncated ? r.cursor : undefined; } while (cursor);
}
async function saveListing(ctx, l) {
  const env = ctx.env; const id = l.id; const back = `/admin/ilanlar/${id}`;
  const ov = parseJson(l.overrides, {}); const set = {};
  for (const f of TEXT_FIELDS) { if (!ctx.form[f]) continue; let v = ctx.r(f).replace(/\r\n/g, '\n'); v = f.startsWith('desc_') ? v.trim() : v.trim().slice(0, MAXLEN[f] || 120); set[f] = v; }
  if (set.title !== undefined && !set.title) return errRedirect(ctx, back, 'Başlık boş olamaz.');
  if (set.listing_type !== undefined && !['Satılık', 'Kiralık'].includes(set.listing_type)) return errRedirect(ctx, back, 'Tür Satılık veya Kiralık olmalı.');
  if (set.prog_status !== undefined && !STATUSES.includes(set.prog_status)) return errRedirect(ctx, back, 'Geçersiz durum.');
  if (set.currency !== undefined) set.currency = (set.currency || 'TRY').toUpperCase().slice(0, 4);
  for (const f of NUM_FIELDS) if (ctx.form[f]) set[f] = Math.max(f.includes('lat') || f.includes('lon') ? -180 : 0, toNum(ctx.s(f), 0));
  for (const f of BOOL_FIELDS) if (ctx.form[f]) set[f] = truthy(ctx.s(f)) ? 1 : 0;
  if (ctx.form._tags_present) set.tags = JSON.stringify([...new Set(ctx.all('tags').map((x) => x.trim()).filter(Boolean))]);
  const adminSet = {};
  if (ctx.form.visible) adminSet.visible = truthy(ctx.s('visible')) ? 1 : 0;
  if (ctx.form.featured) adminSet.featured = truthy(ctx.s('featured')) ? 1 : 0;
  if (ctx.form.sort_key) adminSet.sort_key = toInt(ctx.s('sort_key'), 0);
  if (ctx.form.published_at && ctx.s('published_at')) adminSet.published_at = ctx.s('published_at').slice(0, 25);
  if (ctx.form.slug && ctx.s('slug')) {
    const sl = slugify(ctx.s('slug')); const clash = sl ? await first(env, 'SELECT id FROM listings WHERE slug=? AND id<>?', sl, id) : null;
    if (!sl || clash) return errRedirect(ctx, back, 'Bu bağlantı adı (slug) boş veya başka ilanda kullanılıyor.');
    adminSet.slug = sl;
  }
  for (const [k, v] of Object.entries(set)) { if (String(l[k] ?? '') !== String(v)) ov[k] = true; }
  const all_ = { ...set, ...adminSet, overrides: JSON.stringify(ov), updated_at: nowIso() };
  const keys = Object.keys(all_);
  await run(env, `UPDATE listings SET ${keys.map((k) => `${k}=?`).join(',')} WHERE id=?`, ...keys.map((k) => all_[k]), id);
  await logActivity(env, ctx.who, `İlan düzenlendi: #${id}`);
  return okRedirect(ctx, back, 'İlan kaydedildi. Elle değiştirdiğiniz alanlar program eşitlemesinde korunur.');
}

async function editPage(ctx, l) {
  const id = l.id; const ov = parseJson(l.overrides, {});
  const tagsAll = await all(ctx.env, 'SELECT name,color FROM tags ORDER BY sort_key,name');
  const mine = new Set(jarr(l.tags));
  const ovb = (f) => (ov[f] ? html` <span class="badge ovr" title="Elle değiştirildi; program eşitlemesi bu alanı değiştirmez">elle</span>` : '');
  const F = (name, label, o = {}) => fld({ name, label, value: l[name], flag: ovb(name), ...o });
  const nums = photoNums(l);
  const gallery = nums.length ? html`<div class="gallery">${nums.map((n, i) => html`<figure><img src="/img/l/${id}/${n}.jpg?v=${esc(l.updated_at)}" alt="Fotoğraf ${i + 1}" loading="lazy"><figcaption>${i === 0 ? html`<b>Kapak</b> ` : ''}#${i + 1}
${btnPost(ctx, `/admin/ilanlar/${id}/foto-yukari`, '←', { fields: { n } })}${btnPost(ctx, `/admin/ilanlar/${id}/foto-asagi`, '→', { fields: { n } })}${i ? btnPost(ctx, `/admin/ilanlar/${id}/foto-kapak`, 'Kapak yap', { fields: { n } }) : ''}${btnPost(ctx, `/admin/ilanlar/${id}/foto-sil`, 'Sil', { cls: 'btn sm danger', confirm: 'Fotoğraf silinsin mi?', fields: { n } })}</figcaption></figure>`)}</div>` : html`<p class="muted">Fotoğraf yok.</p>`;
  const body = html`<p class="row"><a class="btn ghost" href="/admin/ilanlar">← İlanlar</a> <a class="btn ghost" href="/ilan/${encodeURIComponent(l.slug)}" target="_blank" rel="noopener">Sitede aç</a> ${btnPost(ctx, `/admin/ilanlar/${id}/gorunur`, l.visible ? 'Sitede gizle' : 'Sitede göster', { cls: 'btn' })}${btnPost(ctx, `/admin/ilanlar/${id}/one-cikar`, l.featured ? 'Öne çıkarmayı kaldır' : 'Öne çıkar')}${btnPost(ctx, `/admin/ilanlar/${id}/sil`, 'İlanı kalıcı sil', { cls: 'btn danger', confirm: 'İlan ve tüm fotoğrafları kalıcı olarak silinsin mi? (Program yeniden gönderirse geri gelir.)' })}</p>
${postForm(ctx, `/admin/ilanlar/${id}/kaydet`, html`<input type="hidden" name="_tags_present" value="1">
<div class="card form"><h2>Temel bilgiler</h2><div class="grid two">${F('title', 'Başlık (TR)', { req: true })}${F('title_en', 'Title (EN)')}${F('title_ru', 'Заголовок (RU)')}${F('slug', 'Bağlantı adı (slug)', { help: 'Sitedeki adres: /ilan/<slug>', flag: '' })}
${F('listing_type', 'Tür', { type: 'select', options: ['Satılık', 'Kiralık'] })}${F('category', 'Kategori (Villa, Daire, Arazi…)', { attrs: { list: 'catlist' } })}${F('prog_status', 'Durum', { type: 'select', options: STATUSES })}${F('published_at', 'Yayın tarihi', { flag: '' })}</div></div>
<div class="card form"><h2>Konum</h2><div class="grid two">${F('district', 'İlçe')}${F('neighborhood', 'Mahalle')}${F('address', 'Adres')}${F('lat', 'Enlem', { type: 'number', step: 'any' })}${F('lon', 'Boylam', { type: 'number', step: 'any' })}${F('hide_exact_location', 'Tam konumu gizle (yaklaşık daire)', { type: 'bool' })}</div></div>
<div class="card form"><h2>Fiyat ve özellikler</h2><div class="grid two">${F('price', 'Fiyat', { type: 'number', step: 'any' })}${F('currency', 'Para birimi', { type: 'select', options: ['TRY', 'EUR', 'USD', 'GBP', 'RUB'] })}${F('price_note', 'Fiyat notu')}${F('gross_m2', 'Brüt m²', { type: 'number', step: 'any' })}${F('net_m2', 'Net m²', { type: 'number', step: 'any' })}${F('rooms', 'Oda')}${F('bathrooms', 'Banyo')}${F('building_age', 'Bina yaşı')}${F('floor', 'Kat')}${F('total_floors', 'Kat sayısı')}${F('heating', 'Isıtma')}${F('deed_status', 'Tapu')}${F('furnished', 'Eşyalı', { type: 'bool' })}${F('swap_ok', 'Takas', { type: 'bool' })}${F('credit_ok', 'Krediye uygun', { type: 'bool' })}</div></div>
<div class="card form"><h2>Açıklamalar</h2>${F('desc_tr', 'Açıklama (TR)', { type: 'textarea', rows: 8 })}${F('desc_en', 'Description (EN)', { type: 'textarea', rows: 6 })}${F('desc_ru', 'Описание (RU)', { type: 'textarea', rows: 6 })}</div>
<div class="card form"><h2>Etiketler</h2><div class="tagpick">${tagsAll.length ? tagsAll.map((t) => html`<label class="tagchip" style="--c:${t.color}"><input type="checkbox" name="tags" value="${t.name}"${mine.has(t.name) ? raw(' checked') : ''}> ${t.name}</label>`) : html`<span class="muted">Etiket yok. <a href="/admin/etiketler">Etiket ekleyin</a>.</span>`}</div>${ov.tags ? html`<small>Etiketler elle değiştirildi.</small>` : ''}</div>
<div class="card form"><h2>Danışman ve site ayarları</h2><div class="grid two">${F('agent_name', 'Danışman adı')}${F('agent_phone', 'Danışman telefonu')}${fld({ name: 'visible', label: 'Sitede görünür', type: 'bool', value: l.visible })}${fld({ name: 'featured', label: 'Öne çıkan', type: 'bool', value: l.featured })}${fld({ name: 'sort_key', label: 'Sıra anahtarı (büyük = önce)', type: 'number', value: l.sort_key })}</div></div>
<datalist id="catlist"><option>Villa</option><option>Daire</option><option>Arazi</option><option>Müstakil</option><option>İşyeri</option><option>Tarla</option></datalist>
<div class="sticky-actions"><button class="btn primary" type="submit">Kaydet</button></div>`, { id: 'listingform' })}
${Object.keys(ov).length ? html`<div class="card"><h2>Elle değiştirilen alanlar</h2><p class="muted">Bu alanlara program eşitlemesi dokunmaz. “Programdan yeniden al” ile kaldırırsanız bir sonraki eşitlemede programdaki değer geri gelir.</p><div class="row">${Object.keys(ov).map((f) => btnPost(ctx, `/admin/ilanlar/${id}/override-kaldir`, `${f} ✕ programdan yeniden al`, { fields: { field: f } }))}</div></div>` : ''}
<div class="card"><h2>Fotoğraflar (${nums.length})</h2>${gallery}${postForm(ctx, `/admin/ilanlar/${id}/foto-yukle`, html`<input type="file" name="photos" accept="image/jpeg,image/png,image/webp" multiple required> <button class="btn" type="submit">Yükle</button> <small>JPEG/PNG/WebP, en çok 8 MB</small>`, { multipart: true })}</div>`;
  return shell(ctx, { title: `İlan #${id}`, active: '/admin/ilanlar', body });
}

// ------------------------------------------------------------------ Etiketler
async function tagsSection(ctx, parts) {
  const env = ctx.env; const act = parts[1] || '';
  if (ctx.method === 'POST') {
    const name = ctx.s('name').slice(0, 40);
    const color = /^#[0-9a-fA-F]{6}$/.test(ctx.s('color')) ? ctx.s('color') : '#d4af37';
    if (act === 'ekle') {
      if (!name) return errRedirect(ctx, '/admin/etiketler', 'Etiket adı gerekli.');
      if (await first(env, 'SELECT id FROM tags WHERE name=?', name)) return errRedirect(ctx, '/admin/etiketler', 'Bu etiket zaten var.');
      await run(env, 'INSERT INTO tags(name,name_en,name_ru,color,sort_key) VALUES(?,?,?,?,?)', name, ctx.s('name_en').slice(0, 40), ctx.s('name_ru').slice(0, 40), color, toInt(ctx.s('sort_key'), 0));
      await logActivity(env, ctx.who, `Etiket eklendi: ${name}`);
      return okRedirect(ctx, '/admin/etiketler', 'Etiket eklendi.');
    }
    const id = toInt(parts[2], 0); const t = id ? await first(env, 'SELECT * FROM tags WHERE id=?', id) : null;
    if (!t) return errRedirect(ctx, '/admin/etiketler', 'Etiket bulunamadı.');
    if (act === 'kaydet') {
      if (!name) return errRedirect(ctx, '/admin/etiketler', 'Etiket adı gerekli.');
      if (name !== t.name && await first(env, 'SELECT id FROM tags WHERE name=?', name)) return errRedirect(ctx, '/admin/etiketler', 'Bu ad başka bir etikette var.');
      await run(env, 'UPDATE tags SET name=?,name_en=?,name_ru=?,color=?,sort_key=? WHERE id=?', name, ctx.s('name_en').slice(0, 40), ctx.s('name_ru').slice(0, 40), color, toInt(ctx.s('sort_key'), 0), id);
      if (name !== t.name) await retag(env, t.name, name);
      await logActivity(env, ctx.who, `Etiket düzenlendi: ${t.name} → ${name}`);
      return okRedirect(ctx, '/admin/etiketler', 'Etiket güncellendi.');
    }
    if (act === 'sil') { await retag(env, t.name, null); await run(env, 'DELETE FROM tags WHERE id=?', id); await logActivity(env, ctx.who, `Etiket silindi: ${t.name}`); return okRedirect(ctx, '/admin/etiketler', 'Etiket silindi ve ilanlardan çıkarıldı.'); }
    return errRedirect(ctx, '/admin/etiketler', 'Geçersiz işlem.');
  }
  const rows = await all(env, 'SELECT * FROM tags ORDER BY sort_key,name');
  const counts = {}; for (const l of await all(env, "SELECT tags FROM listings WHERE tags<>'[]'")) for (const n of jarr(l.tags)) counts[n] = (counts[n] || 0) + 1;
  const body = html`<div class="card"><p class="muted">Etiketler ilan kartlarının üstünde rozet olarak görünür. Programda portföy oluştururken de bu liste seçilir (program etiket listesini siteyle eşitler).</p><div class="tablewrap"><table class="tbl"><thead><tr><th>Etiket</th><th>EN</th><th>RU</th><th>Renk</th><th>Sıra</th><th>İlan</th><th></th></tr></thead><tbody>${rows.length ? rows.map((t) => html`<tr><td colspan="7"><form method="post" action="/admin/etiketler/kaydet/${t.id}" class="tagrow"><input type="hidden" name="_csrf" value="${ctx.csrf}"><input name="name" value="${t.name}" aria-label="Ad"><input name="name_en" value="${t.name_en}" aria-label="EN"><input name="name_ru" value="${t.name_ru}" aria-label="RU"><input type="color" name="color" value="${t.color}" aria-label="Renk"><input type="number" name="sort_key" value="${t.sort_key}" aria-label="Sıra" style="width:5em"><span class="muted">${counts[t.name] || 0} ilan</span><button class="btn sm primary" type="submit">Kaydet</button></form>${btnPost(ctx, `/admin/etiketler/sil/${t.id}`, 'Sil', { cls: 'btn sm danger', confirm: `“${t.name}” etiketi silinsin ve ilanlardan çıkarılsın mı?` })}</td></tr>`) : emptyRow(7, 'Etiket yok.')}</tbody></table></div></div>
<div class="card"><h2>Yeni etiket</h2>${postForm(ctx, '/admin/etiketler/ekle', html`<div class="grid two">${fld({ name: 'name', label: 'Ad (TR)', req: true })}${fld({ name: 'name_en', label: 'Name (EN)' })}${fld({ name: 'name_ru', label: 'Название (RU)' })}${fld({ name: 'color', label: 'Renk', type: 'color', value: '#d4af37' })}</div><button class="btn primary" type="submit">Ekle</button>`)}</div>`;
  return shell(ctx, { title: 'Etiketler', body });
}
async function retag(env, from, to) {
  for (const l of await all(env, 'SELECT id,tags FROM listings WHERE tags LIKE ?', `%${JSON.stringify(from).slice(1, -1)}%`)) {
    const a = jarr(l.tags); if (!a.includes(from)) continue;
    const b = [...new Set(a.map((x) => (x === from ? to : x)).filter(Boolean))];
    await run(env, 'UPDATE listings SET tags=? WHERE id=?', JSON.stringify(b), l.id);
  }
}
