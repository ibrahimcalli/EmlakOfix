//! Arayüz dili: kaynak metinler Türkçedir; `ti("Türkçe metin")` seçili dilde karşılığını döndürür,
//! karşılığı yoksa Türkçesini gösterir. Çeviriler assets/ui_en.tsv dosyasındadır (Türkçe<TAB>İngilizce).
//! Yeni dil eklemek için aynı biçimde yeni bir .tsv ekleyip `UiLang` ve `dict()` içine bağlayın.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiLang {
    Tr,
    En,
}

impl UiLang {
    pub const ALL: [UiLang; 2] = [UiLang::Tr, UiLang::En];
    pub fn code(self) -> &'static str {
        match self {
            UiLang::Tr => "tr",
            UiLang::En => "en",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            UiLang::Tr => "Türkçe",
            UiLang::En => "English",
        }
    }
    pub fn from_code(c: &str) -> UiLang {
        match c {
            "en" => UiLang::En,
            _ => UiLang::Tr,
        }
    }
}

static CUR: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: UiLang) {
    CUR.store(if l == UiLang::En { 1 } else { 0 }, Ordering::Relaxed);
}
pub fn lang() -> UiLang {
    if CUR.load(Ordering::Relaxed) == 1 {
        UiLang::En
    } else {
        UiLang::Tr
    }
}

const EN_TSV: &str = include_str!("../assets/ui_en.tsv");

fn unesc(s: &str) -> String {
    s.replace("\\n", "\n")
}

fn en_dict() -> &'static HashMap<&'static str, &'static str> {
    static D: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    D.get_or_init(|| {
        let mut m = HashMap::new();
        for line in EN_TSV.lines() {
            if let Some((k, v)) = line.split_once('\t') {
                let (k, v): (&'static str, &'static str) = (Box::leak(unesc(k).into_boxed_str()), Box::leak(unesc(v).into_boxed_str()));
                m.insert(k, v);
            }
        }
        m
    })
}

/// Arayüz metni: seçili dile çevirir (Türkçe kaynak metin → karşılık).
pub fn ti(s: &'static str) -> &'static str {
    match lang() {
        UiLang::Tr => s,
        UiLang::En => en_dict().get(s).copied().unwrap_or(s),
    }
}

/// Belirli bir dile çevirir (testler ve dinamik metinler için).
pub fn translate(l: UiLang, s: &str) -> String {
    match l {
        UiLang::Tr => s.to_string(),
        UiLang::En => en_dict().get(s).map(|x| x.to_string()).unwrap_or_else(|| s.to_string()),
    }
}

/// Sözlük sağlığı: (geçerli satır sayısı, hatalı giriş listesi)
pub fn audit() -> (usize, Vec<String>) {
    let mut bad = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut n = 0;
    for (i, line) in EN_TSV.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match line.split_once('\t') {
            None => bad.push(format!("satır {}: sekme yok", i + 1)),
            Some((k, v)) => {
                if v.trim().is_empty() {
                    bad.push(format!("satır {}: boş çeviri", i + 1));
                } else if !seen.insert(k.to_string()) {
                    bad.push(format!("satır {}: yinelenen anahtar", i + 1));
                } else {
                    n += 1;
                }
            }
        }
    }
    (n, bad)
}
