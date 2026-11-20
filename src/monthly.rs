//! Aylık ofis raporu (DOCX / PDF). Veriler sunucudaki /api/reports/monthly yanıtından gelir.
use crate::common::OutputSettings;
use serde_json::Value;
use std::io::Cursor;
use std::path::Path;

pub enum Block {
    Title(String),
    Sub(String),
    H(String),
    P(String),
    Kpi(Vec<(String, String)>),
    Table(Vec<String>, Vec<Vec<String>>),
}

fn num(v: &Value) -> String {
    let n = v.as_f64().unwrap_or(0.0);
    let s = format!("{:.0}", n.abs());
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    format!("{}{}", if n < 0.0 { "-" } else { "" }, out.chars().rev().collect::<String>())
}

pub fn month_name(ym: &str) -> String {
    let names = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];
    let (y, m) = ym.split_once('-').unwrap_or((ym, "1"));
    let mi: usize = m.parse().unwrap_or(1);
    format!("{} {}", names.get(mi.wrapping_sub(1)).copied().unwrap_or("?"), y)
}

/// Rapor verisini sayfa bloklarına çevirir.
pub fn blocks_for(r: &Value, office: &str) -> Vec<Block> {
    let ym = r["ym"].as_str().unwrap_or("");
    let mut b = vec![Block::Title(format!("Aylık Ofis Raporu — {}", month_name(ym))), Block::Sub(format!("{office}  •  Hazırlanma: {}", chrono::Local::now().format("%d.%m.%Y")))];
    b.push(Block::Kpi(vec![
        ("Yeni portföy".into(), num(&r["new_total"])),
        ("Yayına alınan".into(), num(&r["published"])),
        ("Satılan".into(), num(&r["sold"])),
        ("Kiralanan".into(), num(&r["rented"])),
    ]));
    let list = |k: &str| r[k].as_array().cloned().unwrap_or_default();
    let bt = list("by_type");
    if !bt.is_empty() {
        b.push(Block::H("Yeni portföyler — türe göre".into()));
        b.push(Block::Table(vec!["Tür".into(), "Adet".into()], bt.iter().map(|x| vec![x["name"].as_str().unwrap_or("").into(), num(&x["n"])]).collect()));
    }
    let bd = list("by_district");
    if !bd.is_empty() {
        b.push(Block::H("Yeni portföyler — ilçeye göre".into()));
        b.push(Block::Table(vec!["İlçe".into(), "Adet".into()], bd.iter().map(|x| vec![x["name"].as_str().unwrap_or("").into(), num(&x["n"])]).collect()));
    }
    let cm = list("commissions");
    b.push(Block::H("Komisyon".into()));
    if cm.is_empty() {
        b.push(Block::P("Bu ay kayıtlı komisyon yok.".into()));
    } else {
        b.push(Block::Table(
            vec!["Para birimi".into(), "İşlem".into(), "İşlem tutarı".into(), "Komisyon".into()],
            cm.iter().map(|x| vec![x["currency"].as_str().unwrap_or("").into(), num(&x["n"]), num(&x["deal"]), num(&x["commission"])]).collect(),
        ));
        let cs = list("commission_status");
        if !cs.is_empty() {
            b.push(Block::Table(vec!["Durum".into(), "Adet".into(), "Komisyon".into()], cs.iter().map(|x| vec![x["status"].as_str().unwrap_or("").into(), num(&x["n"]), num(&x["commission"])]).collect()));
        }
    }
    let ap = list("appointments");
    if !ap.is_empty() {
        b.push(Block::H("Randevular".into()));
        b.push(Block::Table(vec!["Durum".into(), "Adet".into()], ap.iter().map(|x| vec![x["status"].as_str().unwrap_or("").into(), num(&x["n"])]).collect()));
    }
    let us = list("users");
    b.push(Block::H("Çalışan bazlı özet".into()));
    if us.is_empty() {
        b.push(Block::P("Bu ay için çalışan hareketi bulunmuyor.".into()));
    } else {
        b.push(Block::Table(
            vec!["Çalışan".into(), "Yeni portföy".into(), "Kapanan".into(), "Randevu".into(), "Mesai günü".into(), "Saat".into(), "Prim".into()],
            us.iter()
                .map(|x| vec![x["name"].as_str().unwrap_or("").into(), num(&x["new"]), num(&x["closed"]), num(&x["appointments"]), num(&x["days"]), format!("{:.1}", x["hours"].as_f64().unwrap_or(0.0)), num(&x["commission"])])
                .collect(),
        ));
    }
    b.push(Block::P("Bu rapor program kayıtlarından otomatik üretilmiştir. Kapanan = portföy durumunun Satıldı/Kiralandı olduğu ay; prim = çalışana ayrılan komisyon payı.".into()));
    b
}

