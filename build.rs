fn main() {
    // Windows: exe dosyasına ve görev çubuğuna program simgesini göm.
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Emlak Ofis");
        let _ = res.compile();
    }
}
