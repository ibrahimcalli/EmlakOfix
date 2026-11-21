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
  // testler: her test ayri istekte calisir (Cloudflare istek basina sorgu siniri), canli ilerleme, kopyala
  var out = $('#testout');
  if (out) {
    var esc = function (s) { return String(s).replace(/[&<>"]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); };
    var post = function (path, params) {
      return fetch(path, { method: 'POST', credentials: 'same-origin', headers: { 'X-Requested-With': 'fetch', 'X-CSRF-Token': csrf, 'Content-Type': 'application/x-www-form-urlencoded' }, body: new URLSearchParams(params || {}).toString() }).then(function (r) { return r.text().then(function (txt) { try { return JSON.parse(txt); } catch (e) { var m = /<title>([^<]*)<\/title>/i.exec(txt); throw new Error('HTTP ' + r.status + ' — JSON yerine sayfa döndü' + (m ? ': ' + m[1].trim() : (r.redirected ? ' (yönlendirildi: ' + r.url + ')' : ''))); } }); });
    };
    var copyText = function (txt, btn) {
      var done = function () { var o = btn.textContent; btn.textContent = '✓ Kopyalandı'; setTimeout(function () { btn.textContent = o; }, 1800); };
      if (navigator.clipboard && window.isSecureContext) navigator.clipboard.writeText(txt).then(done, function () { fallback(); }); else fallback();
      function fallback() { var ta = document.createElement('textarea'); ta.value = txt; ta.style.position = 'fixed'; ta.style.opacity = '0'; document.body.appendChild(ta); ta.select(); try { document.execCommand('copy'); done(); } catch (e) { prompt('Kopyalayın (Ctrl+C):', txt); } document.body.removeChild(ta); }
    };
    var asText = function (res, ms, onlyFail) {
      var failed = res.filter(function (r) { return !r.ok; }).length;
      var t = (failed ? '✗ ' + failed + ' başarısız, ' : '✓ ') + (res.length - failed) + ' geçti — ' + res.length + ' test, ' + (ms / 1000).toFixed(1) + ' sn\nModül\tTest\t\tSüre\tHata\n';
      res.slice().sort(function (a, b) { return a.ok === b.ok ? 0 : a.ok ? 1 : -1; }).forEach(function (r) { if (onlyFail && r.ok) return; t += r.module + '\t' + r.name + '\t' + (r.ok ? '✓' : '✗') + '\t' + r.ms + ' ms\t' + (r.error || '') + '\n'; });
      return t;
    };
    var render = function (res, ms, finished, total) {
      var failed = res.filter(function (r) { return !r.ok; }).length;
      var h = '<p class="testsum ' + (failed ? 'fail' : 'pass') + '">' + (finished ? '' : '⏳ ' + res.length + ' / ' + total + ' — ') + (failed ? '✗ ' + failed + ' başarısız, ' : '✓ ') + (res.length - failed) + ' geçti' + (finished ? ' — ' + res.length + ' test, ' + (ms / 1000).toFixed(1) + ' sn' : '') + '</p>';
      if (!finished) h += '<progress max="' + total + '" value="' + res.length + '" style="width:100%"></progress>';
      else h += '<p class="row"><button class="btn" id="cp-all" type="button">📋 Sonucu kopyala</button><button class="btn" id="cp-fail" type="button">📋 Yalnız başarısızları kopyala</button></p>';
      h += '<table><thead><tr><th>Modül</th><th>Test</th><th></th><th>Süre</th><th>Hata</th></tr></thead><tbody>';
      res.slice().sort(function (a, b) { return a.ok === b.ok ? 0 : a.ok ? 1 : -1; }).forEach(function (t) { h += '<tr><td>' + esc(t.module) + '</td><td>' + esc(t.name) + '</td><td class="' + (t.ok ? 'pass' : 'fail') + '">' + (t.ok ? '✓' : '✗') + '</td><td>' + t.ms + ' ms</td><td>' + esc(t.error || '') + '</td></tr>'; });
      out.innerHTML = h + '</tbody></table>';
      if (finished) {
        var a = $('#cp-all'), f = $('#cp-fail');
        if (a) a.addEventListener('click', function () { copyText(asText(res, ms, false), a); });
        if (f) f.addEventListener('click', function () { copyText(asText(res, ms, true), f); });
      }
    };
    var running = false;
    $$('[data-run]').forEach(function (b) {
      b.addEventListener('click', function () {
        if (running) return; running = true;
        var mod = b.getAttribute('data-run'); $$('[data-run]').forEach(function (x) { x.disabled = true; });
        var t0 = Date.now(), res = [];
        var finish = function (msg) { running = false; $$('[data-run]').forEach(function (x) { x.disabled = false; }); if (msg) out.innerHTML = '<p class="fail">' + esc(msg) + '</p>'; };
        out.innerHTML = '<p>⏳ Hazırlanıyor…</p>';
        post('/admin/testler/baslat').then(function (j) {
          if (!j.ok) throw new Error(j.error || 'Başlatılamadı');
          return fetch('/admin/testler/liste' + (mod ? '?module=' + encodeURIComponent(mod) : ''), { credentials: 'same-origin', headers: { 'X-Requested-With': 'fetch' } }).then(function (r) { return r.json(); });
        }).then(function (l) {
          var list = l.tests || [], k = 0;
          var step = function () {
            if (k >= list.length) {
              var ms = Date.now() - t0;
              return post('/admin/testler/bitir', { module: mod, ms: ms, results: JSON.stringify(res) }).catch(function () { return null; }).then(function () { render(res, ms, true, list.length); finish(); });
            }
            var it = list[k++];
            return post('/admin/testler/tek', { i: it.i }).then(function (j) { res.push(j.result || { module: it.module, name: it.name, ok: false, ms: 0, error: j.error || 'Yanıt yok' }); }, function (e) { res.push({ module: it.module, name: it.name, ok: false, ms: 0, error: 'İstek başarısız: ' + e }); }).then(function () { if (k % 3 === 0 || k === list.length) render(res, 0, false, list.length); return step(); });
          };
          return step();
        }).catch(function (e) { finish('Hata: ' + e); });
      });
    });
  }
})();
