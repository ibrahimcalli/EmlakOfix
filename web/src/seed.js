// İlk çalıştırmada varsayılan menü, sayfa, etiket, kategori, banner ve yazıları ekler (tek seferlik, yarışa dayanıklı).
import { run, first, scalar, batch } from './env.js';
import { slugify, nowIso } from './util.js';
import { sanitizeHtml } from './sanitize.js';
import POSTS from '../seed/posts.js';

export const SEED_MENU = [
  { label: 'ANA SAYFA', en: 'HOME', ru: 'ГЛАВНАЯ', kind: 'home', target: '', slug: '' },
  { label: 'SATILIK VİLLA', en: 'VILLAS FOR SALE', ru: 'ВИЛЛЫ НА ПРОДАЖУ', kind: 'listings', target: 'Satılık|Villa', slug: 'satilik-villa' },
  { label: 'SATILIK DAİRE', en: 'APARTMENTS FOR SALE', ru: 'КВАРТИРЫ НА ПРОДАЖУ', kind: 'listings', target: 'Satılık|Daire', slug: 'satilik-daire' },
  { label: 'SATILIK ARAZİ', en: 'LAND FOR SALE', ru: 'ЗЕМЕЛЬНЫЕ УЧАСТКИ', kind: 'listings', target: 'Satılık|Arazi', slug: 'satilik-arazi' },
  { label: 'KİRALIK VİLLA', en: 'VILLAS FOR RENT', ru: 'ВИЛЛЫ В АРЕНДУ', kind: 'listings', target: 'Kiralık|Villa', slug: 'kiralik-villa' },
  { label: 'KİRALIK DAİRE', en: 'APARTMENTS FOR RENT', ru: 'КВАРТИРЫ В АРЕНДУ', kind: 'listings', target: 'Kiralık|Daire', slug: 'kiralik-daire' },
  { label: 'DİĞER İLANLAR', en: 'OTHER LISTINGS', ru: 'ДРУГИЕ ОБЪЯВЛЕНИЯ', kind: 'listings', target: '*|*', slug: 'diger-ilanlar' },
  { label: 'BLOG', en: 'BLOG', ru: 'БЛОГ', kind: 'blog', target: '', slug: 'blog' },
  { label: 'HAKKIMIZDA', en: 'ABOUT US', ru: 'О НАС', kind: 'page', target: 'hakkimizda', slug: 'hakkimizda' },
  { label: 'İLETİŞİM', en: 'CONTACT', ru: 'КОНТАКТЫ', kind: 'page', target: 'iletisim', slug: 'iletisim' },
];
export const SEED_TAGS = [
  ['Müstakil', 'Detached', 'Отдельный дом', '#2f6f4f'],
  ['Villa', 'Villa', 'Вилла', '#1b2a41'],
  ['Fırsat', 'Bargain', 'Выгодное предложение', '#c0392b'],
  ['Yeni', 'New', 'Новое', '#2980b9'],
  ['Deniz Manzaralı', 'Sea View', 'Вид на море', '#0e86a8'],
  ['Eşyalı', 'Furnished', 'С мебелью', '#8e6b3e'],
  ['Havuzlu', 'With Pool', 'С бассейном', '#17a2b8'],
  ['Yatırımlık', 'Investment', 'Для инвестиций', '#d4af37'],
  ['Takas', 'Swap', 'Обмен', '#7d3c98'],
  ['Krediye Uygun', 'Mortgage Eligible', 'Подходит для ипотеки', '#27ae60'],
];
export const SEED_CATEGORIES = [
  { name: 'ARAZİ ÇEŞİTLERİ VE İMAR DURUMLARI', en: 'LAND TYPES AND ZONING STATUS', ru: 'ВИДЫ ЗЕМЛИ И СТАТУС ЗОНИРОВАНИЯ', kind: 'blog' },
  { name: 'GAYRİMENKUL SATIN ALMA SÜREÇLERİ', en: 'REAL ESTATE PURCHASE PROCESSES', ru: 'ПРОЦЕСС ПОКУПКИ НЕДВИЖИМОСТИ', kind: 'blog' },
  { name: 'HİSSELİ ARAZİLERLE İLGİLİ YAZILAR', en: 'ARTICLES ON SHARED-TITLE LAND', ru: 'СТАТЬИ О ЗЕМЛЕ В ДОЛЕВОЙ СОБСТВЕННОСТИ', kind: 'blog' },
  { name: 'SEKTÖRE DAİR ÇIKAN HABERLER', en: 'INDUSTRY NEWS', ru: 'НОВОСТИ ОТРАСЛИ', kind: 'news' },
];
const PAGES = [
  {
    slug: 'hakkimizda', title: 'Hakkımızda', title_en: 'About Us', title_ru: 'О нас',
    body: '<p>Sahibinden Fethiye, Fethiye ve çevresinde satılık ve kiralık villa, daire ve arazi ilanlarıyla hizmet veren bir gayrimenkul danışmanlık platformudur.</p><h2>Neler yapıyoruz?</h2><ul><li>Satılık ve kiralık portföy danışmanlığı</li><li>Arazi ve imar durumu bilgilendirmesi</li><li>Tapu ve satın alma sürecinde yönlendirme</li></ul><p>Her müşterimize dürüst, şeffaf ve güvenilir bir hizmet sunmayı amaçlıyoruz. Bize telefonla, WhatsApp ile veya iletişim formundan ulaşabilirsiniz.</p>',
    body_en: '<p>Sahibinden Fethiye is a real estate consultancy platform offering villas, apartments and land for sale and rent in Fethiye and its surroundings.</p><h2>What we do</h2><ul><li>Sales and rental portfolio consultancy</li><li>Land and zoning information</li><li>Guidance through the title deed and purchase process</li></ul><p>We aim to provide every client with honest, transparent and reliable service. You can reach us by phone, WhatsApp or the contact form.</p>',
    body_ru: '<p>Sahibinden Fethiye — консультационная платформа по недвижимости: виллы, квартиры и земельные участки на продажу и в аренду в Фетхие и окрестностях.</p><h2>Чем мы занимаемся</h2><ul><li>Консультации по объектам для продажи и аренды</li><li>Информация о земле и зонировании</li><li>Сопровождение оформления права собственности и покупки</li></ul><p>Мы стремимся предоставлять каждому клиенту честный, прозрачный и надёжный сервис. Свяжитесь с нами по телефону, WhatsApp или через форму.</p>',
  },
  {
    slug: 'iletisim', title: 'İletişim', title_en: 'Contact', title_ru: 'Контакты',
    body: '<p>Sorularınız, ilan talepleriniz ve randevu için bize ulaşın. Aşağıdaki formu doldurabilir veya telefon ve WhatsApp ile arayabilirsiniz.</p><p>Adres: Fethiye / Muğla</p>',
    body_en: '<p>Contact us with your questions, listing requests and appointments. Fill in the form below or call us by phone or WhatsApp.</p><p>Address: Fethiye / Muğla, Turkey</p>',
    body_ru: '<p>Свяжитесь с нами по вопросам, заявкам на объекты и для записи на встречу. Заполните форму ниже или позвоните нам по телефону или в WhatsApp.</p><p>Адрес: Фетхие / Мугла, Турция</p>',
  },
];

