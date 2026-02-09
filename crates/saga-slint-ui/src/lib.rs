slint::include_modules!();

use saga_core::model::{Echo};
use saga_storage_sqlite::Storage;
use chrono::Local;
use std::rc::Rc;


pub fn run() {
    let storage = Storage::new("saga.db").expect("Failed to open database");

    let ui = App::new().expect("Failed to create UI");

    load_data(&ui, &storage);

    let ui_weak = ui.as_weak();
    let storage_clone = Rc::new(storage);

    // load echoes
    {
        let ui_weak = ui_weak.clone();
        let storage = storage_clone.clone();
        ui.on_load_echoes(move || {
            if let Some(ui) = ui_weak.upgrade() {
                load_echoes_data(&ui, &storage);
            }
        });
    }

    // create echo callback
    {
        let ui_weak = ui_weak.clone();
        let storage = storage_clone.clone();
        ui.on_create_echo_at_index(move |index, markdown| {
            let sections = storage.get_all_sections()
                .expect("Failed to load sections");

            if let Some(section) = sections.get(index as usize) {
                let echo = Echo::new(
                    Local::now().date_naive(),
                    section.id,
                    markdown.to_string(),
                );

                storage.save_echo(&echo).expect("Failed to save echo");

                if let Some(ui) = ui_weak.upgrade() {
                    load_echoes_data(&ui, &storage);
                }
            }
        });
    }

    ui.run().expect("Failed to run UI");
}

fn load_data(ui: &App, storage: &Storage) {
    load_sections_data(ui, storage);
    load_echoes_data(ui, storage);
}

fn load_sections_data(ui: &App, storage: &Storage) {
    let sections = storage.get_all_sections()
        .expect("Failed to load sections");

    // Extract just the names
    let section_names: Vec<slint::SharedString> = sections.iter()
        .map(|s| s.name.clone().into())
        .collect();

    ui.set_section_names(Rc::new(slint::VecModel::from(section_names)).into());
}

fn load_echoes_data(ui: &App, storage: &Storage) {
    let today = Local::now().date_naive();
    let echoes = storage.get_echoes_for_day(today)
        .expect("Failed to load echoes");

    let sections = storage.get_all_sections()
        .expect("Failed to load sections");

    let echo_items: Vec<EchoItem> = echoes.iter()
        .map(|e| {
            let section_name = sections.iter()
                .find(|s| s.id == e.section_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            EchoItem {
                id: e.id.to_string().into(),
                markdown: e.markdown.clone().into(),
                section_name: section_name.into(),
                day: e.day.to_string().into(),
                created_at: e.created_at.format("%I:%M %p").to_string().into(),
            }
        })
        .collect();

    ui.set_echoes(Rc::new(slint::VecModel::from(echo_items)).into());
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    run();
}
