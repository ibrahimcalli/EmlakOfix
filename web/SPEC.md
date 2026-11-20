# sahibindenfethiye.com.tr — Cloudflare Worker sitesi (şartname)

Sahibi: İbrahim (Fethiye emlakçısı, programcı DEĞİL). Arayüz dili Türkçe; site TR/EN/RU.
Hedef: Cloudflare Workers (JS, derleme adımı YOK, tek bağımlılık gerektirmeden) + D1 (schema.sql) + R2 (bağlama adı MEDIA) + statik varlıklar (public/, bağlama ASSETS).
Gizli değerler (Worker secret): SITE_TOKEN (program <-> site paylaşılan anahtar). Değişkenler: SITE_URL.
İlk açılışta (D1 boş ise) admin/kurulum ekranı: /admin/kurulum ilk yöneticiyi oluşturur VEYA SSO ile girilir. Varsayılan menü, sayfalar (Hakkımızda, İletişim), kategoriler, etiketler seed'lenir (seed/ klasörü).
Çalışma ortamı sınırı: bu sandbox'ta Cloudflare'e erişim YOK. Bu yüzden: kod, Cloudflare API'lerini ince bir katmanın (src/env.js: db.prepare().bind().all()/first()/run(), r2.get/put/delete/list) arkasında kullanır; testler node:sqlite tabanlı bir D1 taklidi ve bellek içi R2 taklidiyle `node --test` ile çalışır (test/shim.js). Worker kodu `export default { fetch }` biçimindedir.

## Program -> Site API (Bearer SITE_TOKEN)
- GET  /api/sync/ping -> {ok:true, version, listings:n, posts:n}
- PUT  /api/sync/listing/:id   JSON gövde: {id,title,title_en,title_ru,listing_type,category,district,neighborhood,address,lat,lon,price,currency,gross_m2,net_m2,rooms,building_age,floor,total_floors,heating,deed_status,furnished,swap_ok,credit_ok,desc_tr,desc_en,desc_ru,tags:[ad,...],photo_count,agent_name,agent_phone,prog_status,published_at}. Upsert; `overrides` içindeki alanlara DOKUNMAZ. slug yoksa başlıktan üret (Türkçe karakterleri çevir) + "-<id>".
- DELETE /api/sync/listing/:id  (listeyi + R2 fotoğraflarını siler)
- PUT  /api/sync/photo/:id/:n  ham JPEG gövde -> R2 `l/<id>/<n>.jpg`; DELETE aynı yol.
- PUT  /api/sync/tags  {tags:[{name,name_en,name_ru,color}]}  (ad bazlı upsert, silmez)
- PUT  /api/sync/post  {slug?,title,category,excerpt,body,image_name?,status,author,lang_variants?}  category = ad; yoksa oluşturulur. Gövde sanitize edilir.
- PUT  /api/sync/media/:name  ham görsel -> R2 `m/<name>`; yanıt {url:"/media/<name>"}
- GET  /media/<name> ve /img/l/<id>/<n>.jpg : R2'den, Cache-Control uzun, ETag.
- GET  /api/sync/export -> tüm ilan/yazı/ayar JSON (yedek)
SSO: GET /admin/sso?u=<ad>&n=<görünen ad>&exp=<unix sn>&sig=<hex HMAC-SHA256(SITE_TOKEN, "u|exp")> ; exp geçmemiş (en çok 5 dk ileride) ve imza doğruysa o adla admin oturumu açılır (admins tablosunda source='program' kaydı oluşur). Programdaki yönetici = web yöneticisi.