const seeded = new WeakSet(); // env başına: bu izolede kontrol bir kez yapılsın

const BANNERS_V2 = [
  ['/assets/banner/fethiye-evler.jpg', "Fethiye'de hayalinizdeki ev", 'Satılık ve kiralık villa, daire ve arazi ilanları', '/satilik-villa', 'İlanları İncele'],
  ['/assets/banner/arazi-gunbatimi.jpg', 'Arazi ve yatırım fırsatları', 'Fethiye ve çevresinde satılık arazi ilanları', '/satilik-arazi', 'Arazileri Gör'],
  ['/assets/banner/sehir-aksami.jpg', 'Güvenilir emlak danışmanlığı', 'Alım, satım ve kiralamada yanınızdayız', '/iletisim', 'Bize Ulaşın'],
];
// Tek seferlik: ana sayfa banner gorselleri (gorselsiz mevcut banner'a gorsel, eksikler eklenir). Yonetici silerse geri gelmez.
async function ensureV2(env) {
  if (await first(env, "SELECT value FROM settings WHERE key='banners_v2'")) return;
  const claim = await run(env, "INSERT OR IGNORE INTO settings(key,value) VALUES('banners_v2',?)", nowIso());
  if (claim && claim.meta && claim.meta.changes !== 1) return;
  const st = [];
  const n = Number(await scalar(env, 'SELECT COUNT(*) FROM banners')) || 0;
  const bare = await first(env, "SELECT id FROM banners WHERE image='' ORDER BY id LIMIT 1");
  if (bare) st.push(["UPDATE banners SET image=? WHERE id=?", BANNERS_V2[0][0], bare.id]);
  if (n <= 1) BANNERS_V2.slice(bare ? 1 : 0).forEach((b, i) => st.push(['INSERT INTO banners(title,subtitle,image,link,button,sort_key,visible) VALUES(?,?,?,?,?,?,1)', b[1], b[2], b[0], b[3], b[4], 2 + i]));
  if (st.length) await batch(env, st);
}

