fn main() {
    slint_build::compile("ui/app.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");

        res.set("FileDescription", "Saga Bell Tool");
        res.set("ProductName", "Saga Bell");
        res.compile().unwrap();
    }
}