## Genel site (responsive: telefon/tablet/masaüstü; CSS değişkenleriyle tema; sağ kenar çubuğu masaüstünde sağda, mobilde altta)
Referans tarz: https://www.ibrahimcalli.com.tr/ (WordPress/Estatik): üstte logo + telefon + menü, listede kartlar, sağda kenar çubuğu (Ara kutusu, SAYFALAR menüsü, Kategoriler, EMLAK HABER, SEKTÖREL HABERLER), altta sosyal ikonlar + telif satırı.
- Ana sayfa: banner/slider (banners tablosu; ok+nokta, otomatik; yoksa varsayılan marka bandı), ardından ilan vitrini: görünüm anahtarı THUMB (varsayılan) / LİSTE (seçim çerezde hatırlanır); sütun sayısı 2/3/4 ve sayfa başı ilan sayısı admin ayarı (settings: home_columns, home_per_page, home_default_view, home_show_sidebar). Sayfalama. Filtre çubuğu (tür, kategori, ilçe, fiyat aralığı, etiket, arama).
- İlan kartı: öne çıkan fotoğraf; kart üstünde SAĞ/SOL oklarla fotoğraflar sayfaya girmeden değişir (JS, ilk fotoğraf hemen, diğerleri tembel yüklenir; dokunmatik kaydırma); sol üstte ETİKET rozetleri (tags tablosundaki renkle), sağ üstte Satılık/Kiralık; başlık, konum, fiyat (TRY + isteğe bağlı döviz karşılığı ayarı), oda/banyo/m²; satıldı/kiralandı ise şerit.
- Menü (menu_items, ağaç/alt menü, admin tarafından eklenip çıkarılıp sıralanabilir): varsayılan: ANA SAYFA, SATILIK VİLLA, SATILIK DAİRE, SATILIK ARAZİ, KİRALIK VİLLA, KİRALIK DAİRE, DİĞER İLANLAR, BLOG, HAKKIMIZDA, İLETİŞİM. kind=listings target "Satılık|Villa" -> /satilik-villa ; "DİĞER İLANLAR" = tanımlı kategoriler dışındakiler (target "*|*" özel anlamı: menüdeki diğer filtrelerde olmayanlar). Her sayfa slug'ı menüden gelir, TR slug'lar (satilik-villa).
- İlan detayı /ilan/<slug>: galeri (büyük + küçük resimler, tam ekran, klavye/dokunmatik), etiketler, özellik tablosu, açıklama (sitenin dili sekmesi/anahtarı TR/EN/RU; seçilen dil boşsa TR'ye düşer), Leaflet+OpenStreetMap haritası (hide_exact_location ise ~300 m kaydırılmış daire), WhatsApp/telefon/form düğmeleri, benzer ilanlar, paylaş, Schema.org JSON-LD, Open Graph.
- Dil: /en/…, /ru/… önekleri; dil anahtarı üstte; hreflang.
- Blog: /blog (kategori listesi + yazılar), /blog/<kategori-slug>, /yazi/<slug> (sanitize edilmiş HTML, görsel, ilgili yazılar), kenar çubuğunda son yazılar/haberler. kind=news kategorileri "EMLAK HABER / SEKTÖREL HABERLER" olarak görünür.
- Sayfalar /<slug> (pages), İletişim formu (messages'a yazar; basit bot koruması: bal tuzağı alanı + zaman + IP başına hız sınırı), /sitemap.xml, /robots.txt, 404, güvenlik başlıkları (CSP uyumlu, nonce yoksa inline script'i en aza indir), sıkıştırma/önbellek.
- Tema ayarları (settings, hepsi admin panelinde): site adı, logo (varsayılan /logo.png), favicon, renk paleti (birincil, vurgu, arka plan, yüzey, metin, kenarlık, açık/koyu mod + sistem), yazı tipleri (başlık/gövde: güvenli liste; Google Fonts ile yükleme açma/kapama), köşe yuvarlaklığı, kart gölgesi, kapsayıcı genişliği, başlık stili, kenar çubuğu yan/gizli, kenar çubuğu parçaları (hangi widget, sırası), alt bilgi metni, sosyal bağlantılar, telefon/WhatsApp/e-posta/adres, analitik kimliği, özel CSS, özel head/footer HTML (yalnızca admin), bakım modu.

## Admin paneli /admin (Türkçe, responsive, kullanışlı; "her detaya müdahale")
Giriş (kullanıcı adı+parola PBKDF2/WebCrypto, deneme sınırı, oturum çerezi HttpOnly Secure SameSite=Lax, CSRF belirteci tüm POST'larda) + SSO.
Bölümler: Panel (sayaçlar, son mesajlar, son etkinlik) · İlanlar (liste/arama/filtre, TÜM alanları düzenle — düzenlenen alan overrides'a işlenir, "programdan yeniden al" ile geri dönülür —, gizle/göster, öne çıkar, sırala, fotoğraf sırası/silme/yükleme, etiket seç, sil) · Etiketler (ekle/düzenle/sil/renk/çeviri) · Menü (sürükle-bırak veya yukarı/aşağı; ekle/sil/gizle; alt menü) · Blog yazıları (liste, WYSIWYG-lite editör: kalın/italik/başlık/liste/bağlantı/görsel yükleme+yerleştirme; kategori, öne çıkan görsel, taslak/yayın, TR/EN/RU gövde, SEO alanları) · Haberler (kind=news kategorileri; aynı editör) · Kategoriler · Sayfalar · Bannerlar · Mesajlar (gelen kutusu, okundu, sil, yanıtla bağlantısı) · Tema ve Görünüm (yukarıdaki tüm ayarlar, canlı önizleme) · Genel ayarlar · Kullanıcılar (admin ekle/sil/parola, program yöneticileri) · Medya kitaplığı (R2 listesi, yükle, sil) · Yedek (export/import JSON) · Etkinlik günlüğü · **TESTLER**.
**TESTLER bölümü**: tek düğme "Tüm testleri çalıştır" (+ modül başına çalıştır). Worker içinde çalışan canlı sistem testleri (veritabanı şeması, her API, sayfa üretimi, menü, etiketler, blog, tema CSS, kimlik/SSO/CSRF, sanitize/XSS, medya/R2, iletişim formu, sitemap, i18n, ayar etkileri, sync round-trip) — test verisi `__test__` ön ekiyle yazılıp sonunda temizlenir; sonuç tablosu (modül, test, ✓/✗, süre, hata), geçmiş çalıştırmalar (test_runs), "raporu indir". Aynı test gövdeleri `node --test` ile yerelde de çalışır (npm test).

## Güvenlik / kalite
Tüm SQL bind parametreli; HTML çıktısı kaçışlı; blog/sayfa HTML'i beyaz liste ile sanitize; yükleme tür/boyut kontrolü; hız sınırları; sabit-zamanlı token karşılaştırma. Erişilebilirlik (alt, label, odak), hızlı yükleme (görseller lazy), Türkçe yerel biçim (1.550.000 ₺).
Seed: seed/posts.json = ibrahimcalli.com.tr'den 5 yazı (tam metin ve görseller çekilebiliyorsa), şu kategorilerde: ARAZİ ÇEŞİTLERİ VE İMAR DURUMLARI, GAYRİMENKUL SATIN ALMA SÜREÇLERİ, HİSSELİ ARAZİLERLE İLGİLİ YAZILAR, SEKTÖRE DAİR ÇIKAN HABERLER (news). Varsayılan etiketler: Müstakil, Villa, Fırsat, Yeni, Deniz Manzaralı, Eşyalı, Havuzlu, Yatırımlık, Takas, Krediye Uygun.
Deploy: wrangler.toml (assets, d1, r2, secrets notu), package.json (test script), README_KURULUM.md (Cloudflare panelinden adım adım: D1 oluştur, schema çalıştır, R2, Workers Builds ile GitHub bağlama root dizini `web`, SITE_TOKEN secret, alan adı sahibindenfethiye.com.tr bağlama).

## Ek sözleşme notları (program tarafı yazıldı ve derlendi; Worker bunlara UYMALI)
- Program, ilanı yalnızca yönetici web yayınını onayladıktan sonra gönderir. `prog_status` değeri Yayında / Kiralandı / Satıldı dışındaysa (Pasif, Taslak vb.) ilan sitede GİZLİ olmalı (listelerde ve sitemap'te yok, detay 404).
- Fotoğraflar `PUT /api/sync/photo/:id/:n` ile (n=0 kapak) ham JPEG olarak gelir; ilan JSON'u (`photo_count`) fotoğraflardan SONRA gelir. Eksik fotoğraf varsa kırık görsel göstermeyin. Fazla fotoğraflar için `DELETE /api/sync/photo/:id/:n` gelebilir (gövde boş, Content-Type application/octet-stream).
- `PUT /api/sync/post` yanıtı JSON olmalı ve `{ok:true, id, slug, url:"<tam adres>"}` içermeli (program `url` alanını gösterir). `image_name` verilmişse (daha önce /api/sync/media ile yüklenmiş) öne çıkan görsel odur. `status` "yayinda"|"taslak". Aynı slug/başlık tekrar gelirse güncelle (upsert).
- `GET /api/sync/ping` yanıtı `{ok:true, version, listings, posts}`.
- SSO'da `u` = programdaki kullanıcı adı, `n` = görünen ad.
- Tüm /api/sync/* uçları SITE_TOKEN dışında 401 döner (sabit zamanlı karşılaştırma). Hata gövdeleri kısa Türkçe metin.
- Etiketler: `tags` tablosunda ad bazlı; ilanın `tags` JSON'u etiket ADLARINI tutar; kartta tags tablosundaki renk kullanılır, EN/RU sayfada name_en/name_ru (boşsa ad).
