fn main() {
    if cfg!(target_os = "windows") && std::env::var("PROFILE").as_deref() == Ok("release") {
        let mut resource = winres::WindowsResource::new();
        resource
            .set_manifest_file("app.manifest")
            .set_icon("assets/icon.ico")
            .set("ProductName", "GoodbyeDPI Manager")
            .set("FileDescription", "Bật và tắt dịch vụ GoodbyeDPI")
            .set("LegalCopyright", "GoodbyeDPI Manager")
            .set("ProductVersion", "1.5.0")
            .set("FileVersion", "1.5.0");
        resource.compile().expect("không thể biên dịch tài nguyên Windows");
    }
}

