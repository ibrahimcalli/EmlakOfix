// Varsayılan blog yazıları (ibrahimcalli.com.tr kaynaklı). Worker dosya okuyamadığı için JS modülü.
// NOT: Kaynak siteye bu oturumda erişilemedi (WebFetch izin listesinde değil); aşağıdaki gövdeler KISA ÖZETTİR.
// Tam metinleri yönetim panelinden (Blog yazıları) yapıştırarak değiştirebilirsiniz. Her özet kaynak bağlantısını içerir.
const SRC = 'https://www.ibrahimcalli.com.tr/';
const note = (path) => `<p><i>Bu yazı kısa özettir. Ayrıntılı ve güncel metin için <a href="${SRC}${path}/" target="_blank">kaynak yazıyı</a> okuyabilirsiniz.</i></p>`;
const warn = '<p><b>Önemli:</b> Mevzuat sık değişir. İşlem yapmadan önce ilgili belediye, tapu müdürlüğü veya yetkili mali müşavir ile güncel durumu teyit edin.</p>';

export default [
  {
    slug: 'kirsal-yerlesik-alanlarda-hisseli-parsellerin-yapilasma-rejimi',
    title: 'Kırsal Yerleşik Alanlarda Hisseli Parsellerin Yapılaşma Rejimi',
    category: 'HİSSELİ ARAZİLERLE İLGİLİ YAZILAR',
    excerpt: 'Hisseli tapulu parsellerde, özellikle kırsal yerleşik alanlarda yapı yapabilmenin temel şartları ve dikkat edilmesi gereken noktalar.',
    published_at: '2025-01-15T09:00:00.000Z',
    body: `${note('kirsal-yerlesik-alanlarda-hisseli-parsellerin-yapilasma-rejimi')}
<h2>Hisseli tapu nedir?</h2>
<p>Hisseli tapuda bir parselin mülkiyeti birden fazla kişiye paylı (hisseli) olarak aittir. Her hissedarın parselin belirli bir yeri üzerinde değil, tamamında oranına göre hakkı vardır.</p>
<h2>Yapılaşmada neye bakılır?</h2>
<ul>
<li>Parselin imar durumu: planlı alan mı, kırsal yerleşik alan mı, tarım arazisi mi?</li>
<li>Hissedarların yapı yapılmasına muvafakati (rızası).</li>
<li>İlgili belediye veya il özel idaresinden alınacak yapı ruhsatı ve parselin yapılaşma koşulları.</li>
<li>Parselin ifraz (bölünme) şartlarını sağlayıp sağlamadığı.</li>
</ul>
<h2>Satın almadan önce</h2>
<p>Hisseli bir parseli almadan önce tapu kaydını, imar durumunu ve diğer hissedarların durumunu mutlaka araştırın. Hissenin fiilen kullanılacağı alanın belirlenmesi çoğu zaman diğer hissedarlarla anlaşmayı gerektirir.</p>
${warn}`,
  },
  {
    slug: '7584-sayili-kanun-ve-6831-ek-madde-22-2b-ve-orman-kadastrosunda-yeni-donem',
    title: '7584 Sayılı Kanun ve 6831 Ek Madde 22 (2/B) ve Orman Kadastrosunda Yeni Dönem',
    category: 'SEKTÖRE DAİR ÇIKAN HABERLER',
    excerpt: 'Orman kadastrosu ve 2/B uygulamalarına ilişkin yeni düzenlemenin taşınmaz sahipleri ve alıcılar açısından özeti.',
    published_at: '2025-03-10T09:00:00.000Z',
    body: `${note('7584-sayili-kanun-ve-6831-ek-madde-22-2b-ve-orman-kadastrosunda-yeni-donem')}
<h2>Konu nedir?</h2>
<p>Orman Kanunu'nda yapılan değişiklikler, orman kadastrosu ve orman sınırları dışına çıkarılan (2/B) alanlara ilişkin uygulamaları etkilemektedir. Bu alanlarda taşınmazı olan veya satın almayı düşünenler için düzenlemenin kapsamı önemlidir.</p>
<h2>Taşınmaz alırken dikkat edilecekler</h2>
<ul>
<li>Tapu kaydında ve ilgili kurum kayıtlarında orman veya 2/B şerhi olup olmadığını sorgulayın.</li>
<li>Parselin kadastro ve sınır durumunu güncel harita üzerinden kontrol edin.</li>
<li>Satış sözleşmesinden önce yetkili kurumlardan yazılı bilgi almayı tercih edin.</li>
</ul>
${warn}`,
  },
  {
    slug: 'evini-kiraya-verenler-icin-kira-geliri-beyannamesi-rehberi-2026-guncel',
    title: 'Evini Kiraya Verenler İçin Kira Geliri Beyannamesi Rehberi (2026 Güncel)',
    category: 'SEKTÖRE DAİR ÇIKAN HABERLER',
    excerpt: 'Konut kira geliri elde edenlerin beyan yükümlülüğü, istisna ve beyan zamanı hakkında genel bilgilendirme.',
    published_at: '2026-01-20T09:00:00.000Z',
    body: `${note('evini-kiraya-verenler-icin-kira-geliri-beyannamesi-rehberi-2026-guncel')}
<h2>Kimler beyanname verir?</h2>
<p>Konut olarak kiraya verdiği taşınmazdan gelir elde eden kişiler, yıllık kira geliri o yıl için belirlenen istisna tutarını aşıyorsa gelirini beyan etmekle yükümlüdür. İstisna tutarı her yıl yeniden belirlenir.</p>
<h2>Genel çerçeve</h2>
<ul>
<li>Beyanname, kira gelirinin elde edildiği yılı izleyen yılın mart ayında verilir.</li>
<li>Gider yöntemi (götürü veya gerçek gider) seçimi vergi tutarını etkiler.</li>
<li>Kira gelirlerinin banka veya PTT aracılığıyla tahsil edilmesi gerekir.</li>
</ul>
<p>Güncel istisna tutarı, oranlar ve takvim için Gelir İdaresi Başkanlığı duyurularına bakın.</p>
${warn}`,
  },
  {
    slug: 'ticari-arsa-nedir',
    title: 'Ticari Arsa Nedir?',
    category: 'ARAZİ ÇEŞİTLERİ VE İMAR DURUMLARI',
    excerpt: 'İmar planında ticaret alanı olarak ayrılmış arsaların özellikleri, kullanım şekilleri ve değerini etkileyen unsurlar.',
    published_at: '2025-02-05T09:00:00.000Z',
    body: `${note('ticari-arsa-nedir')}
<h2>Tanım</h2>
<p>Ticari arsa, imar planında ticaret (ticari) kullanımına ayrılmış arsadır. Üzerinde iş yeri, mağaza, ofis gibi ticari yapılar inşa edilebilir.</p>
<h2>Değerini etkileyen unsurlar</h2>
<ul>
<li>Konum, ana yola cephe ve ulaşım kolaylığı.</li>
<li>İmar planındaki yapılaşma koşulları (emsal, kat adedi).</li>
<li>Çevredeki ticari yoğunluk ve müşteri potansiyeli.</li>
</ul>
<p>Satın almadan önce imar durumu belgesini ilgili belediyeden alarak yapılaşma koşullarını teyit etmek gerekir.</p>
${warn}`,
  },
  {
    slug: 'muhtelif-tapu-nedir',
    title: 'Muhtelif Tapu Nedir?',
    category: 'GAYRİMENKUL SATIN ALMA SÜREÇLERİ',
    excerpt: 'Tapu kayıtlarında geçen "muhtelif" ifadesinin ne anlama geldiği ve satın alma sürecinde nelere dikkat edilmesi gerektiği.',
    published_at: '2025-02-20T09:00:00.000Z',
    body: `${note('muhtelif-tapu-nedir')}
<h2>Kısaca</h2>
<p>"Muhtelif" sözcüğü "çeşitli" anlamına gelir. Tapu kayıtlarında karşılaşılan bu ifadenin kapsamı ve sonuçları için kaydın tapu müdürlüğünden alınacak güncel örneğiyle incelenmesi gerekir.</p>
<h2>Satın alma sürecinde</h2>
<ul>
<li>Güncel tapu kaydı örneğini ve takyidat (şerh, ipotek) bilgisini isteyin.</li>
<li>Satıcının tapudaki malik olduğunu ve yetkisini doğrulayın.</li>
<li>Devir işlemini tapu müdürlüğünde, resmi değerleme raporu ile gerçekleştirin.</li>
</ul>
${warn}`,
  },
];
