//! PDF / DOCX tanıtım çıktısı ve WhatsApp metni.
use crate::common::*;
use crate::i18n_doc::{floor_text, lbl, lbl_nl, tr, Lang};
use std::io::Cursor;
use std::path::Path;

pub struct Agent {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub photo: Option<Vec<u8>>,
}

pub struct ExportData {
    pub p: Portfolio,
    pub photos: Vec<Vec<u8>>,
    pub settings: OutputSettings,
    pub agent: Option<Agent>,
    pub logo: Option<Vec<u8>>,
    pub lang: Lang,
}

pub const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
pub const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

pub fn detail_rows(p: &Portfolio, lg: Lang) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |k: &str, val: String| {
        let val = val.trim().to_string();
        if !val.is_empty() && val != "-" && val != "Belirtilmedi" {
            v.push((tr(lg, k), val));
        }
    };
    add("Durum", format!("{} {}", tr(lg, &p.listing_type), tr(lg, &p.category)));
    let loc = [p.district.as_str(), p.neighborhood.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" / ");
    add("Konum", if loc.is_empty() { String::new() } else { format!("Muğla / {loc}") });
    add("Brüt alan", fmt_m2(p.gross_m2));
    add("Net alan", fmt_m2(p.net_m2));
    add("Oda sayısı", tr(lg, &p.rooms));
    add("Bina yaşı", p.building_age.clone());
    let kat = floor_text(lg, p.floor.trim(), p.total_floors.trim());
    add("Kat", kat);
    add("Isıtma", tr(lg, &p.heating));
    add("Tapu durumu", tr(lg, &p.deed_status));
    if p.furnished {
        add("Eşya", tr(lg, "Eşyalı"));
    }
    if p.credit_ok {
        add("Kredi", tr(lg, "Krediye uygun"));
    }
    if p.swap_ok {
        add("Takas", tr(lg, "Takasa uygun"));
    }
    v
}

pub fn price_str(lg: Lang, price: f64, cur: &str) -> String {
    if price <= 0.0 { tr(lg, "Fiyat sorunuz") } else { fmt_price(price, cur) }
}

pub fn whatsapp_text(p: &Portfolio, s: &OutputSettings, agent: &Option<Agent>, lg: Lang) -> String {
    let mut t = String::new();
    t.push_str(&format!("*{}*\n", p.title));
    if s.show_listing_no {
        t.push_str(&lbl_nl(lg, "İlan No", &p.listing_no()));
    }
    t.push_str(&format!("{} {}\n", tr(lg, &p.listing_type), tr(lg, &p.category)));
    let loc = [p.district.as_str(), p.neighborhood.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" / ");
    if !loc.is_empty() {
        t.push_str(&format!("Konum: {loc}\n"));
    }
    if s.show_price {
        t.push_str(&lbl_nl(lg, "Fiyat", &price_str(lg, p.price, &p.currency)));
    }
    let mut spec = Vec::new();
    if p.gross_m2 > 0.0 {
        spec.push(fmt_m2(p.gross_m2));
    }
    if !p.rooms.is_empty() && p.rooms != "-" {
        spec.push(p.rooms.clone());
    }
    if !spec.is_empty() {
        t.push_str(&format!("{}\n", spec.join(" • ")));
    }
    if s.show_description && !p.description.trim().is_empty() {
        let d: String = p.description.trim().chars().take(350).collect();
        t.push_str(&format!("\n{}{}\n", d, if p.description.trim().chars().count() > 350 { "…" } else { "" }));
    }
    if s.show_map_link {
        if let (Some(la), Some(lo)) = (p.lat, p.lon) {
            t.push_str(&format!("\nKonum: {}\n", map_url(la, lo)));
        }
    }
    if let Some(a) = agent {
        t.push('\n');
        if s.show_agent_name && !a.name.is_empty() {
            t.push_str(&lbl_nl(lg, "Danışman", &a.name));
        }
        if s.show_agent_phone && !a.phone.is_empty() {
            t.push_str(&lbl_nl(lg, "Tel", &a.phone));
        }
        if s.show_agent_email && !a.email.is_empty() {
            t.push_str(&lbl_nl(lg, "E-posta", &a.email));
        }
    }
    if !s.office_name.trim().is_empty() {
        t.push_str(&format!("{}\n", s.office_name.trim()));
    }
    t
}

