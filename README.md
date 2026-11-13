# Emlak Ofis (EmlakOfix)

Rust (egui + SQLite + tiny_http) ile yazılmış, Windows ve Linux'ta çalışan emlak ofisi masaüstü programı.
Yalnızca ofis içi kullanım içindir; tek bir ana bilgisayar sunucu olur, diğer bilgisayarlar ağdan bağlanır.

## Özellikler
- Kullanıcı girişi, roller (yönetici / müdür / çalışan); CRM kayıtları yalnızca yöneticiye görünür
- Portföy formu (fotoğraf, konum), küçük resim / liste görünümü (boyutlanabilir), Muğla ilçe-mahalle verisi
- Harita: kendi portföylerim / hepsi, satılık-kiralık-arazi renkleri, kısa etiketler
- PDF / DOCX tanıtım çıktısı (TR / EN / DE / RU), WhatsApp paylaşımı
- Mesai, görev ve bildirimler, hedefler, takvim, izin/puantaj, komisyon, denetim kaydı
- Ağ üzerinden ortak kullanım (otomatik keşif, isteğe bağlı HTTPS + sertifika sabitleme), şifreli yedek
- Uygulama içi güncelleme + geri alma, ed25519 imzalı güncelleme paketleri
- Gerçek harita altlığı (OpenStreetMap sokak, Esri uydu; önbellekli), arayüz Türkçe / English

## Derleme
Linux (Mint/Ubuntu): `./derle_linux.sh` — Windows: `derle_windows.bat`

Programda Ayarlar > Testler sekmesi tüm otomatik testleri çalıştırır (sonuçlar kopyalanabilir veya DOCX olarak kaydedilir).
Elle: `cargo build --release` ; test: `cargo test --release`
Linux için gerekli paketler: `build-essential pkg-config libxkbcommon-dev libwayland-dev`

## Güncelleme paketi
`python3 arac/paketle_guncelleme.py --surum X.Y.Z --linux target/release/emlak-ofis`
Programda: Ayarlar > Hakkında ve güncelleme > "Güncelleme paketi seç (.zip)".

## Güvenlik notu
Veritabanı (`emlak.db`), yedekler ve ayar dosyaları müşteri bilgisi içerir; bu depoya **eklenmez** (`.gitignore`).
Bağlantı şifreli değildir (HTTP); yalnızca güvendiğiniz ofis ağında kullanın, dışarıya açmayın.
