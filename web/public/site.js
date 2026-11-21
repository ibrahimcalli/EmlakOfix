/* sahibindenfethiye.com.tr — küçük, bağımlılıksız istemci kodu */
(function () {
  'use strict';
  var d = document, S = d.currentScript || d.querySelector('script[data-l-prev]');
  var L = function (k, f) { return (S && S.getAttribute('data-l-' + k)) || f; };
  var $ = function (s, r) { return (r || d).querySelector(s); };
  var $$ = function (s, r) { return Array.prototype.slice.call((r || d).querySelectorAll(s)); };

  // --- menü (mobil) ---
  var nt = $('.nav-toggle'), nav = $('#nav');
  if (nt && nav) nt.addEventListener('click', function () {
    var o = nav.classList.toggle('open'); nt.setAttribute('aria-expanded', o ? 'true' : 'false');
  });
  $$('.sub-toggle').forEach(function (b) {
    b.addEventListener('click', function () {
      var li = b.closest('li'), o = li.classList.toggle('open'); b.setAttribute('aria-expanded', o ? 'true' : 'false');
    });
  });

  // --- banner kaydırıcı ---
  $$('[data-slider]').forEach(function (el) {
    var slides = $$('.slide', el), dots = $$('.dot', el), i = 0, timer = null;
    if (slides.length < 2) return;
    function go(n) {
      i = (n + slides.length) % slides.length;
      slides.forEach(function (s, k) { s.classList.toggle('active', k === i); s.setAttribute('aria-hidden', k === i ? 'false' : 'true'); });
      dots.forEach(function (x, k) { x.classList.toggle('on', k === i); });
    }
    function play() {
      stop();
      if (el.getAttribute('data-autoplay') === '1' && !window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        timer = setInterval(function () { go(i + 1); }, parseInt(el.getAttribute('data-interval'), 10) || 6000);
    }
    function stop() { if (timer) { clearInterval(timer); timer = null; } }
    var p = $('.hero-prev', el), n = $('.hero-next', el);
    if (p) p.addEventListener('click', function () { go(i - 1); play(); });
    if (n) n.addEventListener('click', function () { go(i + 1); play(); });
    dots.forEach(function (x, k) { x.addEventListener('click', function () { go(k); play(); }); });
    el.addEventListener('mouseenter', stop); el.addEventListener('mouseleave', play);
    el.addEventListener('focusin', stop); el.addEventListener('focusout', play);
    var sx = null;
    el.addEventListener('touchstart', function (e) { sx = e.touches[0].clientX; stop(); }, { passive: true });
    el.addEventListener('touchend', function (e) {
      if (sx !== null) { var dx = e.changedTouches[0].clientX - sx; if (Math.abs(dx) > 40) go(i + (dx < 0 ? 1 : -1)); }
      sx = null; play();
    });
    go(0); play();
  });

  // --- ilan kartı: sayfaya girmeden fotoğraf değiştirme ---
  $$('.cm[data-photos]').forEach(function (cm) {
    var photos = (cm.getAttribute('data-photos') || '').split(',').filter(Boolean), base = cm.getAttribute('data-base');
    var prev = $('.cm-prev', cm), next = $('.cm-next', cm), cnt = $('.cm-count', cm), img = $('.cm-link img', cm), i = 0, sx = null;
    if (photos.length < 2 || !img) return;
    if (prev) prev.hidden = false; if (next) next.hidden = false;
    var cache = {};
    function show(n) {
      i = (n + photos.length) % photos.length;
      var url = base + photos[i] + '.jpg';
      img.src = url; img.alt = img.alt || '';
      if (cnt) cnt.textContent = (i + 1) + '/' + photos.length;
      var nx = base + photos[(i + 1) % photos.length] + '.jpg';
      if (!cache[nx]) { cache[nx] = new Image(); cache[nx].src = nx; }
    }
    function stopNav(e) { e.preventDefault(); e.stopPropagation(); }
    if (prev) prev.addEventListener('click', function (e) { stopNav(e); show(i - 1); });
    if (next) next.addEventListener('click', function (e) { stopNav(e); show(i + 1); });
    cm.addEventListener('touchstart', function (e) { sx = e.touches[0].clientX; }, { passive: true });
    cm.addEventListener('touchend', function (e) {
      if (sx === null) return; var dx = e.changedTouches[0].clientX - sx; sx = null;
      if (Math.abs(dx) > 40) { show(i + (dx < 0 ? 1 : -1)); cm.setAttribute('data-swiped', '1'); setTimeout(function () { cm.removeAttribute('data-swiped'); }, 400); }
    });
    cm.addEventListener('click', function (e) { if (cm.getAttribute('data-swiped')) { e.preventDefault(); e.stopPropagation(); } }, true);
    img.addEventListener('error', function () { if (photos.length > 1) { photos.splice(i, 1); if (photos.length < 2) { if (prev) prev.hidden = true; if (next) next.hidden = true; if (cnt) cnt.hidden = true; } show(i); } });
    cm.addEventListener('mouseenter', function () { show(i); var nx = base + photos[(i + 1) % photos.length] + '.jpg'; if (!cache[nx]) { cache[nx] = new Image(); cache[nx].src = nx; } }, { once: true });
  });

  // --- ilan galerisi + tam ekran ---
  $$('[data-gallery]').forEach(function (g) {
    var photos = (g.getAttribute('data-photos') || '').split(',').filter(Boolean), base = g.getAttribute('data-base'), alt = g.getAttribute('data-alt') || '';
    var img = $('.g-main img', g), cnt = $('.g-count', g), thumbs = $$('.g-thumbs button', g), i = 0;
    function show(n) {
      i = (n + photos.length) % photos.length; img.src = base + photos[i] + '.jpg';
      if (cnt) cnt.textContent = (i + 1) + ' / ' + photos.length;
      thumbs.forEach(function (t, k) { if (k === i) t.setAttribute('aria-current', 'true'); else t.removeAttribute('aria-current'); });
      if (thumbs[i] && thumbs[i].scrollIntoView) thumbs[i].scrollIntoView({ block: 'nearest', inline: 'center' });
    }
    var p = $('.g-prev', g), n = $('.g-next', g);
    if (p) p.addEventListener('click', function () { show(i - 1); });
    if (n) n.addEventListener('click', function () { show(i + 1); });
    thumbs.forEach(function (t) { t.addEventListener('click', function () { show(parseInt(t.getAttribute('data-i'), 10) || 0); }); });
    var sx = null, main = $('.g-main', g);
    main.addEventListener('touchstart', function (e) { sx = e.touches[0].clientX; }, { passive: true });
    main.addEventListener('touchend', function (e) { if (sx === null) return; var dx = e.changedTouches[0].clientX - sx; sx = null; if (Math.abs(dx) > 40) show(i + (dx < 0 ? 1 : -1)); });
    var open = $('.g-open', g);
    if (open) open.addEventListener('click', function () {
      var lb = d.createElement('div'); lb.className = 'lb'; lb.setAttribute('role', 'dialog'); lb.setAttribute('aria-modal', 'true');
      lb.innerHTML = '<img alt=""><button class="x" type="button"></button><button class="p" type="button"></button><button class="n" type="button"></button><span class="c"></span>';
      var li = $('img', lb), c = $('.c', lb);
      $('.x', lb).textContent = '×'; $('.x', lb).setAttribute('aria-label', L('close', 'Close'));
      $('.p', lb).textContent = '‹'; $('.p', lb).setAttribute('aria-label', L('prev', 'Previous'));
      $('.n', lb).textContent = '›'; $('.n', lb).setAttribute('aria-label', L('next', 'Next'));
      function upd() { li.src = base + photos[i] + '.jpg'; li.alt = alt; c.textContent = (i + 1) + ' / ' + photos.length; show(i); }
      function close() { lb.remove(); d.removeEventListener('keydown', key); open.focus(); }
      function key(e) { if (e.key === 'Escape') close(); else if (e.key === 'ArrowLeft') { i = (i - 1 + photos.length) % photos.length; upd(); } else if (e.key === 'ArrowRight') { i = (i + 1) % photos.length; upd(); } }
      $('.x', lb).addEventListener('click', close);
      $('.p', lb).addEventListener('click', function () { i = (i - 1 + photos.length) % photos.length; upd(); });
      $('.n', lb).addEventListener('click', function () { i = (i + 1) % photos.length; upd(); });
      lb.addEventListener('click', function (e) { if (e.target === lb) close(); });
      var lsx = null;
      lb.addEventListener('touchstart', function (e) { lsx = e.touches[0].clientX; }, { passive: true });
      lb.addEventListener('touchend', function (e) { if (lsx === null) return; var dx = e.changedTouches[0].clientX - lsx; lsx = null; if (Math.abs(dx) > 40) { i = (i + (dx < 0 ? 1 : -1) + photos.length) % photos.length; upd(); } });
      d.addEventListener('keydown', key); d.body.appendChild(lb); upd(); $('.x', lb).focus();
    });
  });

  // --- bağlantı kopyala ---
  $$('[data-copy]').forEach(function (b) {
    b.addEventListener('click', function () {
      var v = b.getAttribute('data-copy'), done = function () { var o = b.textContent; b.textContent = L('copied', 'Copied'); setTimeout(function () { b.textContent = o; }, 1500); };
      if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(v).then(done, done); else done();
    });
  });
  // --- sıralama otomatik gönder ---
  $$('[data-autosubmit]').forEach(function (s) { s.addEventListener('change', function () { var f = s.form || $('#ff'); if (f) f.submit(); }); });

  // --- harita (görününce yükle: Leaflet + OpenStreetMap) ---
  var map = $('#map');
  if (map && 'IntersectionObserver' in window) {
    new IntersectionObserver(function (en, ob) {
      if (!en[0].isIntersecting) return; ob.disconnect();
      var css = d.createElement('link'); css.rel = 'stylesheet'; css.href = map.getAttribute('data-css'); d.head.appendChild(css);
      var js = d.createElement('script'); js.src = map.getAttribute('data-js'); js.onload = function () {
        var lat = parseFloat(map.getAttribute('data-lat')), lon = parseFloat(map.getAttribute('data-lon')), approx = map.getAttribute('data-approx') === '1';
        var m = window.L.map(map, { scrollWheelZoom: false }).setView([lat, lon], approx ? 14 : 15);
        var street = window.L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', { maxZoom: 19, attribution: '&copy; OpenStreetMap' });
        var sat = window.L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}', { maxZoom: 19, attribution: 'Imagery &copy; Esri, Maxar, Earthstar Geographics' });
        var base = {}; base[map.getAttribute('data-lbl-sat') || 'Uydu'] = sat; base[map.getAttribute('data-lbl-street') || 'Sokak'] = street;
        (map.getAttribute('data-base') === 'sokak' ? street : sat).addTo(m);
        window.L.control.layers(base, null, { collapsed: false }).addTo(m);
        if (approx) window.L.circle([lat, lon], { radius: parseInt(map.getAttribute('data-radius'), 10) || 300, color: '#d4af37', fillOpacity: .2 }).addTo(m);
        else window.L.marker([lat, lon]).addTo(m);
      };
      d.head.appendChild(js);
    }, { rootMargin: '200px' }).observe(map);
  }
})();