fn hex_rgb(h: &str) -> (u8, u8, u8) {
    let h = h.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&h[0..2], 16), u8::from_str_radix(&h[2..4], 16), u8::from_str_radix(&h[4..6], 16)) {
            return (r, g, b);
        }
    }
    (31, 78, 121)
}

fn img_dims(b: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(Cursor::new(b)).with_guessed_format().ok()?.into_dimensions().ok()
}

/// Logo PNG'si şeffaflık içerebilir; PDF için beyaz zemine yassıltıp JPEG yapar.
fn flatten_to_jpeg(b: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(b).ok()?;
    let rgba = img.to_rgba8();
    let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let m = |c: u8| (c as f32 * a + 255.0 * (1.0 - a)) as u8;
        rgb.put_pixel(x, y, image::Rgb([m(p[0]), m(p[1]), m(p[2])]));
    }
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    image::DynamicImage::ImageRgb8(rgb).write_with_encoder(enc).ok()?;
    Some(out)
}

pub fn write_pdf(d: &ExportData, path: &Path) -> Result<(), String> {
    use genpdf::elements::{Break, FrameCellDecorator, LinearLayout, Paragraph, TableLayout};
    use genpdf::style::{Color, Style};
    use genpdf::{fonts, Alignment, Document, Element, Margins, SimplePageDecorator};

    let s = &d.settings;
    let p = &d.p;
    let lg = d.lang;
    let (r, g, b) = hex_rgb(&s.accent);
    let accent = Color::Rgb(r, g, b);
    let gray = Color::Rgb(110, 110, 110);

    let regular = fonts::FontData::new(FONT_REGULAR.to_vec(), None).map_err(|e| e.to_string())?;
    let bold = fonts::FontData::new(FONT_BOLD.to_vec(), None).map_err(|e| e.to_string())?;
    let family = fonts::FontFamily { regular: regular.clone(), bold: bold.clone(), italic: regular, bold_italic: bold };
    let mut doc = Document::new(family);
    doc.set_title(p.title.clone());
    doc.set_font_size(10);
    let mut dec = SimplePageDecorator::new();
    dec.set_margins(Margins::trbl(14, 12, 14, 12));
    let office = s.office_name.clone();
    let page_word = tr(lg, "Sayfa");
    dec.set_header(move |page| {
        Paragraph::new(format!("{}   |   {} {}", office, page_word, page)).aligned(Alignment::Right).styled(Style::new().with_font_size(8).with_color(Color::Rgb(130, 130, 130)))
    });
    doc.set_page_decorator(dec);

    // Üst bilgi: logo + ofis adı
    let mut head = TableLayout::new(vec![1, 4]);
    let logo_el = if s.show_logo { d.logo.as_ref().and_then(|l| flatten_to_jpeg(l)) } else { None };
    let mut left = LinearLayout::vertical();
    if let Some(lj) = &logo_el {
        if let Some((w, _)) = img_dims(lj) {
            if let Ok(img) = genpdf::elements::Image::from_reader(Cursor::new(lj.clone())) {
                left.push(img.with_dpi(w as f64 * 25.4 / 30.0));
            }
        }
    }
    let mut right = LinearLayout::vertical();
    right.push(Paragraph::new(s.office_name.clone()).aligned(Alignment::Right).styled(Style::new().bold().with_font_size(15).with_color(accent)));
    if !s.tagline.trim().is_empty() {
        right.push(Paragraph::new(s.tagline.clone()).aligned(Alignment::Right).styled(Style::new().with_font_size(9).with_color(gray)));
    }
    if s.show_listing_no || s.show_date {
        let mut parts = Vec::new();
        if s.show_listing_no {
            parts.push(lbl(lg, "İlan No", &p.listing_no()));
        }
        if s.show_date {
            parts.push(chrono::Local::now().format("%d.%m.%Y").to_string());
        }
        right.push(Paragraph::new(parts.join("   •   ")).aligned(Alignment::Right).styled(Style::new().with_font_size(9).with_color(gray)));
    }
    head.row().element(left).element(right).push().map_err(|e| e.to_string())?;
    doc.push(head);
    doc.push(Break::new(1.0));

    // Başlık
    doc.push(Paragraph::new(p.title.clone()).styled(Style::new().bold().with_font_size(20).with_color(accent)));
    let loc = [p.district.as_str(), p.neighborhood.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" / ");
    doc.push(Paragraph::new(format!("{} {}{}", tr(lg, &p.listing_type), tr(lg, &p.category), if loc.is_empty() { String::new() } else { format!("  •  Muğla / {loc}") })).styled(Style::new().with_font_size(11).with_color(gray)));
    if s.show_price {
        doc.push(Break::new(0.5));
        doc.push(Paragraph::new(price_str(lg, p.price, &p.currency)).styled(Style::new().bold().with_font_size(18)));
    }
    doc.push(Break::new(0.8));

    // Fotoğraflar
    if s.show_photos && !d.photos.is_empty() {
        let maxp = s.max_photos.clamp(1, 30) as usize;
        let photos: Vec<&Vec<u8>> = d.photos.iter().take(maxp).collect();
        let place = |bytes: &Vec<u8>, max_w: f64, max_h: f64| -> Option<genpdf::elements::Image> {
            let (w, h) = img_dims(bytes)?;
            let aspect = w as f64 / h as f64;
            let target_w = max_w.min(max_h * aspect);
            let img = genpdf::elements::Image::from_reader(Cursor::new(bytes.clone())).ok()?;
            Some(img.with_dpi(w as f64 * 25.4 / target_w))
        };
        if let Some(img) = place(photos[0], 184.0, 105.0) {
            doc.push(img.with_alignment(Alignment::Center));
            doc.push(Break::new(0.6));
        }
        let rest = &photos[1..];
        for pair in rest.chunks(2) {
            let mut t = TableLayout::new(vec![1, 1]);
            let mut row = t.row();
            for i in 0..2 {
                if let Some(bytes) = pair.get(i) {
                    if let Some(img) = place(bytes, 90.0, 64.0) {
                        row = row.element(img.with_alignment(Alignment::Center).padded(1));
                        continue;
                    }
                }
                row = row.element(Paragraph::new("").padded(1));
            }
            row.push().map_err(|e| e.to_string())?;
            doc.push(t);
        }
        doc.push(Break::new(0.6));
    }

    // Özellik tablosu
    if s.show_details {
        let rows = detail_rows(p, lg);
        if !rows.is_empty() {
            doc.push(Paragraph::new(tr(lg, "Özellikler")).styled(Style::new().bold().with_font_size(13).with_color(accent)));
            doc.push(Break::new(0.3));
            let mut t = TableLayout::new(vec![2, 5]);
            t.set_cell_decorator(FrameCellDecorator::new(true, true, false));
            for (k, v) in rows {
                t.row()
                    .element(Paragraph::new(k).styled(Style::new().bold()).padded(1.5))
                    .element(Paragraph::new(v).padded(1.5))
                    .push()
                    .map_err(|e| e.to_string())?;
            }
            doc.push(t);
            doc.push(Break::new(0.8));
        }
    }

    if s.show_address && !p.address.trim().is_empty() {
        doc.push(Paragraph::new(lbl(lg, "Adres", &p.address.trim())).styled(Style::new().with_font_size(10)));
        doc.push(Break::new(0.4));
    }

    if s.show_description && !p.description.trim().is_empty() {
        doc.push(Paragraph::new(tr(lg, "Açıklama")).styled(Style::new().bold().with_font_size(13).with_color(accent)));
        doc.push(Break::new(0.3));
        for line in p.description.trim().lines() {
            doc.push(Paragraph::new(line.to_string()));
        }
        doc.push(Break::new(0.8));
    }

    if s.show_map_link {
        if let (Some(la), Some(lo)) = (p.lat, p.lon) {
            doc.push(Paragraph::new(lbl(lg, "Konum (harita)", &map_url(la, lo))).styled(Style::new().with_font_size(9).with_color(accent)));
            doc.push(Break::new(0.8));
        }
    }

    // İletişim bloğu (belge altı)
    let mut contact_lines: Vec<(String, bool)> = Vec::new();
    let mut agent_photo: Option<Vec<u8>> = None;
    if let Some(a) = &d.agent {
        if s.show_agent_name && !a.name.is_empty() {
            contact_lines.push((a.name.clone(), true));
        }
        if s.show_agent_phone && !a.phone.is_empty() {
            contact_lines.push((lbl(lg, "Tel", &a.phone), false));
        }
        if s.show_agent_email && !a.email.is_empty() {
            contact_lines.push((lbl(lg, "E-posta", &a.email), false));
        }
        if s.show_agent_photo {
            agent_photo = a.photo.clone();
        }
    }
    if s.show_office_contact {
        if !s.office_name.trim().is_empty() && !contact_lines.is_empty() {
            contact_lines.push((s.office_name.clone(), false));
        }
        if !s.office_phone.trim().is_empty() {
            contact_lines.push((lbl(lg, "Ofis", &s.office_phone), false));
        }
        if !s.office_address.trim().is_empty() {
            contact_lines.push((s.office_address.clone(), false));
        }
    }
    if !contact_lines.is_empty() || agent_photo.is_some() {
        doc.push(Paragraph::new(tr(lg, "İletişim")).styled(Style::new().bold().with_font_size(13).with_color(accent)));
        doc.push(Break::new(0.3));
        let mut t = TableLayout::new(vec![1, 5]);
        t.set_cell_decorator(FrameCellDecorator::new(false, true, false));
        let mut photo_cell = LinearLayout::vertical();
        if let Some(ph) = &agent_photo {
            if let (Some(pj), Some((w, _))) = (flatten_to_jpeg(ph), img_dims(ph)) {
                if let Ok(img) = genpdf::elements::Image::from_reader(Cursor::new(pj)) {
                    photo_cell.push(img.with_dpi(w as f64 * 25.4 / 26.0).padded(1.5));
                }
            }
        }
        let mut text_cell = LinearLayout::vertical();
        for (line, strong) in contact_lines {
            let st = if strong { Style::new().bold().with_font_size(13) } else { Style::new().with_font_size(10) };
            text_cell.push(Paragraph::new(line).styled(st).padded(Margins::trbl(0.6, 1, 0.6, 2)));
        }
        t.row().element(photo_cell).element(text_cell).push().map_err(|e| e.to_string())?;
        doc.push(t);
    }

    if !s.footer_note.trim().is_empty() {
        doc.push(Break::new(1.0));
        doc.push(Paragraph::new(s.footer_note.clone()).aligned(Alignment::Center).styled(Style::new().italic().with_font_size(8).with_color(gray)));
    }

    doc.render_to_file(path).map_err(|e| format!("PDF yazılamadı: {e}"))
}

