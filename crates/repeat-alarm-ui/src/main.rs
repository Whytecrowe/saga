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
    let db_path = "alarms.db";
    let conn = Connection::open(db_path)?;

    // Initialize the database table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS alarms (
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

    // Load initial alarms from the database
    let mut stmt = conn.prepare(
        "SELECT id, name, note, interval_minutes, start_hour, end_hour, is_active FROM alarms",
    )?;
    let initial_alarms_iter = stmt.query_map([], |row| {
        Ok(Alarm {
            id: row.get::<_, String>(0)?.into(),
            name: row.get::<_, String>(1)?.into(),
            note: row.get::<_, String>(2)?.into(),
            interval_minutes: row.get(3)?,
            start_hour: row.get(4)?,
            end_hour: row.get(5)?,
            is_active: row.get(6)?,
        })
    })?;

    let mut initial_alarms = Vec::new();
    for alarm in initial_alarms_iter {
        initial_alarms.push(alarm?);
    }

    let ui = App::new()?;

    // Update UI with initial data
    let model = Rc::new(VecModel::from(initial_alarms.clone()));
    ui.set_alarms(model.into());

    // 1. Thread-safe Storage for our Alarms
    let shared_alarms = Arc::new(Mutex::new(initial_alarms));

    // 2. Background Scheduler Thread
    let alarms_for_thread = Arc::clone(&shared_alarms);
    thread::spawn(move || {
        loop {
            let now = Local::now();
            let current_hour = now.hour();
            let current_minute = now.minute();

            // Lock the mutex to read the current alarms
            if let Ok(alarms) = alarms_for_thread.lock() {
                for alarm in alarms.iter() {
                    if alarm.is_active {
                        if current_hour >= alarm.start_hour as u32
                            && current_hour < alarm.end_hour as u32
                        {
                            // Simple logic: Trigger if the minute is a multiple of the interval
                            if current_minute % (alarm.interval_minutes as u32).max(1) == 0 {
                                let _ = Notification::new()
                                    .summary(&alarm.name)
                                    .body(&alarm.note)
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
    let update_ui_model = move |new_list: Vec<Alarm>| {
        if let Some(ui) = ui_handle_model.upgrade() {
            let model = Rc::new(VecModel::from(new_list));
            ui.set_alarms(model.into());
        }
    };

    // on save
    let update_ui_save = update_ui_model.clone();
    let shared_save = Arc::clone(&shared_alarms);
    ui.on_save_alarm(move |id, name, note, interval, start, end| {
        let mut new_alarms = Vec::new();
        let conn = Connection::open("alarms.db").unwrap();

        if let Ok(mut lock) = shared_save.lock() {
            if id.is_empty() {
                // Create New
                let new_id = Uuid::new_v4().to_string();
                let alarm = Alarm {
                    id: new_id.clone().into(),
                    name: name.clone(),
                    note: note.clone(),
                    interval_minutes: interval,
                    start_hour: start,
                    end_hour: end,
                    is_active: true,
                };
                let _ = conn.execute(
                    "INSERT INTO alarms (id, name, note, interval_minutes, start_hour, end_hour, is_active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![new_id, name.to_string(), note.to_string(), interval, start, end, true],
                );
                lock.push(alarm);
            } else {
                // Edit Existing
                if let Some(alarm) = lock.iter_mut().find(|a| a.id == id) {
                    alarm.name = name.clone();
                    alarm.note = note.clone();
                    alarm.interval_minutes = interval;
                    alarm.start_hour = start;
                    alarm.end_hour = end;
                    let _ = conn.execute(
                        "UPDATE alarms SET name = ?1, note = ?2, interval_minutes = ?3, start_hour = ?4, end_hour = ?5 WHERE id = ?6",
                        params![name.to_string(), note.to_string(), interval, start, end, id.to_string()],
                    );
                }
            }
            new_alarms = lock.clone();
        }
        update_ui_save(new_alarms);
    });

    // on toggle
    let update_ui_toggle = update_ui_model.clone();
    let shared_toggle = Arc::clone(&shared_alarms);
    ui.on_toggle_alarm(move |id, active| {
        let mut new_alarms = Vec::new();
        let conn = Connection::open("alarms.db").unwrap();

        if let Ok(mut lock) = shared_toggle.lock() {
            if let Some(alarm) = lock.iter_mut().find(|a| a.id == id) {
                alarm.is_active = active;
                let _ = conn.execute(
                    "UPDATE alarms SET is_active = ?1 WHERE id = ?2",
                    params![active, id.to_string()],
                );
            }
            new_alarms = lock.clone();
        }
        update_ui_toggle(new_alarms);
    });

    // on delete
    let update_ui_delete = update_ui_model.clone();
    let shared_delete = Arc::clone(&shared_alarms);
    ui.on_delete_alarm(move |id| {
        let mut new_alarms = Vec::new();
        let conn = Connection::open("alarms.db").unwrap();

        if let Ok(mut lock) = shared_delete.lock() {
            lock.retain(|a| a.id != id);
            let _ = conn.execute("DELETE FROM alarms WHERE id = ?1", params![id.to_string()]);
            new_alarms = lock.clone();
        }
        update_ui_delete(new_alarms);
    });

    ui.run()?;
    Ok(())
}
