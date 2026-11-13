//! Aylık rapor PDF'i.
use serde_json::Value;
use std::path::Path;

fn n(v: &Value, k: &str) -> i64 {
    v[k].as_i64().unwrap_or(0)
}
fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

pub fn write_report_pdf(d: &Value, office: &str, path: &Path) -> Result<(), String> {
    use genpdf::elements::{Break, Paragraph, TableLayout};
    use genpdf::style::Style;
    use genpdf::{fonts, Document, Element, Margins, SimplePageDecorator};
    let regular = fonts::FontData::new(crate::export::FONT_REGULAR.to_vec(), None).map_err(|e| e.to_string())?;
    let bold = fonts::FontData::new(crate::export::FONT_BOLD.to_vec(), None).map_err(|e| e.to_string())?;
    let family = fonts::FontFamily { regular: regular.clone(), bold: bold.clone(), italic: regular, bold_italic: bold };
    let mut doc = Document::new(family);
    doc.set_title("Aylık rapor");
    doc.set_font_size(10);
    let mut dec = SimplePageDecorator::new();
    dec.set_margins(Margins::trbl(14, 14, 14, 14));
    doc.set_page_decorator(dec);
    doc.push(Paragraph::new(format!("{office} — Aylık Rapor ({})", s(d, "month"))).styled(Style::new().bold().with_font_size(16)));
    doc.push(Break::new(1));
    let mut t = TableLayout::new(vec![3, 1]);
    for (k, l) in [
        ("active", "Yayındaki portföy"), ("new_month", "Ay içinde eklenen"), ("published_month", "Ay içinde yayına alınan"),
        ("rented_month", "Ay içinde kiralanan"), ("sold_month", "Ay içinde satılan"), ("stale", "60+ gündür yayında"),
        ("buyers", "Aktif alıcı"), ("open_tasks", "Açık görev"),
    ] {
        t.row().element(Paragraph::new(l)).element(Paragraph::new(n(d, k).to_string())).push().map_err(|e| e.to_string())?;
    }
    doc.push(t);
    doc.push(Break::new(1));
    doc.push(Paragraph::new("Çalışan performansı").styled(Style::new().bold().with_font_size(12)));
    let mut t = TableLayout::new(vec![3, 1, 1, 1, 1]);
    t.row().element(Paragraph::new("Çalışan").styled(Style::new().bold())).element(Paragraph::new("Eklenen").styled(Style::new().bold()))
        .element(Paragraph::new("Yayın").styled(Style::new().bold())).element(Paragraph::new("Kira").styled(Style::new().bold()))
        .element(Paragraph::new("Satış").styled(Style::new().bold())).push().map_err(|e| e.to_string())?;
    for u in d["by_user"].as_array().cloned().unwrap_or_default() {
        t.row().element(Paragraph::new(s(&u, "user"))).element(Paragraph::new(n(&u, "created").to_string()))
            .element(Paragraph::new(n(&u, "published").to_string())).element(Paragraph::new(n(&u, "rented").to_string()))
            .element(Paragraph::new(n(&u, "sold").to_string())).push().map_err(|e| e.to_string())?;
    }
    doc.push(t);
    doc.push(Break::new(1));
    doc.push(Paragraph::new("İlçelere göre yayındaki portföyler").styled(Style::new().bold().with_font_size(12)));
    let mut t = TableLayout::new(vec![3, 1]);
    for u in d["by_district"].as_array().cloned().unwrap_or_default() {
        t.row().element(Paragraph::new(s(&u, "name"))).element(Paragraph::new(n(&u, "n").to_string())).push().map_err(|e| e.to_string())?;
    }
    doc.push(t);
    doc.render_to_file(path).map_err(|e| e.to_string())
}