pub fn write_docx(d: &ExportData, path: &Path) -> Result<(), String> {
    use docx_rs::*;
    let s = &d.settings;
    let p = &d.p;
    let lg = d.lang;
    let (r, g, b) = hex_rgb(&s.accent);
    let accent = format!("{:02X}{:02X}{:02X}", r, g, b);
    let cm = |v: f64| (v * 360000.0) as u32;

    let pic = |bytes: &[u8], max_w_cm: f64, max_h_cm: f64| -> Option<Pic> {
        let (w, h) = img_dims(bytes)?;
        let aspect = w as f64 / h as f64;
        let tw = max_w_cm.min(max_h_cm * aspect);
        let th = tw / aspect;
        Some(Pic::new(bytes).size(cm(tw), cm(th)))
    };
    let text = |t: &str, size: usize, bold: bool, color: &str| -> Run {
        let mut run = Run::new().add_text(t).size(size).color(color);
        if bold {
            run = run.bold();
        }
        run
    };

    let mut docx = Docx::new().page_margin(PageMargin::new().top(700).bottom(700).left(800).right(800));

    // Üst bilgi
    if s.show_logo {
        if let Some(l) = &d.logo {
            if let Some(pc) = pic(l, 4.0, 2.5) {
                docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pc)));
            }
        }
    }
    docx = docx.add_paragraph(Paragraph::new().add_run(text(&s.office_name, 30, true, &accent)));
    if !s.tagline.trim().is_empty() {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&s.tagline, 18, false, "666666")));
    }
    let mut meta = Vec::new();
    if s.show_listing_no {
        meta.push(lbl(lg, "İlan No", &p.listing_no()));
    }
    if s.show_date {
        meta.push(chrono::Local::now().format("%d.%m.%Y").to_string());
    }
    if !meta.is_empty() {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&meta.join("   •   "), 18, false, "666666")));
    }

    docx = docx.add_paragraph(Paragraph::new().add_run(text(&p.title, 40, true, &accent)));
    let loc = [p.district.as_str(), p.neighborhood.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" / ");
    docx = docx.add_paragraph(Paragraph::new().add_run(text(
        &format!("{} {}{}", tr(lg, &p.listing_type), tr(lg, &p.category), if loc.is_empty() { String::new() } else { format!("  •  Muğla / {loc}") }),
        22,
        false,
        "666666",
    )));
    if s.show_price {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&price_str(lg, p.price, &p.currency), 36, true, "000000")));
    }

    if s.show_photos && !d.photos.is_empty() {
        let maxp = s.max_photos.clamp(1, 30) as usize;
        let photos: Vec<&Vec<u8>> = d.photos.iter().take(maxp).collect();
        if let Some(pc) = pic(photos[0], 17.0, 10.0) {
            docx = docx.add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(Run::new().add_image(pc)));
        }
        for pair in photos[1..].chunks(2) {
            let mut para = Paragraph::new().align(AlignmentType::Center);
            for bytes in pair {
                if let Some(pc) = pic(bytes, 8.4, 6.0) {
                    para = para.add_run(Run::new().add_image(pc)).add_run(Run::new().add_text("  "));
                }
            }
            docx = docx.add_paragraph(para);
        }
    }

    if s.show_details {
        let rows = detail_rows(p, lg);
        if !rows.is_empty() {
            docx = docx.add_paragraph(Paragraph::new().add_run(text(&tr(lg, "Özellikler"), 26, true, &accent)));
            let trs: Vec<TableRow> = rows
                .into_iter()
                .map(|(k, v)| {
                    TableRow::new(vec![
                        TableCell::new().width(2600, WidthType::Dxa).add_paragraph(Paragraph::new().add_run(text(&k, 20, true, "000000"))),
                        TableCell::new().width(7400, WidthType::Dxa).add_paragraph(Paragraph::new().add_run(text(&v, 20, false, "000000"))),
                    ])
                })
                .collect();
            docx = docx.add_table(Table::new(trs).set_grid(vec![2600, 7400]));
        }
    }
    if s.show_address && !p.address.trim().is_empty() {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&lbl(lg, "Adres", &p.address.trim()), 20, false, "000000")));
    }
    if s.show_description && !p.description.trim().is_empty() {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&tr(lg, "Açıklama"), 26, true, &accent)));
        for line in p.description.trim().lines() {
            docx = docx.add_paragraph(Paragraph::new().add_run(text(line, 20, false, "000000")));
        }
    }
    if s.show_map_link {
        if let (Some(la), Some(lo)) = (p.lat, p.lon) {
            docx = docx.add_paragraph(Paragraph::new().add_run(text(&lbl(lg, "Konum (harita)", &map_url(la, lo)), 18, false, &accent)));
        }
    }

    // İletişim
    let mut lines: Vec<(String, bool)> = Vec::new();
    let mut photo: Option<&Vec<u8>> = None;
    if let Some(a) = &d.agent {
        if s.show_agent_name && !a.name.is_empty() {
            lines.push((a.name.clone(), true));
        }
        if s.show_agent_phone && !a.phone.is_empty() {
            lines.push((lbl(lg, "Tel", &a.phone), false));
        }
        if s.show_agent_email && !a.email.is_empty() {
            lines.push((lbl(lg, "E-posta", &a.email), false));
        }
        if s.show_agent_photo {
            photo = a.photo.as_ref();
        }
    }
    if s.show_office_contact {
        if !s.office_name.trim().is_empty() && !lines.is_empty() {
            lines.push((s.office_name.clone(), false));
        }
        if !s.office_phone.trim().is_empty() {
            lines.push((lbl(lg, "Ofis", &s.office_phone), false));
        }
        if !s.office_address.trim().is_empty() {
            lines.push((s.office_address.clone(), false));
        }
    }
    if !lines.is_empty() || photo.is_some() {
        docx = docx.add_paragraph(Paragraph::new().add_run(text(&tr(lg, "İletişim"), 26, true, &accent)));
        let mut photo_cell = TableCell::new().width(1800, WidthType::Dxa);
        match photo.and_then(|ph| pic(ph, 2.8, 3.2)) {
            Some(pc) => photo_cell = photo_cell.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pc))),
            None => photo_cell = photo_cell.add_paragraph(Paragraph::new()),
        }
        let mut text_cell = TableCell::new().width(8200, WidthType::Dxa);
        for (l, strong) in lines {
            text_cell = text_cell.add_paragraph(Paragraph::new().add_run(text(&l, if strong { 26 } else { 20 }, strong, "000000")));
        }
        docx = docx.add_table(Table::new(vec![TableRow::new(vec![photo_cell, text_cell])]).set_grid(vec![1800, 8200]));
    }
    if !s.footer_note.trim().is_empty() {
        docx = docx.add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(text(&s.footer_note, 16, false, "888888").italic()));
    }

    let file = std::fs::File::create(path).map_err(|e| format!("Dosya oluşturulamadı: {e}"))?;
    docx.build().pack(file).map_err(|e| format!("DOCX yazılamadı: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(col: [u8; 3]) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(1200, 800, image::Rgb(col));
        let mut out = Vec::new();
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
        image::DynamicImage::ImageRgb8(img).write_with_encoder(enc).unwrap();
        out
    }

    #[test]
    fn pdf_ve_docx_uretilir() {
        let mut p = Portfolio::new_blank();
        p.id = 7;
        p.title = "Ölüdeniz'de deniz manzaralı villa ğüşıİÖÇ".into();
        p.district = "Fethiye".into();
        p.neighborhood = "Ölüdeniz".into();
        p.price = 12_500_000.0;
        p.gross_m2 = 220.0;
        p.rooms = "4+1".into();
        p.lat = Some(36.55);
        p.lon = Some(29.12);
        p.description = "Çok güzel bir villa. Işıklı, şık, ğüşiöç.".into();
        p.listing_type = "Satılık".into();
        p.category = "Villa".into();
        p.heating = "Klima".into();
        p.floor = "2".into();
        p.total_floors = "3".into();
        let p2 = p.clone();
        let d = ExportData {
            p,
            photos: vec![jpeg([200, 30, 30]), jpeg([30, 200, 30]), jpeg([30, 30, 200])],
            lang: Lang::Tr,
            settings: OutputSettings::default(),
            agent: Some(Agent { name: "Ayşe Çalışan".into(), phone: "0532 123 45 67".into(), email: "ayse@x.com".into(), photo: Some(jpeg([90, 90, 90])) }),
            logo: None,
        };
        let dir = std::env::temp_dir().join("emlak_export_test");
        std::fs::create_dir_all(&dir).unwrap();
        write_pdf(&d, &dir.join("t.pdf")).unwrap();
        write_docx(&d, &dir.join("t.docx")).unwrap();
        assert!(std::fs::metadata(dir.join("t.pdf")).unwrap().len() > 5000);
        assert!(std::fs::metadata(dir.join("t.docx")).unwrap().len() > 5000);

        // Çok dilli çıktı: her dilde PDF/DOCX üretilir, etiketler çevrilir.
        for l in Lang::ALL {
            let d2 = ExportData { p: p2.clone(), photos: vec![], lang: l, settings: OutputSettings::default(), agent: None, logo: None };
            write_pdf(&d2, &dir.join(format!("t_{}.pdf", l.code()))).unwrap();
            write_docx(&d2, &dir.join(format!("t_{}.docx", l.code()))).unwrap();
        }
        let en = detail_rows(&p2, Lang::En);
        assert!(en.iter().any(|(k, v)| k == "Gross area" && v == "220 m²"));
        assert!(en.iter().any(|(k, v)| k == "Heating" && v == "Air conditioning"));
        assert!(en.iter().any(|(k, v)| k == "Floor" && v == "Floor 2 of 3"));
        let de = whatsapp_text(&p2, &OutputSettings::default(), &None, Lang::De);
        assert!(de.contains("Zu verkaufen Villa") && de.contains("Preis: 12"));
        let ru = whatsapp_text(&p2, &OutputSettings::default(), &None, Lang::Ru);
        assert!(ru.contains("Продаётся Вилла") && ru.contains("Цена:"));
        let tr_ = whatsapp_text(&p2, &OutputSettings::default(), &None, Lang::Tr);
        assert!(tr_.contains("Satılık Villa") && tr_.contains("Fiyat:"));
        assert_eq!(Lang::from_code("xx"), Lang::Tr);
    }
}
