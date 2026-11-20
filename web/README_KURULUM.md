# sahibindenfethiye.com.tr — Cloudflare Kurulum Rehberi

Bu klasör (`kaynak/web`) sitenin tamamıdır. Derleme gerektirmez. Cloudflare üzerinde 3 şey kullanır:
**Worker** (site + yönetim paneli), **D1** (veritabanı), **R2** (fotoğraf/görsel deposu).

## 1) Cloudflare hesabı ve alan adı
1. dash.cloudflare.com → hesap açın / girin.
2. `sahibindenfethiye.com.tr` alan adını Cloudflare'e ekleyin (Add a site). Alan adı sağlayıcınızda **nameserver**'ları Cloudflare'in verdiği ikisiyle değiştirin (yayılması birkaç saat sürebilir).

## 2) Veritabanı (D1)
1. Sol menü **Storage & Databases → D1 SQL Database → Create**. Ad: `sahibindenfethiye`.
2. Oluşunca **Database ID**'yi kopyalayın → `wrangler.toml` içindeki `BURAYA-D1-KIMLIGI` yerine yapıştırın (GitHub'da düzenleyin).
3. Veritabanı sayfasında **Console** sekmesi → `schema.sql` dosyasının **tüm içeriğini** yapıştırıp **Execute** edin.

## 3) Görsel deposu (R2)
**R2 Object Storage → Create bucket**. Ad: `sahibindenfethiye-medya` (kamuya açık yapmayın; site kendisi sunar).

## 4) Siteyi yayınlama (iki yoldan biri)
**A) GitHub ile (önerilen):** Workers & Pages → Create → **Import a repository** → GitHub deponuzu seçin → *Root directory*: `kaynak/web` → Deploy. Her GitHub güncellemesinde site kendiliğinden yenilenir.
**B) Bilgisayardan:** `cd kaynak/web && npx wrangler deploy` (Node.js gerekir; ilk seferde `npx wrangler login`).

## 5) Gizli anahtar (SITE_TOKEN)
Workers → `sahibindenfethiye` → **Settings → Variables and Secrets → Add → Type: Secret**, ad `SITE_TOKEN`, değer: uzun rastgele bir metin (en az 32 karakter; örn. parola üretici). **Aynı metni** programda da gireceksiniz. Kaydedince **Deploy**.

## 6) Alan adını bağlama
Workers → `sahibindenfethiye` → **Settings → Domains & Routes → Add → Custom domain** → `sahibindenfethiye.com.tr` (ve isterseniz `www.sahibindenfethiye.com.tr`).

## 7) Programı bağlama
Program → **Ayarlar → Web sitesi**: Adres `https://sahibindenfethiye.com.tr`, Anahtar = SITE_TOKEN → **Bağlantıyı dene** (yeşil “bağlandı” görmelisiniz).

## 8) Yönetim paneline giriş
- Programdaki **“Web yönetim girişi”** düğmesi sizi şifresiz (imzalı bağlantıyla) panele alır; program yöneticisi = web yöneticisi.
- Doğrudan `https://sahibindenfethiye.com.tr/admin` → ilk açılışta **/admin/kurulum** ile kendi yöneticinizi de oluşturabilirsiniz.

## 9) Testler
Panelde **Testler** → **▶ Tüm testleri çalıştır** (veya tek modül). Sonuçlar listelenir, JSON rapor indirilebilir. Testler `__test__` önekli veri üretir ve işi bitince siler; gerçek ilan/yazılara dokunmaz. Güncelleme sonrası bir kez çalıştırmanız önerilir. Bilgisayarda: `cd kaynak/web && node --test test/all.test.js` (Node 22+).

## 10) Günlük kullanım
- İlanlar programdan gelir; programda **“Web’de yayınla”** talebini yönetici onaylayınca ilan sitede çıkar ve **Web yayını** hedefine otomatik sayılır.
- Panelden ilan alanını değiştirirseniz o alan programdan gelen güncellemeyle ezilmez (ilan sayfasında “override kaldır” ile geri alınır).
- Blog/haber: panelden veya programdaki **Blog yayınla** sekmesinden (resimli).
- Tema, renk, yazı tipi, menü, banner, sütun sayısı (2/3/4), sayfa başına ilan: **Ayarlar** ve **Menü**.
- Yedek: **Yedek → Dışa aktar** (JSON). Fotoğraflar R2'de durur; ayrıca programda zaten özgün kopyalar vardır.

## Sık sorunlar
| Belirti | Çözüm |
|---|---|
| Sitede “Beklenmeyen hata” | D1'e `schema.sql` çalıştırılmamış ya da `database_id` yanlış |
| Programda “Yetkisiz” | SITE_TOKEN iki yerde farklı (boşluk/satır sonu dikkat) |
| Görseller çıkmıyor | R2 kovası adı `sahibindenfethiye-medya` değil / bağlama (MEDIA) yok |
| Alan adı açılmıyor | Nameserver değişimi yayılmadı; Custom domain bağlanmadı |

## Ücretsiz sınırlar (yaklaşık, Cloudflare değiştirebilir)
Worker günde 100.000 istek; D1 5 GB ve günlük okuma/yazma kotası; R2 10 GB depolama. Tek bir emlak ofisi sitesi için genelde fazlasıyla yeter; aşılırsa ücretli plan aylık birkaç dolardır.

## Dürüst notlar
Bu site Cloudflare'in kendisinde henüz **canlı denenmedi**; testler yerel D1/R2 benzetimiyle geçti. İlk yayında Testler sayfasını çalıştırıp çıkan hatayı bana iletin.
