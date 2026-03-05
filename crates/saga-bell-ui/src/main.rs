slint::include_modules!();

use chrono::{Local, Timelike};
use notify_rust::Notification;
use rusqlite::{Connection, Result, params};
use slint::{ComponentHandle, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "bells.db";
    let conn = Connection::open(db_path)?;

    // Initialize the database table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS bells (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            note TEXT NOT NULL,
            interval_minutes INTEGER NOT NULL,
            start_hour INTEGER NOT NULL,
            end_hour INTEGER NOT NULL,
            is_active BOOLEAN NOT NULL
        )",
        [],
    )?;

    // Load initial bells from the database
    let mut stmt = conn.prepare(
        "SELECT id, name, note, interval_minutes, start_hour, end_hour, is_active FROM bells",
    )?;
    let initial_bells_iter = stmt.query_map([], |row| {
        Ok(Bell {
            id: row.get::<_, String>(0)?.into(),
            name: row.get::<_, String>(1)?.into(),
            note: row.get::<_, String>(2)?.into(),
            interval_minutes: row.get(3)?,
            start_hour: row.get(4)?,
            end_hour: row.get(5)?,
            is_active: row.get(6)?,
        })
    })?;

    let mut initial_bells = Vec::new();
    for bell in initial_bells_iter {
        initial_bells.push(bell?);
    }

    let ui = App::new()?;

    // Update UI with initial data
    let model = Rc::new(VecModel::from(initial_bells.clone()));
    ui.set_bells(model.into());

    // 1. Thread-safe Storage for our Bells
    let shared_bells = Arc::new(Mutex::new(initial_bells));

    // 2. Background Scheduler Thread
    let bells_for_thread = Arc::clone(&shared_bells);
    thread::spawn(move || {
        loop {
            let now = Local::now();
            let current_hour = now.hour();
            let current_minute = now.minute();

            // Lock the mutex to read the current bells
            if let Ok(bells) = bells_for_thread.lock() {
                for bell in bells.iter() {
                    if bell.is_active {
                        if current_hour >= bell.start_hour as u32
                            && current_hour < bell.end_hour as u32
                        {
                            // Simple logic: Trigger if the minute is a multiple of the interval
                            if current_minute % (bell.interval_minutes as u32).max(1) == 0 {
                                let _ = Notification::new()
                                    .appname("Saga Bell")
                                    .summary(&bell.name)
                                    .body(&bell.note)
                                    .icon("bell")
                                    .show();
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_secs(60));
        }
    });

    // 3. UI Event Handling
    // This helper updates ONLY the UI model
    let ui_handle_model = ui.as_weak();
    let update_ui_model = move |new_list: Vec<Bell>| {
        if let Some(ui) = ui_handle_model.upgrade() {
            let model = Rc::new(VecModel::from(new_list));
            ui.set_bells(model.into());
        }
    };

    // on save
    let update_ui_save = update_ui_model.clone();
    let shared_save = Arc::clone(&shared_bells);
    ui.on_save_bell(move |id, name, note, interval, start, end| {
        let mut new_bells = Vec::new();
        let conn = Connection::open("bells.db").unwrap();

        if let Ok(mut lock) = shared_save.lock() {
            if id.is_empty() {
                // Create New
                let new_id = Uuid::new_v4().to_string();
                let bell = Bell {
                    id: new_id.clone().into(),
                    name: name.clone(),
                    note: note.clone(),
                    interval_minutes: interval,
                    start_hour: start,
                    end_hour: end,
                    is_active: true,
                };
                let _ = conn.execute(
                    "INSERT INTO bells (id, name, note, interval_minutes, start_hour, end_hour, is_active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![new_id, name.to_string(), note.to_string(), interval, start, end, true],
                );
                lock.push(bell);
            } else {
                // Edit Existing
                if let Some(bell) = lock.iter_mut().find(|a| a.id == id) {
                    bell.name = name.clone();
                    bell.note = note.clone();
                    bell.interval_minutes = interval;
                    bell.start_hour = start;
                    bell.end_hour = end;
                    let _ = conn.execute(
                        "UPDATE bells SET name = ?1, note = ?2, interval_minutes = ?3, start_hour = ?4, end_hour = ?5 WHERE id = ?6",
                        params![name.to_string(), note.to_string(), interval, start, end, id.to_string()],
                    );
                }
            }
            new_bells = lock.clone();
        }
        update_ui_save(new_bells);
    });

    // on toggle
    let update_ui_toggle = update_ui_model.clone();
    let shared_toggle = Arc::clone(&shared_bells);
    ui.on_toggle_bell(move |id, active| {
        let mut new_bells = Vec::new();
        let conn = Connection::open("bells.db").unwrap();

        if let Ok(mut lock) = shared_toggle.lock() {
            if let Some(bell) = lock.iter_mut().find(|a| a.id == id) {
                bell.is_active = active;
                let _ = conn.execute(
                    "UPDATE bells SET is_active = ?1 WHERE id = ?2",
                    params![active, id.to_string()],
                );
            }
            new_bells = lock.clone();
        }
        update_ui_toggle(new_bells);
    });

    // on delete
    let update_ui_delete = update_ui_model.clone();
    let shared_delete = Arc::clone(&shared_bells);
    ui.on_delete_bell(move |id| {
        let mut new_bells = Vec::new();
        let conn = Connection::open("bells.db").unwrap();

        if let Ok(mut lock) = shared_delete.lock() {
            lock.retain(|a| a.id != id);
            let _ = conn.execute("DELETE FROM bells WHERE id = ?1", params![id.to_string()]);
            new_bells = lock.clone();
        }
        update_ui_delete(new_bells);
    });

    ui.run()?;
    Ok(())
}
