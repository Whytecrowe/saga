slint::include_modules!();

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_repeat_alarm_MainActivity_onCreate(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
) {
    // For now, simple bridge. Real Android init will need more work.
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
}

// Re-export things if needed or just keep it for compilation
