//! Belge (PDF/DOCX/WhatsApp) çıktısı için dil desteği: TR / EN / DE / RU.
//! Kullanıcının yazdığı metinler (başlık, açıklama, adres) çevrilmez; sabit etiketler ve
//! listeden seçilen değerler (kategori, ısıtma, tapu vb.) çevrilir.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum Lang {
    Tr,
    En,
    De,
    Ru,
}

#[allow(dead_code)]
impl Lang {
    pub const ALL: [Lang; 4] = [Lang::Tr, Lang::En, Lang::De, Lang::Ru];
    pub fn code(self) -> &'static str {
        match self {
            Lang::Tr => "tr",
            Lang::En => "en",
            Lang::De => "de",
            Lang::Ru => "ru",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Lang::Tr => "Türkçe",
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Ru => "Русский",
        }
    }
    pub fn from_code(c: &str) -> Lang {
        match c {
            "en" => Lang::En,
            "de" => Lang::De,
            "ru" => Lang::Ru,
            _ => Lang::Tr,
        }
    }
}

/// (Türkçe, İngilizce, Almanca, Rusça)
const T: &[(&str, &str, &str, &str)] = &[
    ("İlan No", "Listing No", "Inseratsnr.", "№ объявления"),
    ("Konum", "Location", "Lage", "Расположение"),
    ("Fiyat", "Price", "Preis", "Цена"),
    ("Fiyat sorunuz", "Price on request", "Preis auf Anfrage", "Цена по запросу"),
    ("Danışman", "Consultant", "Berater", "Консультант"),
    ("Tel", "Tel", "Tel", "Тел"),
    ("E-posta", "E-mail", "E-Mail", "E-mail"),
    ("Ofis", "Office", "Büro", "Офис"),
    ("Adres", "Address", "Adresse", "Адрес"),
    ("Konum (harita)", "Location (map)", "Lage (Karte)", "Расположение (карта)"),
    ("Özellikler", "Features", "Ausstattung", "Характеристики"),
    ("Açıklama", "Description", "Beschreibung", "Описание"),
    ("İletişim", "Contact", "Kontakt", "Контакты"),
    ("Sayfa", "Page", "Seite", "Стр."),
    ("Durum", "Type", "Art", "Тип"),
    ("Brüt alan", "Gross area", "Bruttofläche", "Общая площадь"),
    ("Net alan", "Net area", "Nettofläche", "Полезная площадь"),
    ("Oda sayısı", "Rooms", "Zimmer", "Комнат"),
    ("Bina yaşı", "Building age", "Gebäudealter", "Возраст здания"),
    ("Kat", "Floor", "Etage", "Этаж"),
    ("Isıtma", "Heating", "Heizung", "Отопление"),
    ("Tapu durumu", "Title deed", "Grundbuch", "Право собственности"),
    ("Eşya", "Furniture", "Möbel", "Мебель"),
    ("Kredi", "Mortgage", "Finanzierung", "Ипотека"),
    ("Takas", "Swap", "Tausch", "Обмен"),
    ("Eşyalı", "Furnished", "Möbliert", "С мебелью"),
    ("Krediye uygun", "Mortgage eligible", "Finanzierbar", "Подходит для ипотеки"),
    ("Takasa uygun", "Open to swap", "Tausch möglich", "Возможен обмен"),
    ("Satılık", "For sale", "Zu verkaufen", "Продаётся"),
    ("Kiralık", "For rent", "Zu vermieten", "Аренда"),
    ("Daire", "Apartment", "Wohnung", "Квартира"),
    ("Villa", "Villa", "Villa", "Вилла"),
    ("Müstakil Ev", "Detached house", "Einfamilienhaus", "Отдельный дом"),
    ("Dubleks / Tripleks", "Duplex / Triplex", "Maisonette", "Дуплекс / Триплекс"),
    ("Rezidans", "Residence", "Residenz", "Резиденция"),
    ("Arsa", "Plot of land", "Grundstück", "Земельный участок"),
    ("Tarla", "Field", "Acker", "Поле"),
    ("Zeytinlik / Bağ-Bahçe", "Olive grove / Garden", "Olivenhain / Garten", "Оливковая роща / Сад"),
    ("Dükkan", "Shop", "Laden", "Магазин"),
    ("Depo", "Warehouse", "Lager", "Склад"),
    ("Otel / Pansiyon", "Hotel / Guesthouse", "Hotel / Pension", "Отель / Пансион"),
    ("Bina", "Building", "Gebäude", "Здание"),
    ("Devren", "Business transfer", "Geschäftsübernahme", "Передача бизнеса"),
    ("Diğer", "Other", "Sonstige", "Другое"),
    ("Yok", "None", "Keine", "Нет"),
    ("Soba", "Stove", "Ofen", "Печь"),
    ("Doğalgaz (Kombi)", "Natural gas (combi boiler)", "Erdgas (Therme)", "Газ (котёл)"),
    ("Merkezi Sistem", "Central heating", "Zentralheizung", "Центральное"),
    ("Klima", "Air conditioning", "Klimaanlage", "Кондиционер"),
    ("Yerden Isıtma", "Underfloor heating", "Fußbodenheizung", "Тёплый пол"),
    ("Güneş Enerjisi", "Solar energy", "Solarenergie", "Солнечная энергия"),
    ("Kat Mülkiyeti", "Condominium title", "Wohnungseigentum", "Кондоминиум"),
    ("Kat İrtifakı", "Construction servitude", "Baulastrecht", "Право застройки"),
    ("Hisseli Tapu", "Shared title deed", "Anteilige Eigentümerschaft", "Долевая собственность"),
    ("Müstakil Tapulu", "Freehold title", "Alleineigentum", "Отдельный титул"),
    ("Tarla Tapulu", "Field title", "Ackerland-Titel", "Титул на поле"),
    ("Arsa Tapulu", "Plot title", "Grundstückstitel", "Титул на участок"),
    ("Zilyetlik", "Possession right", "Besitzrecht", "Право владения"),
    ("Stüdyo", "Studio", "Studio", "Студия"),
    ("Açık Ofis", "Open-plan office", "Großraumbüro", "Open space"),
];

pub fn tr(lg: Lang, s: &str) -> String {
    if lg == Lang::Tr {
        return s.to_string();
    }
    for (a, en, de, ru) in T {
        if *a == s {
            return match lg {
                Lang::En => en,
                Lang::De => de,
                Lang::Ru => ru,
                Lang::Tr => a,
            }
            .to_string();
        }
    }
    s.to_string()
}

/// "Etiket: değer"
pub fn lbl(lg: Lang, label: &str, value: &str) -> String {
    format!("{}: {}", tr(lg, label), value)
}
pub fn lbl_nl(lg: Lang, label: &str, value: &str) -> String {
    format!("{}: {}\n", tr(lg, label), value)
}

pub fn floor_text(lg: Lang, f: &str, t: &str) -> String {
    match (f, t) {
        ("", "") => String::new(),
        (f, "") => f.to_string(),
        ("", t) => match lg {
            Lang::Tr => format!("{t} katlı bina"),
            Lang::En => format!("{t}-storey building"),
            Lang::De => format!("{t}-stöckiges Gebäude"),
            Lang::Ru => format!("{t}-этажное здание"),
        },
        (f, t) => match lg {
            Lang::Tr => format!("{f}. kat / {t} katlı"),
            Lang::En => format!("Floor {f} of {t}"),
            Lang::De => format!("Etage {f} von {t}"),
            Lang::Ru => format!("{f} этаж из {t}"),
        },
    }
}
