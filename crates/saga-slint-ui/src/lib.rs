slint::include_modules!();

use chrono::Local;
use saga_core::model::{Echo, Section};
use saga_storage_sqlite::Storage;
use std::rc::Rc;

pub fn run() {
    let db_path = get_database_path();
    let storage = Storage::new(&db_path).expect("Failed to open database");

    let ui = App::new().expect("Failed to create UI");

    load_data(&ui, &storage);

    let ui_weak = ui.as_weak();
    let storage_clone = Rc::new(storage);

    // add sections
    {
        let ui_weak = ui_weak.clone();
        let storage = storage_clone.clone();

        ui.on_add_section(move |name| {
            let new_section = Section::new(name.to_string(), 0);
            storage
                .save_section(&new_section)
                .expect("Failed to save section");

            if let Some(ui) = ui_weak.upgrade() {
                load_data(&ui, &storage);
            }
        });
    }

    // create echo callback
    {
        let ui_weak = ui_weak.clone();
        let storage = storage_clone.clone();

        ui.on_save_echo(move |echo_item| {
            let section_name = echo_item.section_name.to_string();
            let title = echo_item.title.to_string();
            let markdown = echo_item.markdown.to_string();
            let id_str = echo_item.id.to_string();

            // Find or create section
            let sections = storage.get_all_sections().expect("Failed to load sections");

            let section = sections
                .iter()
                .find(|s| s.name == section_name.as_str())
                .cloned()
                .unwrap_or_else(|| {
                    let max_sort_order = sections.iter().map(|s| s.sort_order).max().unwrap_or(0);

                    // Section doesn't exist - create it
                    let new_section = Section::new(section_name.to_string(), max_sort_order + 1);
                    storage
                        .save_section(&new_section)
                        .expect("Failed to save section");
                    new_section
                });

            if id_str.is_empty() {
                // Make a new Echo if ID doesn't exist yet
                let day_str = echo_item.day.to_string();
                let target_day = chrono::NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| Local::now().date_naive());

                let echo = Echo::new(
                    target_day,
                    section.id,
                    title.to_string(),
                    markdown.to_string(),
                );

                storage.save_echo(&echo).expect("Failed to save echo");
            } else {
                // Update existing Echo
                let uuid = uuid::Uuid::parse_str(&id_str).expect("Invalid UUID for Echo");
                if let Some(mut echo) = storage.get_echo(&uuid).unwrap() {
                    echo.title = title;
                    echo.markdown = markdown;
                    echo.section_id = section.id;
                    echo.updated_at = Local::now();

                    storage.update_echo(&echo).expect("Failed to update echo");
                }
            }

            if let Some(ui) = ui_weak.upgrade() {
                load_data(&ui, &storage);
            }
        });
    }

    ui.run().expect("Failed to run UI");
}

#[cfg(target_os = "android")]
fn get_database_path() -> String {
    use jni::JavaVM;
    use jni::objects::{JObject, JString};

    let ctx = ndk_context::android_context();

    // 1. Get the Java VM and attach the current thread
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    // 2. Wrap JNI calls in a local frame (size 16 is plenty for this)
    // This ensures all JObjects created inside are cleared from memory immediately after.
    let path_result: String = env
        .with_local_frame(16, |env| {
            let context = unsafe { JObject::from_raw(ctx.context().cast()) };

            // Call context.getFilesDir() -> returns a File object
            let files_dir = env
                .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])
                .map_err(|e| format!("Failed to call getFilesDir: {:?}", e))?
                .l()?;

            // Call files_dir.getAbsolutePath() -> returns a String object
            let path_obj = env
                .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
                .map_err(|e| format!("Failed to get path: {:?}", e))?
                .l()?;

            // Convert the Java String object into a Rust String
            let jstring: JString = path_obj.into();
            let rust_str: String = env.get_string(&jstring)?.into();

            Ok(rust_str)
        })
        .expect("Failed to retrieve Android files directory");

    // 3. Construct the final path
    format!("{}/saga.db", path_result)
}

#[cfg(not(target_os = "android"))]
fn get_database_path() -> String {
    "saga.db".to_string()
}

fn load_data(ui: &App, storage: &Storage) {
    let all_sections = load_echoes_data(ui, storage);

    let mut journey_days: Vec<JourneyDay> = Vec::new();
    let today = Local::now().date_naive();

    for i in 0..14 {
        let date = today - chrono::Duration::days(i);
        journey_days.push(JourneyDay {
            day_id: date.to_string().into(),
            display_name: date.format("%b %e").to_string().to_uppercase().into(),
            is_today: i == 0,
        })
    }

    ui.set_journey_days(Rc::new(slint::VecModel::from(journey_days)).into());

    let section_names: Vec<slint::SharedString> =
        all_sections.iter().map(|s| s.name.clone().into()).collect();

    // Update the UI
    ui.set_sections(Rc::new(slint::VecModel::from(section_names)).into());
}

fn load_echoes_data(ui: &App, storage: &Storage) -> Vec<Section> {
    let echoes = storage.get_all_echoes().expect("Failed to load echoes");

    let sections = storage.get_all_sections().expect("Failed to load sections");

    let echo_items: Vec<EchoItem> = echoes
        .iter()
        .map(|e| {
            let section_name = sections
                .iter()
                .find(|s| s.id == e.section_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let preview_text = e
                .markdown
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(100)
                .collect::<String>();

            EchoItem {
                id: e.id.to_string().into(),
                title: e.title.clone().into(),
                preview: preview_text.into(),
                markdown: e.markdown.clone().into(),
                section_name: section_name.into(),
                day: e.day.to_string().into(),
                created_at: e.created_at.format("%I:%M %p").to_string().into(),
            }
        })
        .collect();

    ui.set_echoes(Rc::new(slint::VecModel::from(echo_items)).into());

    sections
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    run();
}
