slint::include_modules!();

use chrono::{Local, NaiveDate};
use saga_core::model::{
    ECHO_TYPE_PLAIN, ECHO_TYPE_TASK, Echo, EchoContent, PlainData, Priority,
};
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

    let storage = Rc::new(storage);

    load_timeline(&ui, &storage);

    // ---- create a Plain Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_create_plain(move |day_iso, body| {
            let echo = Echo::new(
                parse_day(&day_iso),
                String::new(),
                EchoContent::PlainEcho(PlainData {
                    markdown: body.to_string(),
                }),
            );
            storage.save_echo(&echo).expect("Failed to save echo");
            reload(&ui_weak, &storage);
        });
    }

    // ---- edit a Plain Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_update_plain(move |id, body| {
            if let Some(mut echo) = load_echo(&storage, &id) {
                echo.update_content(EchoContent::PlainEcho(PlainData {
                    markdown: body.to_string(),
                }));
                storage.update_echo(&echo).expect("Failed to update echo");
            }
            reload(&ui_weak, &storage);
        });
    }

    // ---- create a Task Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_create_task(move |day_iso, title, description, priority, completed| {
            let mut echo = Echo::new_task(parse_day(&day_iso), title.to_string());
            if let Some(task) = echo.as_task_mut() {
                task.description = non_empty(&description);
                task.priority = parse_priority(&priority);
                if completed {
                    task.complete();
                }
            }
            storage.save_echo(&echo).expect("Failed to save task");
            reload(&ui_weak, &storage);
        });
    }

    // ---- edit a Task Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_update_task(move |id, title, description, priority, completed| {
            if let Some(mut echo) = load_echo(&storage, &id) {
                echo.title = title.to_string();
                if let Some(task) = echo.as_task_mut() {
                    task.description = non_empty(&description);
                    task.priority = parse_priority(&priority);
                    if completed {
                        task.complete();
                    } else {
                        task.uncomplete();
                    }
                }
                echo.updated_at = Local::now();
                storage.update_echo(&echo).expect("Failed to update task");
            }
            reload(&ui_weak, &storage);
        });
    }

    // ---- toggle a Task's completion from the card ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_toggle_complete(move |id| {
            if let Some(mut echo) = load_echo(&storage, &id) {
                if let Some(task) = echo.as_task_mut() {
                    if task.completed {
                        task.uncomplete();
                    } else {
                        task.complete();
                    }
                }
                echo.updated_at = Local::now();
                storage.update_echo(&echo).expect("Failed to update task");
            }
            reload(&ui_weak, &storage);
        });
    }

    // ---- delete an Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_delete_echo(move |id| {
            let uuid = Uuid::parse_str(&id).expect("Invalid Echo UUID");
            storage.delete_echo(&uuid).expect("Failed to delete echo");
            reload(&ui_weak, &storage);
        });
    }

    ui.run().expect("Failed to run UI");
}

fn reload(ui_weak: &slint::Weak<App>, storage: &Storage) {
    if let Some(ui) = ui_weak.upgrade() {
        load_timeline(&ui, storage);
    }
}

fn load_echo(storage: &Storage, id: &SharedString) -> Option<Echo> {
    let uuid = Uuid::parse_str(id).expect("Invalid Echo UUID");
    storage.get_echo(&uuid).expect("Failed to load echo")
}

fn parse_day(iso: &str) -> NaiveDate {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").unwrap_or_else(|_| Local::now().date_naive())
}

fn non_empty(value: &SharedString) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_priority(value: &str) -> Priority {
    match value {
        "low" => Priority::Low,
        "high" => Priority::High,
        "critical" => Priority::Critical,
        _ => Priority::Medium,
    }
}

fn priority_label(priority: &Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
        Priority::Critical => "critical",
    }
}

// Reads Plain + Task Echoes, groups them by day (newest first), and
// hands the nested model to the timeline.
fn load_timeline(ui: &App, storage: &Storage) {
    let echoes = storage.get_all_echoes().expect("Failed to load echoes");
    let today = Local::now().date_naive();

    let mut by_day: BTreeMap<NaiveDate, Vec<Echo>> = BTreeMap::new();
    for echo in echoes.into_iter().filter(is_shown) {
        by_day.entry(echo.day).or_default().push(echo);
    }
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

// Plain + Task are the only types with UI so far.
fn is_shown(echo: &Echo) -> bool {
    let kind = echo.content_type_name();
    kind == ECHO_TYPE_PLAIN || kind == ECHO_TYPE_TASK
}

fn echo_to_item(echo: &Echo) -> EchoItem {
    let tags: Vec<SharedString> = echo.tags.iter().map(|t| t.clone().into()).collect();

    let mut item = EchoItem {
        id: echo.id.to_string().into(),
        kind: "plain".into(),
        title: SharedString::new(),
        body: SharedString::new(),
        time: echo.created_at.format("%I:%M %p").to_string().into(),
        completed: false,
        priority: SharedString::new(),
        checklist_done: 0,
        checklist_total: 0,
        mood: echo.mood.map(|v| v as i32).unwrap_or(0),
        energy: echo.energy.map(|v| v as i32).unwrap_or(0),
        has_mood: echo.mood.is_some(),
        has_energy: echo.energy.is_some(),
        pinned: echo.pinned,
        tags: ModelRc::new(VecModel::from(tags)),
    };

    match &echo.content {
        EchoContent::TaskEcho(task) => {
            let (done, total) = task.progress();
            item.kind = "task".into();
            item.title = echo.title.clone().into();
            item.body = task.description.clone().unwrap_or_default().into();
            item.completed = task.completed;
            item.priority = priority_label(&task.priority).into();
            item.checklist_done = done as i32;
            item.checklist_total = total as i32;
        }
        EchoContent::PlainEcho(plain) => {
            item.body = plain.markdown.clone().into();
        }
        _ => {}
    }

    item
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
