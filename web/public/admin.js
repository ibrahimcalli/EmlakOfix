// Yonetim paneli etkilesimleri (CSP: yalnizca 'self' betik; satir ici betik yok)
(function () {
  'use strict';
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var $$ = function (s, r) { return Array.prototype.slice.call((r || document).querySelectorAll(s)); };
  var csrf = (($('meta[name=csrf]') || {}).content) || '';
  function post(url, body, isForm) {
    var h = { 'X-Requested-With': 'fetch', 'X-CSRF-Token': csrf };
    if (!isForm) h['Content-Type'] = 'application/json';
    return fetch(url, { method: 'POST', headers: h, body: isForm ? body : JSON.stringify(body), credentials: 'same-origin' }).then(function (r) { return r.json().catch(function () { return { ok: false, error: 'Beklenmeyen yanıt' }; }); });
  }
  // onay
  document.addEventListener('submit', function (e) {
    var f = e.target; var c = f.getAttribute && f.getAttribute('data-confirm');
    if (c && !window.confirm(c)) { e.preventDefault(); return; }
    var sub = e.submitter; var cb = sub && sub.getAttribute('data-confirmbtn');
    if (cb && !window.confirm(cb)) { e.preventDefault(); return; }
    // editorleri textarea'ya yaz
    $$('.editor', f).forEach(function (ed) { ed._sync && ed._sync(); });
    // widgets
    $$('[data-widgets]', f).forEach(function (w) { var out = $('#' + w.getAttribute('data-widgets')); var a = []; $$('.wrow', w).forEach(function (r) { if ($('input', r).checked) a.push(r.getAttribute('data-k')); }); out.value = JSON.stringify(a); });
  });
  // renk kodu
  document.addEventListener('input', function (e) { var t = e.target; if (t.type === 'color' && t.nextElementSibling && t.nextElementSibling.tagName === 'CODE') t.nextElementSibling.textContent = t.value; });
  // widget sirala
  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-up],[data-down]'); if (!b) return;
    var row = b.closest('.wrow'); if (!row) return;
    if (b.hasAttribute('data-up') && row.previousElementSibling) row.parentNode.insertBefore(row, row.previousElementSibling);
    if (b.hasAttribute('data-down') && row.nextElementSibling) row.parentNode.insertBefore(row.nextElementSibling, row);
  });
  // panoya kopyala
  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-copy]'); if (!b) return;
    var v = b.getAttribute('data-copy'); var done = function () { var o = b.textContent; b.textContent = 'Kopyalandı'; setTimeout(function () { b.textContent = o; }, 1200); };
    if (navigator.clipboard) navigator.clipboard.writeText(location.origin + v).then(done); else { var t = document.createElement('textarea'); t.value = location.origin + v; document.body.appendChild(t); t.select(); document.execCommand('copy'); t.remove(); done(); }
  });
  // gorsel yukleme
  function pickAndUpload(cb) {
    var inp = document.createElement('input'); inp.type = 'file'; inp.accept = 'image/jpeg,image/png,image/webp,image/gif';
    inp.onchange = function () { if (!inp.files[0]) return; var fd = new FormData(); fd.append('file', inp.files[0]); post('/admin/medya/yukle', fd, true).then(function (r) { if (r.ok) cb(r.url); else alert(r.error || 'Yüklenemedi'); }); };
    inp.click();
  }
  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-upload]'); if (!b) return;
    pickAndUpload(function (url) { var t = document.getElementById(b.getAttribute('data-target')); if (t) t.value = url; var pv = document.getElementById(b.getAttribute('data-preview')); if (pv) { pv.src = url; pv.hidden = false; } });
  });
  // medya kitapligi toplu yukleme
  var mu = $('#mediaup'); if (mu) mu.addEventListener('click', function () {
    var files = $('#mediafile').files; var st = $('#mediastatus'); if (!files.length) { st.textContent = 'Dosya seçin.'; return; }
    var n = 0, bad = 0; st.textContent = 'Yükleniyor…';
    var next = function (i) { if (i >= files.length) { st.textContent = n + ' yüklendi' + (bad ? ', ' + bad + ' hata' : ''); if (n) setTimeout(function () { location.reload(); }, 700); return; } var fd = new FormData(); fd.append('file', files[i]); post('/admin/medya/yukle', fd, true).then(function (r) { r.ok ? n++ : bad++; next(i + 1); }); };
    next(0);
  });
  // WYSIWYG-lite
  $$('textarea[data-editor]').forEach(function (ta) {
    var wrap = document.createElement('div'); wrap.className = 'editor';
    var tb = document.createElement('div'); tb.className = 'etb';
    var body = document.createElement('div'); body.className = 'ebody'; body.contentEditable = 'true'; body.innerHTML = ta.value;
    var src = document.createElement('textarea'); src.className = 'esrc mono'; src.rows = Math.max(8, ta.rows || 10);
    var mode = 'wysiwyg';
    function sync() { ta.value = (mode === 'wysiwyg' ? body.innerHTML : src.value); }
    wrap._sync = sync;
    function cmd(c, v) { body.focus(); document.execCommand(c, false, v || null); }
    var btns = [['B', function () { cmd('bold'); }, 'Kalın'], ['I', function () { cmd('italic'); }, 'İtalik'], ['H2', function () { cmd('formatBlock', 'h2'); }, 'Başlık'], ['H3', function () { cmd('formatBlock', 'h3'); }, 'Alt başlık'], ['¶', function () { cmd('formatBlock', 'p'); }, 'Paragraf'],
      ['• Liste', function () { cmd('insertUnorderedList'); }, 'Madde listesi'], ['1. Liste', function () { cmd('insertOrderedList'); }, 'Numaralı liste'], ['❝', function () { cmd('formatBlock', 'blockquote'); }, 'Alıntı'],
      ['🔗', function () { var u = prompt('Bağlantı adresi (https://…):'); if (u) cmd('createLink', u); }, 'Bağlantı'], ['⛓✕', function () { cmd('unlink'); }, 'Bağlantıyı kaldır'],
      ['🖼 Görsel', function () { pickAndUpload(function (url) { body.focus(); document.execCommand('insertHTML', false, '<img src="' + url + '" alt="">'); }); }, 'Görsel yükle ve yerleştir'], ['Temizle', function () { cmd('removeFormat'); }, 'Biçimi temizle'],
      ['</> HTML', function () { if (mode === 'wysiwyg') { src.value = body.innerHTML; body.style.display = 'none'; src.style.display = 'block'; mode = 'html'; this.classList.add('on'); } else { body.innerHTML = src.value; src.style.display = 'none'; body.style.display = ''; mode = 'wysiwyg'; this.classList.remove('on'); } }, 'HTML kaynağı']];
    btns.forEach(function (d) { var b = document.createElement('button'); b.type = 'button'; b.textContent = d[0]; b.title = d[2]; b.addEventListener('mousedown', function (e) { e.preventDefault(); }); b.addEventListener('click', d[1]); tb.appendChild(b); });
    wrap.appendChild(tb); wrap.appendChild(body); wrap.appendChild(src); ta.style.display = 'none'; ta.parentNode.insertBefore(wrap, ta.nextSibling);
    body.addEventListener('input', sync);
  });
  // testler
  var out = $('#testout');
  if (out) $$('[data-run]').forEach(function (b) {
    b.addEventListener('click', function () {
      var mod = b.getAttribute('data-run'); $$('[data-run]').forEach(function (x) { x.disabled = true; });
      out.innerHTML = '<p>⏳ Testler çalışıyor… (birkaç saniye sürebilir)</p>';
      var fd = new URLSearchParams(); if (mod) fd.set('module', mod);
      fetch('/admin/testler/calistir', { method: 'POST', credentials: 'same-origin', headers: { 'X-Requested-With': 'fetch', 'X-CSRF-Token': csrf, 'Content-Type': 'application/x-www-form-urlencoded' }, body: fd.toString() }).then(function (r) { return r.json(); }).then(function (j) {
        $$('[data-run]').forEach(function (x) { x.disabled = false; });
        if (!j.ok) { out.textContent = j.error || 'Hata'; return; }
        var r = j.report, esc = function (s) { return String(s).replace(/[&<>"]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); };
        var h = '<p class="testsum ' + (r.failed ? 'fail' : 'pass') + '">' + (r.failed ? '✗ ' + r.failed + ' başarısız, ' : '✓ ') + r.passed + ' geçti — ' + r.results.length + ' test, ' + (r.ms / 1000).toFixed(1) + ' sn</p><table><thead><tr><th>Modül</th><th>Test</th><th></th><th>Süre</th><th>Hata</th></tr></thead><tbody>';
        r.results.slice().sort(function (a, b) { return (a.ok === b.ok) ? 0 : a.ok ? 1 : -1; }).forEach(function (t) { h += '<tr><td>' + esc(t.module) + '</td><td>' + esc(t.name) + '</td><td class="' + (t.ok ? 'pass' : 'fail') + '">' + (t.ok ? '✓' : '✗') + '</td><td>' + t.ms + ' ms</td><td>' + esc(t.error || '') + '</td></tr>'; });
        out.innerHTML = h + '</tbody></table>';
      }).catch(function (e) { $$('[data-run]').forEach(function (x) { x.disabled = false; }); out.textContent = 'İstek başarısız: ' + e; });
    });
  });
})();