fn hex(h: &str) -> (u8, u8, u8) {
    let h = h.trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return ((v >> 16) as u8, (v >> 8) as u8, v as u8);
        }
    }
    (31, 78, 121)
}

pub fn blocks(r: &Value) -> Vec<Block> {
    blocks_for(r, "")
}

pub fn write_docx(r: &Value, s: &OutputSettings, logo: Option<&[u8]>, path: &Path) -> Result<(), String> {
    use docx_rs::*;
    let blocks = blocks_for(r, &s.office_name);
    let (r, g, bl) = hex(&s.accent);
    let acc = format!("{r:02X}{g:02X}{bl:02X}");
    let mut d = Docx::new().page_margin(PageMargin::new().top(800).bottom(800).left(900).right(900));
    let run = |t: &str, size: usize, bold: bool, col: &str| {
        let x = Run::new().add_text(t).size(size).color(col);
        if bold { x.bold() } else { x }
    };
    if s.letterhead {
        for _ in 0..5 {
            d = d.add_paragraph(Paragraph::new().add_run(Run::new().add_text(" ")));
        }
    } else {
        if let (true, Some(l)) = (s.show_logo, logo) {
            if let Ok(img) = image::load_from_memory(l) {
                let (w, h) = (img.width() as f64, img.height() as f64);
                let tw = 4.0f64.min(2.5 * w / h);
                let cm = |v: f64| (v * 360000.0) as u32;
                d = d.add_paragraph(Paragraph::new().add_run(Run::new().add_image(Pic::new(l).size(cm(tw), cm(tw * h / w)))));
            }
        }
        d = d.add_paragraph(Paragraph::new().add_run(run(&s.office_name, 28, true, &acc)));
    }
    for bk in &blocks {
        match bk {
            Block::Title(t) => d = d.add_paragraph(Paragraph::new().add_run(run(t, 38, true, &acc))),
            Block::Sub(t) => d = d.add_paragraph(Paragraph::new().add_run(run(t, 18, false, "666666"))),
            Block::H(t) => d = d.add_paragraph(Paragraph::new().add_run(run(t, 26, true, &acc))),
            Block::P(t) => d = d.add_paragraph(Paragraph::new().add_run(run(t, 20, false, "444444"))),
            Block::Kpi(items) => {
                let cells: Vec<TableCell> = items
                    .iter()
                    .map(|(k, v)| TableCell::new().add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(run(v, 40, true, &acc))).add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(run(k, 18, false, "666666"))))
                    .collect();
                d = d.add_table(Table::new(vec![TableRow::new(cells)]));
            }
            Block::Table(head, rows) => {
                let mut trs = vec![TableRow::new(head.iter().map(|h| TableCell::new().shading(Shading::new().fill(&acc)).add_paragraph(Paragraph::new().add_run(run(h, 18, true, "FFFFFF")))).collect())];
                for r in rows {
                    trs.push(TableRow::new(r.iter().map(|c| TableCell::new().add_paragraph(Paragraph::new().add_run(run(c, 18, false, "222222")))).collect()));
                }
                d = d.add_table(Table::new(trs));
                d = d.add_paragraph(Paragraph::new());
            }
        }
    }
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    d.build().pack(f).map_err(|e| e.to_string())
}