export async function ensureSeed(env) {
  if (seeded.has(env)) return { seeded: false };
  const done = await first(env, "SELECT value FROM settings WHERE key='seeded'");
  if (done) { await ensureV2(env); seeded.add(env); return { seeded: false }; }
  // Yarış: yalnız 'seeded' satırını ilk ekleyen (changes=1) tohumlar.
  const claim = await run(env, "INSERT OR IGNORE INTO settings(key,value) VALUES('seeded',?)", nowIso());
  const won = claim && claim.meta ? claim.meta.changes === 1 : true;
  seeded.add(env);
  if (!won) return { seeded: false };
  try {
    const now = nowIso();
    const st = [];
    if (!Number(await scalar(env, 'SELECT COUNT(*) FROM menu_items'))) {
      SEED_MENU.forEach((m, i) => st.push(['INSERT INTO menu_items(menu,parent_id,label,label_en,label_ru,kind,target,slug,sort_key,visible) VALUES(\'main\',0,?,?,?,?,?,?,?,1)', m.label, m.en, m.ru, m.kind, m.target, m.slug, (i + 1) * 10]));
    }
    for (const p of PAGES) st.push(['INSERT OR IGNORE INTO pages(slug,title,title_en,title_ru,body,body_en,body_ru,visible,updated_at) VALUES(?,?,?,?,?,?,?,1,?)', p.slug, p.title, p.title_en, p.title_ru, sanitizeHtml(p.body), sanitizeHtml(p.body_en), sanitizeHtml(p.body_ru), now]);
    SEED_TAGS.forEach(([n, en, ru, c], i) => st.push(['INSERT OR IGNORE INTO tags(name,name_en,name_ru,color,sort_key) VALUES(?,?,?,?,?)', n, en, ru, c, i + 1]));
    SEED_CATEGORIES.forEach((c, i) => st.push(['INSERT OR IGNORE INTO blog_categories(slug,name,name_en,name_ru,kind,sort_key) VALUES(?,?,?,?,?,?)', slugify(c.name, 80), c.name, c.en, c.ru, c.kind, i + 1]));
    if (!Number(await scalar(env, 'SELECT COUNT(*) FROM banners'))) {
      st.push(['INSERT INTO banners(title,subtitle,image,link,button,sort_key,visible) VALUES(?,?,?,?,?,1,1)', "Fethiye'de hayalinizdeki ev", 'Satılık ve kiralık villa, daire ve arazi ilanları', '', '/satilik-villa', 'İlanları İncele']);
    }
    for (const p of POSTS) {
      const cat = SEED_CATEGORIES.find((c) => c.name === p.category);
      const body = sanitizeHtml(p.body);
      st.push(['INSERT OR IGNORE INTO posts(slug,category_id,title,excerpt,body,image,status,author,source,published_at,updated_at) VALUES(?,COALESCE((SELECT id FROM blog_categories WHERE slug=?),0),?,?,?,\'\',\'yayinda\',?,\'import\',?,?)', p.slug, slugify(cat ? cat.name : p.category, 80), p.title, p.excerpt, body, 'İbrahim Çallı', p.published_at || now, now]);
    }
    await batch(env, st);
    await ensureV2(env);
    return { seeded: true };
  } catch (e) {
    try { await run(env, "DELETE FROM settings WHERE key='seeded'"); } catch { /* yoksay */ }
    seeded.delete(env);
    throw e;
  }
}
