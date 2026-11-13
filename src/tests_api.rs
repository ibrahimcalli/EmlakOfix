//! `cargo test` ile Ayarlar > Testler sekmesindeki aynı testleri çalıştırır (src/selftest.rs).
#[test]
fn tum_otomatik_testler() {
    let r = crate::selftest::run_all();
    let bad: Vec<String> = r.iter().filter(|x| !x.ok).map(|x| format!("[{}] {} -> {}", x.group, x.name, x.detail)).collect();
    println!("{}", crate::selftest::to_text(&r));
    assert!(bad.is_empty(), "başarısız testler:\n{}", bad.join("\n"));
    assert!(r.len() >= 60, "test sayısı beklenenden az: {}", r.len());
}
