slint::include_modules!();

use chrono::{Local, NaiveDate};
use saga_core::model::{ECHO_TYPE_PLAIN, Echo, EchoContent, PlainData};
use saga_storage_sqlite::{Storage, open_default};
use slint::{ModelRc, SharedString, VecModel};
use std::collections::BTreeMap;
use std::rc::Rc;
use uuid::Uuid;

pub fn run() {
    let storage = open_storage();

    let ui = App::new().expect("Failed to create UI");

    let today = Local::now().date_naive();
    ui.set_today_iso(today.to_string().into());

    // Shared, single-threaded ownership so every callback can hold the DB.
    let storage = Rc::new(storage);

    load_timeline(&ui, &storage);

    // ---- create a new Plain Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_create_echo(move |day_iso, body| {
            let day = NaiveDate::parse_from_str(&day_iso, "%Y-%m-%d")
                .unwrap_or_else(|_| Local::now().date_naive());

            let echo = Echo::new(
                day,
                String::new(),
                EchoContent::PlainEcho(PlainData {
                    markdown: body.to_string(),
                }),
            );
            storage.save_echo(&echo).expect("Failed to save echo");

            if let Some(ui) = ui_weak.upgrade() {
                load_timeline(&ui, &storage);
            }
        });
    }

    // ---- edit an existing Echo's body ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_update_echo(move |id, body| {
            let uuid = Uuid::parse_str(&id).expect("Invalid Echo UUID");
            if let Some(mut echo) = storage.get_echo(&uuid).expect("Failed to load echo") {
                echo.update_content(EchoContent::PlainEcho(PlainData {
                    markdown: body.to_string(),
                }));
                storage.update_echo(&echo).expect("Failed to update echo");
            }

            if let Some(ui) = ui_weak.upgrade() {
                load_timeline(&ui, &storage);
            }
        });
    }

    // ---- delete an Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_delete_echo(move |id| {
            let uuid = Uuid::parse_str(&id).expect("Invalid Echo UUID");
            storage.delete_echo(&uuid).expect("Failed to delete echo");

            if let Some(ui) = ui_weak.upgrade() {
                load_timeline(&ui, &storage);
            }
        });
    }

    ui.run().expect("Failed to run UI");
}

// Reads every Plain Echo, groups them by day (newest day first),
// and hands the nested model to the timeline.
fn load_timeline(ui: &App, storage: &Storage) {
    let echoes = storage.get_all_echoes().expect("Failed to load echoes");
    let today = Local::now().date_naive();

    let mut by_day: BTreeMap<NaiveDate, Vec<Echo>> = BTreeMap::new();
    for echo in echoes
        .into_iter()
        .filter(|e| e.content_type_name() == ECHO_TYPE_PLAIN)
    {
        by_day.entry(echo.day).or_default().push(echo);
    }
    // Always show today, even when it's empty, so there's an entry point.
    by_day.entry(today).or_default();

    let mut days: Vec<DaySignpost> = Vec::new();
    for (day, mut day_echoes) in by_day.into_iter().rev() {
        day_echoes.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let items: Vec<EchoItem> = day_echoes.iter().map(echo_to_item).collect();

        days.push(DaySignpost {
            iso: day.to_string().into(),
            label: day.format("%A, %B %e").to_string().into(),
            rel: relative_label(day, today).into(),
            is_today: day == today,
            echoes: ModelRc::new(VecModel::from(items)),
        });
    }

    ui.set_days(ModelRc::new(VecModel::from(days)));
}

fn echo_to_item(echo: &Echo) -> EchoItem {
    let body = match &echo.content {
        EchoContent::PlainEcho(data) => data.markdown.clone(),
        _ => String::new(),
    };
    let tags: Vec<SharedString> = echo.tags.iter().map(|t| t.clone().into()).collect();

    EchoItem {
        id: echo.id.to_string().into(),
        body: body.into(),
        time: echo.created_at.format("%I:%M %p").to_string().into(),
        mood: echo.mood.map(|v| v as i32).unwrap_or(0),
        energy: echo.energy.map(|v| v as i32).unwrap_or(0),
        has_mood: echo.mood.is_some(),
        has_energy: echo.energy.is_some(),
        pinned: echo.pinned,
        tags: ModelRc::new(VecModel::from(tags)),
    }
}

fn relative_label(day: NaiveDate, today: NaiveDate) -> String {
    let diff = (today - day).num_days();
    match diff {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        -1 => "Tomorrow".to_string(),
        n if n > 1 => format!("{n} days ago"),
        n => format!("in {} days", -n),
    }
}

#[cfg(not(target_os = "android"))]
fn open_storage() -> Storage {
    open_default().expect("Failed to open database")
}

#[cfg(target_os = "android")]
fn open_storage() -> Storage {
    Storage::new(android_db_path()).expect("Failed to open database")
}

#[cfg(target_os = "android")]
fn android_db_path() -> String {
    use jni::JavaVM;
    use jni::objects::{JObject, JString};

    let ctx = ndk_context::android_context();

    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    let path_result: String = env
        .with_local_frame(16, |env| {
            let context = unsafe { JObject::from_raw(ctx.context().cast()) };

            let files_dir = env
                .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])
                .map_err(|e| format!("Failed to call getFilesDir: {:?}", e))?
                .l()?;

            let path_obj = env
                .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
                .map_err(|e| format!("Failed to get path: {:?}", e))?
                .l()?;

            let jstring: JString = path_obj.into();
            let rust_str: String = env.get_string(&jstring)?.into();

            Ok(rust_str)
        })
        .expect("Failed to retrieve Android files directory");

    format!("{}/saga.db", path_result)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    run();
}