pub fn write_pdf(r_: &Value, s: &OutputSettings, logo: Option<&[u8]>, path: &Path) -> Result<(), String> {
    let blocks = blocks_for(r_, &s.office_name);
    use genpdf::elements::{Break, FrameCellDecorator, Paragraph, TableLayout};
    use genpdf::style::{Color, Style};
    use genpdf::{fonts, Alignment, Document, Element, Margins, SimplePageDecorator};
    let (r, g, b) = hex(&s.accent);
    let acc = Color::Rgb(r, g, b);
    let reg = fonts::FontData::new(crate::export::FONT_REGULAR.to_vec(), None).map_err(|e| e.to_string())?;
    let bold = fonts::FontData::new(crate::export::FONT_BOLD.to_vec(), None).map_err(|e| e.to_string())?;
    let mut doc = Document::new(fonts::FontFamily { regular: reg.clone(), bold: bold.clone(), italic: reg, bold_italic: bold });
    doc.set_title("Aylık Ofis Raporu");
    doc.set_font_size(10);
    let mut dec = SimplePageDecorator::new();
    dec.set_margins(Margins::trbl(14, 14, 14, 14));
    let (office, lh) = (s.office_name.clone(), s.letterhead);
    dec.set_header(move |p| Paragraph::new(if lh { format!("Sayfa {p}") } else { format!("{office}   |   Sayfa {p}") }).aligned(Alignment::Right).styled(Style::new().with_font_size(8).with_color(Color::Rgb(130, 130, 130))));
    doc.set_page_decorator(dec);
    if s.letterhead {
        doc.push(Break::new(7.0));
    } else {
        if let (true, Some(l)) = (s.show_logo, logo) {
            if let Ok(img) = image::load_from_memory(l) {
                let mut jpg = Vec::new();
                let rgb = img.to_rgb8();
                if image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg, 90).encode_image(&rgb).is_ok() {
                    if let Ok(el) = genpdf::elements::Image::from_reader(Cursor::new(jpg)) {
                        doc.push(el.with_dpi(img.width() as f64 * 25.4 / 28.0));
                    }
                }
            }
        }
        doc.push(Paragraph::new(s.office_name.clone()).styled(Style::new().bold().with_font_size(13).with_color(acc)));
    }
    doc.push(Break::new(0.5));
    for bk in &blocks {
        match bk {
            Block::Title(t) => doc.push(Paragraph::new(t.clone()).styled(Style::new().bold().with_font_size(19).with_color(acc))),
            Block::Sub(t) => doc.push(Paragraph::new(t.clone()).styled(Style::new().with_font_size(9).with_color(Color::Rgb(110, 110, 110)))),
            Block::H(t) => {
                doc.push(Break::new(0.8));
                doc.push(Paragraph::new(t.clone()).styled(Style::new().bold().with_font_size(13).with_color(acc)));
            }
            Block::P(t) => {
                doc.push(Break::new(0.4));
                doc.push(Paragraph::new(t.clone()).styled(Style::new().with_font_size(9).with_color(Color::Rgb(90, 90, 90))));
            }
            Block::Kpi(items) => {
                doc.push(Break::new(0.6));
                let mut t = TableLayout::new(vec![1; items.len()]);
                t.set_cell_decorator(FrameCellDecorator::new(true, true, false));
                let mut row = t.row();
                for (k, v) in items {
                    let mut l = genpdf::elements::LinearLayout::vertical();
                    l.push(Paragraph::new(v.clone()).aligned(Alignment::Center).styled(Style::new().bold().with_font_size(18).with_color(acc)));
                    l.push(Paragraph::new(k.clone()).aligned(Alignment::Center).styled(Style::new().with_font_size(8).with_color(Color::Rgb(110, 110, 110))));
                    row = row.element(l.padded(2));
                }
                row.push().map_err(|e| e.to_string())?;
                doc.push(t);
            }
            Block::Table(head, rows) => {
                doc.push(Break::new(0.4));
                let mut w = vec![1usize; head.len()];
                w[0] = if head.len() > 3 { 3 } else { 2 };
                let mut t = TableLayout::new(w);
                t.set_cell_decorator(FrameCellDecorator::new(true, true, false));
                let mut hr = t.row();
                for h in head {
                    hr = hr.element(Paragraph::new(h.clone()).styled(Style::new().bold().with_font_size(9).with_color(acc)).padded(2));
                }
                hr.push().map_err(|e| e.to_string())?;
                for r in rows {
                    let mut rr = t.row();
                    for c in r {
                        rr = rr.element(Paragraph::new(c.clone()).styled(Style::new().with_font_size(9)).padded(2));
                    }
                    rr.push().map_err(|e| e.to_string())?;
                }
                doc.push(t);
            }
        }
    }
    doc.render_to_file(path).map_err(|e| e.to_string())
}
