slint::include_modules!();

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();

    let ui = App::new().unwrap();
    ui.run().unwrap();
}

// For desktop testing
#[cfg(not(target_os = "android"))]
pub fn run() {
    println!("Creating UI...");
    let ui = App::new().unwrap();
    println!("UI created successfully");

    println!("Running UI...");
    ui.run().unwrap();
    println!("UI closed");
}
