slint::include_modules!();

mod calendar;

use chrono::{Datelike, Local, NaiveDate, Timelike};
use saga_core::model::{
    ECHO_TYPE_PLAIN, ECHO_TYPE_TASK, Echo, EchoContent, PlainData, Priority, Seed, SeedKind, TaskData,
};
use saga_storage_sqlite::{Storage, open_default};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use uuid::Uuid;

pub fn run() {
    let storage = open_storage();

    let ui = App::new().expect("Failed to create UI");

    let now = Local::now();
    let today = now.date_naive();
    ui.set_today_iso(today.to_string().into());
    ui.set_greeting(greeting_for(now.hour()).into());
    ui.set_today_label(now.format("%A, %B %-d").to_string().into());
    if let Some(tomorrow) = today.succ_opt() {
        ui.set_tomorrow_iso(tomorrow.to_string().into());
    }

    let storage = Rc::new(storage);

    // The composer's editable checklist, shared with the callbacks below.
    let checklist_model: Rc<VecModel<ChecklistDraft>> = Rc::new(VecModel::default());
    ui.set_draft_checklist(ModelRc::from(checklist_model.clone()));

    load_timeline(&ui, &storage);
    ui.set_seed_text(seed_text_for(&storage, today).into());

    // ---- save today's intention (create or update the Being seed) ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();

        ui.on_set_seed(move |text| {
            let text = text.trim().to_string();
            let today = Local::now().date_naive();
            match current_seed(&storage, today) {
                Some(mut seed) => {
                    seed.text = text;
                    seed.updated_at = Local::now();
                    storage.update_seed(&seed).expect("Failed to update seed");
                }
                None => {
                    if !text.is_empty() {
                        let seed = Seed::new(text, SeedKind::Being, today, None);
                        storage.save_seed(&seed).expect("Failed to save seed");
                    }
                }
            }
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_seed_text(seed_text_for(&storage, today).into());
            }
        });
    }

    // ---- date picker ----
    // (year, month) currently on show; shared by the two callbacks below.
    let picker_month = Rc::new(Cell::new((today.year(), today.month())));

    // open on the month of the given date, or the current month
    {
        let ui_weak = ui.as_weak();
        let picker_month = picker_month.clone();

        ui.on_picker_show(move |iso| {
            let anchor = parse_iso_day(&iso).unwrap_or_else(|| Local::now().date_naive());
            picker_month.set((anchor.year(), anchor.month()));
            if let Some(ui) = ui_weak.upgrade() {
                show_month(&ui, picker_month.get());
            }
        });
    }

    // step to the previous / next month
    {
        let ui_weak = ui.as_weak();
        let picker_month = picker_month.clone();

        ui.on_picker_shift(move |delta| {
            let (year, month) = picker_month.get();
            picker_month.set(calendar::shift_month(year, month, delta));
            if let Some(ui) = ui_weak.upgrade() {
                show_month(&ui, picker_month.get());
            }
        });
    }

    ui.on_format_day(|iso| {
        parse_iso_day(&iso)
            .map(format_day)
            .unwrap_or_default()
            .into()
    });

    // ---- checklist: add an item ----
    {
        let model = checklist_model.clone();

        ui.on_checklist_add(move |text| {
            let text = text.trim().to_string();
            if !text.is_empty() {
                model.push(ChecklistDraft {
                    text: text.into(),
                    done: false,
                });
            }
        });
    }

    // ---- checklist: remove an item ----
    {
        let model = checklist_model.clone();

        ui.on_checklist_remove(move |index| {
            let i = index as usize;
            if i < model.row_count() {
                model.remove(i);
            }
        });
    }

    // ---- checklist: toggle an item done ----
    {
        let model = checklist_model.clone();

        ui.on_checklist_toggle(move |index| {
            let i = index as usize;
            if let Some(mut row) = model.row_data(i) {
                row.done = !row.done;
                model.set_row_data(i, row);
            }
        });
    }

    // ---- checklist: edit an item's text ----
    {
        let model = checklist_model.clone();

        ui.on_checklist_edit(move |index, text| {
            let i = index as usize;
            if let Some(mut row) = model.row_data(i) {
                row.text = text;
                model.set_row_data(i, row);
            }
        });
    }

    // ---- checklist: fill the draft when the composer opens ----
    {
        let model = checklist_model.clone();
        let storage = storage.clone();

        ui.on_begin_task_edit(move |id| {
            if id.is_empty() {
                model.set_vec(Vec::new());
                return;
            }
            let rows = load_echo(&storage, &id)
                .and_then(|echo| echo.as_task().map(checklist_to_draft))
                .unwrap_or_default();
            model.set_vec(rows);
        });
    }

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
        let checklist_model = checklist_model.clone();

        ui.on_create_task(move |day_iso, title, description, priority, completed, due| {
            let mut echo = Echo::new_task(parse_day(&day_iso), title.to_string());
            if let Some(task) = echo.as_task_mut() {
                task.description = non_empty(&description);
                task.priority = parse_priority(&priority);
                apply_due(task, &due);
                apply_checklist(task, &checklist_model, completed);
            }
            storage.save_echo(&echo).expect("Failed to save task");
            reload(&ui_weak, &storage);
        });
    }

    // ---- edit a Task Echo ----
    {
        let ui_weak = ui.as_weak();
        let storage = storage.clone();
        let checklist_model = checklist_model.clone();

        ui.on_update_task(move |id, title, description, priority, completed, due| {
            if let Some(mut echo) = load_echo(&storage, &id) {
                echo.title = title.to_string();
                if let Some(task) = echo.as_task_mut() {
                    task.description = non_empty(&description);
                    task.priority = parse_priority(&priority);
                    apply_due(task, &due);
                    apply_checklist(task, &checklist_model, completed);
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

// Takes the hour (0-23) rather than reading the clock, so it can be tested.
fn greeting_for(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=21 => "Good evening",
        _ => "Hello, night owl",
    }
}

// Today's intention = the most recent active Being seed.
fn current_seed(storage: &Storage, today: NaiveDate) -> Option<Seed> {
    storage
        .get_seeds_active_on(today)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.kind == SeedKind::Being)
        .last()
}

fn seed_text_for(storage: &Storage, today: NaiveDate) -> String {
    current_seed(storage, today)
        .map(|s| s.text)
        .unwrap_or_default()
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

fn parse_iso_day(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

// "Thu, Nov 12"; the year is added only when it isn't the current one.
fn format_day(date: NaiveDate) -> String {
    let pattern = if date.year() == Local::now().year() {
        "%a, %b %-d"
    } else {
        "%a, %b %-d, %Y"
    };
    date.format(pattern).to_string()
}

// Sets or clears the due date. An existing due time is kept while a
// date remains, and dropped with it.
fn apply_due(task: &mut TaskData, due_iso: &str) {
    match parse_iso_day(due_iso) {
        Some(date) => {
            let time = task.due_time;
            task.set_due(Some(date), time);
        }
        None => task.clear_due(),
    }
}

// Fills the date picker with the given month.
fn show_month(ui: &App, (year, month): (i32, u32)) {
    let today = Local::now().date_naive();

    let cells: Vec<DayCell> = calendar::month_grid(year, month)
        .unwrap_or_default()
        .into_iter()
        .map(|cell| DayCell {
            day: cell.date.day() as i32,
            iso: cell.date.to_string().into(),
            in_month: cell.in_month,
            is_today: cell.date == today,
        })
        .collect();

    let title = NaiveDate::from_ymd_opt(year, month, 1)
        .map(|first| first.format("%B %Y").to_string())
        .unwrap_or_default();

    ui.set_picker_cells(ModelRc::new(VecModel::from(cells)));
    ui.set_picker_title(title.into());
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

fn checklist_to_draft(task: &TaskData) -> Vec<ChecklistDraft> {
    task.checklist
        .iter()
        .map(|item| ChecklistDraft {
            text: item.text.clone().into(),
            done: item.done,
        })
        .collect()
}

// Rebuilds the checklist from the draft rows; completion is set manually,
// independent of the items.
fn apply_checklist(task: &mut TaskData, model: &VecModel<ChecklistDraft>, completed: bool) {
    task.clear_checklist();
    for i in 0..model.row_count() {
        let row = model.row_data(i).expect("row index in range");
        let text = row.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        task.add_item(text);
        if row.done {
            let last = task.checklist.len() - 1;
            task.set_item_done(last, true);
        }
    }
    if completed {
        task.complete();
    } else {
        task.uncomplete();
    }
}

// Reads Plain + Task Echoes, groups them by day (newest first), and
// hands the nested model to the timeline and today's rows to Today.
fn load_timeline(ui: &App, storage: &Storage) {
    let echoes = storage.get_all_echoes().expect("Failed to load echoes");
    let today = Local::now().date_naive();

    let mut by_day: BTreeMap<NaiveDate, Vec<Echo>> = BTreeMap::new();
    for echo in echoes.into_iter().filter(is_shown) {
        by_day.entry(echo.day).or_default().push(echo);
    }
    by_day.entry(today).or_default();

    let mut days: Vec<DaySignpost> = Vec::new();
    let mut today_items: Vec<EchoItem> = Vec::new();
    for (day, mut day_echoes) in by_day.into_iter().rev() {
        day_echoes.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let items: Vec<EchoItem> = day_echoes.iter().map(echo_to_item).collect();

        // The Today screen gets its own copy of today's rows.
        if day == today {
            today_items = items.clone();
        }

        days.push(DaySignpost {
            iso: day.to_string().into(),
            label: day.format("%A, %B %e").to_string().into(),
            rel: relative_label(day, today).into(),
            is_today: day == today,
            echoes: ModelRc::new(VecModel::from(items)),
        });
    }

    ui.set_days(ModelRc::new(VecModel::from(days)));
    ui.set_today_echoes(ModelRc::new(VecModel::from(today_items)));
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
        due: SharedString::new(),
        due_label: SharedString::new(),
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
            if let Some(due) = task.due_date {
                item.due = due.to_string().into();
                item.due_label = format_day(due).into();
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_changes_at_each_boundary() {
        assert_eq!(greeting_for(4), "Hello, night owl");
        assert_eq!(greeting_for(5), "Good morning");
        assert_eq!(greeting_for(11), "Good morning");
        assert_eq!(greeting_for(12), "Good afternoon");
        assert_eq!(greeting_for(17), "Good afternoon");
        assert_eq!(greeting_for(18), "Good evening");
        assert_eq!(greeting_for(21), "Good evening");
        assert_eq!(greeting_for(22), "Hello, night owl");
        assert_eq!(greeting_for(0), "Hello, night owl");
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
