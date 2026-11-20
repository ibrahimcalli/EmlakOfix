// Genel site arayuz metinleri: TR / EN / RU. t(lang, key, ...args) -> {0},{1} yer tutucularini doldurur.
export const LANGS = ['tr', 'en', 'ru'];
export const LANG_NAMES = { tr: 'Türkçe', en: 'English', ru: 'Русский' };
export const LANG_SHORT = { tr: 'TR', en: 'EN', ru: 'RU' };
export const HTML_LANG = { tr: 'tr', en: 'en', ru: 'ru' };
export const OG_LOCALE = { tr: 'tr_TR', en: 'en_GB', ru: 'ru_RU' };

export const normalizeLang = (l, fb = 'tr') => (LANGS.includes(l) ? l : fb);

const D = {
  tr: {
    home: 'Ana Sayfa', listings: 'İlanlar', blog: 'Blog', contact: 'İletişim', menu: 'Menü', close: 'Kapat', skip: 'İçeriğe geç',
    language: 'Dil', call: 'Ara', whatsapp: 'WhatsApp', phone: 'Telefon', email: 'E-posta', address: 'Adres',
    for_sale: 'Satılık', for_rent: 'Kiralık', sold: 'Satıldı', rented: 'Kiralandı', sold_note: 'Bu ilan artık yayında değildir.',
    price_ask: 'Fiyat sorunuz', approx: '≈', listing_no: 'İlan No',
    rooms: 'Oda', baths: 'Banyo', m2: 'm²', gross_m2: 'Brüt m²', net_m2: 'Net m²',
    building_age: 'Bina yaşı', floor: 'Bulunduğu kat', total_floors: 'Kat sayısı', heating: 'Isıtma', deed_status: 'Tapu durumu',
    furnished: 'Eşyalı', swap_ok: 'Takas', credit_ok: 'Krediye uygun', yes: 'Evet', no: 'Hayır',
    type: 'İlan türü', category: 'Kategori', district: 'İlçe', neighborhood: 'Mahalle', published: 'Yayın tarihi', location: 'Konum',
    all_types: 'Tüm türler', all_categories: 'Tüm kategoriler', all_districts: 'Tüm ilçeler', all_tags: 'Tüm etiketler',
    min_price: 'En düşük fiyat', max_price: 'En yüksek fiyat', tag: 'Etiket', search: 'Ara', search_ph: 'İlan, yazı veya kelime ara…', filter: 'Filtrele', clear: 'Temizle',
    sort: 'Sıralama', sort_featured: 'Öne çıkanlar', sort_newest: 'En yeni', sort_price_asc: 'Fiyat (artan)', sort_price_desc: 'Fiyat (azalan)',
    view_thumb: 'Kart görünümü', view_list: 'Liste görünümü', results: '{0} ilan bulundu', no_results: 'Aramanıza uygun ilan bulunamadı.',
    prev: 'Önceki', next: 'Sonraki', page: 'Sayfa', page_of: 'Sayfa {0} / {1}', pagination: 'Sayfalama',
    prev_photo: 'Önceki fotoğraf', next_photo: 'Sonraki fotoğraf', photo_n: 'Fotoğraf {0} / {1}', photos: 'Fotoğraflar', no_photo: 'Fotoğraf yok', enlarge: 'Büyüt', fullscreen: 'Tam ekran',
    description: 'Açıklama', features: 'Özellikler', map: 'Harita', map_approx: 'Konum yaklaşık olarak gösterilmektedir.', open_map: 'Haritada aç',
    similar: 'Benzer İlanlar', share: 'Paylaş', copy_link: 'Bağlantıyı kopyala', copied: 'Kopyalandı', details: 'Ayrıntılar',
    wa_text: 'Merhaba, {0} numaralı ilan hakkında bilgi almak istiyorum: {1}',
    ask_about: 'Bu ilan hakkında bilgi alın', send_message: 'Mesaj gönder',
    f_name: 'Adınız Soyadınız', f_phone: 'Telefon', f_email: 'E-posta', f_message: 'Mesajınız', f_send: 'Gönder',
    f_ok: 'Mesajınız alındı. En kısa sürede size dönüş yapacağız.', f_err_invalid: 'Lütfen adınızı, geçerli bir iletişim bilgisini ve mesajınızı doldurun.',
    f_err_rate: 'Çok fazla mesaj gönderdiniz. Lütfen daha sonra tekrar deneyin.', f_err_fast: 'Form çok hızlı gönderildi. Lütfen tekrar deneyin.', f_err_origin: 'İstek reddedildi.',
    contact_us: 'Bize Ulaşın', contact_info: 'İletişim Bilgileri', required: 'zorunlu',
    w_search: 'Ara', w_pages: 'SAYFALAR', w_categories: 'Kategoriler', w_news: 'EMLAK HABER', w_news2: 'SEKTÖREL HABERLER', w_blogposts: 'Son Yazılar', w_tags: 'Etiketler', w_contact: 'İletişim',
    blog_all: 'Tüm yazılar', blog_empty: 'Henüz yazı yok.', read_more: 'Devamını oku', related: 'İlgili Yazılar', by: 'Yazar', latest_posts: 'Son Yazılar', in_cat: '{0} kategorisindeki yazılar',
    notfound_title: 'Sayfa bulunamadı', notfound_text: 'Aradığınız sayfa taşınmış veya kaldırılmış olabilir.', back_home: 'Ana sayfaya dön', browse_listings: 'İlanlara göz atın',
    maintenance: 'Bakım Çalışması', other_listings: 'Diğer İlanlar', all_listings: 'Tüm İlanlar', see_all: 'Tümünü gör',
    breadcrumb: 'Gezinti', slide_prev: 'Önceki', slide_next: 'Sonraki', slide_n: 'Slayt {0}', rights: 'Tüm hakları saklıdır.', followus: 'Bizi takip edin',
    hero_cta: 'İlanları incele', hero_cta2: 'Bize ulaşın', views: 'Görüntülenme',
  },
  en: {
    home: 'Home', listings: 'Listings', blog: 'Blog', contact: 'Contact', menu: 'Menu', close: 'Close', skip: 'Skip to content',
    language: 'Language', call: 'Call', whatsapp: 'WhatsApp', phone: 'Phone', email: 'Email', address: 'Address',
    for_sale: 'For Sale', for_rent: 'For Rent', sold: 'Sold', rented: 'Rented', sold_note: 'This listing is no longer available.',
    price_ask: 'Ask for price', approx: '≈', listing_no: 'Listing No',
    rooms: 'Rooms', baths: 'Baths', m2: 'm²', gross_m2: 'Gross m²', net_m2: 'Net m²',
    building_age: 'Building age', floor: 'Floor', total_floors: 'Total floors', heating: 'Heating', deed_status: 'Title deed',
    furnished: 'Furnished', swap_ok: 'Swap considered', credit_ok: 'Mortgage eligible', yes: 'Yes', no: 'No',
    type: 'Listing type', category: 'Category', district: 'District', neighborhood: 'Neighbourhood', published: 'Published', location: 'Location',
    all_types: 'All types', all_categories: 'All categories', all_districts: 'All districts', all_tags: 'All tags',
    min_price: 'Min price', max_price: 'Max price', tag: 'Tag', search: 'Search', search_ph: 'Search listings, posts or words…', filter: 'Filter', clear: 'Clear',
    sort: 'Sort', sort_featured: 'Featured', sort_newest: 'Newest', sort_price_asc: 'Price (low to high)', sort_price_desc: 'Price (high to low)',
    view_thumb: 'Grid view', view_list: 'List view', results: '{0} listings found', no_results: 'No listings match your search.',
    prev: 'Previous', next: 'Next', page: 'Page', page_of: 'Page {0} of {1}', pagination: 'Pagination',
    prev_photo: 'Previous photo', next_photo: 'Next photo', photo_n: 'Photo {0} of {1}', photos: 'Photos', no_photo: 'No photo', enlarge: 'Enlarge', fullscreen: 'Full screen',
    description: 'Description', features: 'Features', map: 'Map', map_approx: 'The location shown is approximate.', open_map: 'Open in map',
    similar: 'Similar Listings', share: 'Share', copy_link: 'Copy link', copied: 'Copied', details: 'Details',
    wa_text: 'Hello, I would like information about listing no. {0}: {1}',
    ask_about: 'Ask about this listing', send_message: 'Send message',
    f_name: 'Your name', f_phone: 'Phone', f_email: 'Email', f_message: 'Your message', f_send: 'Send',
    f_ok: 'Your message has been received. We will get back to you shortly.', f_err_invalid: 'Please fill in your name, a valid contact detail and your message.',
    f_err_rate: 'Too many messages sent. Please try again later.', f_err_fast: 'The form was sent too quickly. Please try again.', f_err_origin: 'Request rejected.',
    contact_us: 'Get in Touch', contact_info: 'Contact Details', required: 'required',
    w_search: 'Search', w_pages: 'PAGES', w_categories: 'Categories', w_news: 'REAL ESTATE NEWS', w_news2: 'INDUSTRY NEWS', w_blogposts: 'Latest Posts', w_tags: 'Tags', w_contact: 'Contact',
    blog_all: 'All posts', blog_empty: 'No posts yet.', read_more: 'Read more', related: 'Related Posts', by: 'By', latest_posts: 'Latest Posts', in_cat: 'Posts in {0}',
    notfound_title: 'Page not found', notfound_text: 'The page you are looking for may have been moved or removed.', back_home: 'Back to home', browse_listings: 'Browse listings',
    maintenance: 'Under Maintenance', other_listings: 'Other Listings', all_listings: 'All Listings', see_all: 'See all',
    breadcrumb: 'Breadcrumb', slide_prev: 'Previous', slide_next: 'Next', slide_n: 'Slide {0}', rights: 'All rights reserved.', followus: 'Follow us',
    hero_cta: 'Browse listings', hero_cta2: 'Contact us', views: 'Views',
  },
  ru: {
    home: 'Главная', listings: 'Объекты', blog: 'Блог', contact: 'Контакты', menu: 'Меню', close: 'Закрыть', skip: 'К содержимому',
    language: 'Язык', call: 'Позвонить', whatsapp: 'WhatsApp', phone: 'Телефон', email: 'Эл. почта', address: 'Адрес',
    for_sale: 'Продажа', for_rent: 'Аренда', sold: 'Продано', rented: 'Сдано', sold_note: 'Это объявление больше не актуально.',
    price_ask: 'Цена по запросу', approx: '≈', listing_no: 'Номер объекта',
    rooms: 'Комнат', baths: 'Санузлов', m2: 'м²', gross_m2: 'Общая м²', net_m2: 'Полезная м²',
    building_age: 'Возраст здания', floor: 'Этаж', total_floors: 'Этажность', heating: 'Отопление', deed_status: 'Тапу (право собственности)',
    furnished: 'С мебелью', swap_ok: 'Обмен', credit_ok: 'Подходит для ипотеки', yes: 'Да', no: 'Нет',
    type: 'Тип объявления', category: 'Категория', district: 'Район', neighborhood: 'Квартал', published: 'Дата публикации', location: 'Расположение',
    all_types: 'Все типы', all_categories: 'Все категории', all_districts: 'Все районы', all_tags: 'Все метки',
    min_price: 'Цена от', max_price: 'Цена до', tag: 'Метка', search: 'Поиск', search_ph: 'Поиск объектов, статей, слов…', filter: 'Найти', clear: 'Сбросить',
    sort: 'Сортировка', sort_featured: 'Рекомендуемые', sort_newest: 'Новые', sort_price_asc: 'Цена по возрастанию', sort_price_desc: 'Цена по убыванию',
    view_thumb: 'Плитка', view_list: 'Список', results: 'Найдено объектов: {0}', no_results: 'По вашему запросу ничего не найдено.',
    prev: 'Назад', next: 'Вперёд', page: 'Страница', page_of: 'Страница {0} из {1}', pagination: 'Навигация по страницам',
    prev_photo: 'Предыдущее фото', next_photo: 'Следующее фото', photo_n: 'Фото {0} из {1}', photos: 'Фотографии', no_photo: 'Нет фото', enlarge: 'Увеличить', fullscreen: 'Во весь экран',
    description: 'Описание', features: 'Характеристики', map: 'Карта', map_approx: 'Расположение показано приблизительно.', open_map: 'Открыть на карте',
    similar: 'Похожие объекты', share: 'Поделиться', copy_link: 'Копировать ссылку', copied: 'Скопировано', details: 'Подробности',
    wa_text: 'Здравствуйте! Меня интересует объект № {0}: {1}',
    ask_about: 'Спросите об этом объекте', send_message: 'Отправить сообщение',
    f_name: 'Ваше имя', f_phone: 'Телефон', f_email: 'Эл. почта', f_message: 'Сообщение', f_send: 'Отправить',
    f_ok: 'Ваше сообщение получено. Мы скоро свяжемся с вами.', f_err_invalid: 'Укажите имя, корректный контакт и текст сообщения.',
    f_err_rate: 'Слишком много сообщений. Попробуйте позже.', f_err_fast: 'Форма отправлена слишком быстро. Попробуйте ещё раз.', f_err_origin: 'Запрос отклонён.',
    contact_us: 'Свяжитесь с нами', contact_info: 'Контактные данные', required: 'обязательно',
    w_search: 'Поиск', w_pages: 'СТРАНИЦЫ', w_categories: 'Категории', w_news: 'НОВОСТИ НЕДВИЖИМОСТИ', w_news2: 'ОТРАСЛЕВЫЕ НОВОСТИ', w_blogposts: 'Последние статьи', w_tags: 'Метки', w_contact: 'Контакты',
    blog_all: 'Все статьи', blog_empty: 'Статей пока нет.', read_more: 'Читать далее', related: 'Похожие статьи', by: 'Автор', latest_posts: 'Последние статьи', in_cat: 'Статьи в категории {0}',
    notfound_title: 'Страница не найдена', notfound_text: 'Возможно, страница была перемещена или удалена.', back_home: 'На главную', browse_listings: 'Смотреть объекты',
    maintenance: 'Технические работы', other_listings: 'Другие объекты', all_listings: 'Все объекты', see_all: 'Показать все',
    breadcrumb: 'Навигация', slide_prev: 'Назад', slide_next: 'Вперёд', slide_n: 'Слайд {0}', rights: 'Все права защищены.', followus: 'Мы в соцсетях',
    hero_cta: 'Смотреть объекты', hero_cta2: 'Связаться', views: 'Просмотры',
  },
};

export function t(lang, key, ...args) {
  let s = (D[lang] && D[lang][key]) ?? D.tr[key] ?? key;
  args.forEach((a, i) => { s = s.split(`{${i}}`).join(String(a ?? '')); });
  return s;
}
export const hasKey = (key) => key in D.tr;
export const i18nKeys = () => Object.keys(D.tr);
export const i18nMissing = (lang) => Object.keys(D.tr).filter((k) => !(k in (D[lang] || {})));
