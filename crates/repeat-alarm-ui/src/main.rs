slint::include_modules!();

use chrono::{Local, Timelike};
use notify_rust::Notification;
use slint::{ComponentHandle, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

fn main() -> Result<(), slint::PlatformError> {
    let ui = App::new()?;

    // 1. Thread-safe Storage for our Alarms
    let shared_alarms = Arc::new(Mutex::new(Vec::<Alarm>::new()));

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
        if let Ok(mut lock) = shared_save.lock() {
            if id.is_empty() {
                // Create New
                lock.push(Alarm {
                    id: Uuid::new_v4().to_string().into(),
                    name,
                    note,
                    interval_minutes: interval,
                    start_hour: start,
                    end_hour: end,
                    is_active: true,
                });
            } else if let Some(alarm) = lock.iter_mut().find(|a| a.id == id) {
                // Edit Existing
                alarm.name = name;
                alarm.note = note;
                alarm.interval_minutes = interval;
                alarm.start_hour = start;
                alarm.end_hour = end;
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
        if let Ok(mut lock) = shared_toggle.lock() {
            if let Some(alarm) = lock.iter_mut().find(|a| a.id == id) {
                alarm.is_active = active;
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
        if let Ok(mut lock) = shared_delete.lock() {
            lock.retain(|a| a.id != id);
            new_alarms = lock.clone();
        }
        update_ui_delete(new_alarms);
    });

    ui.run()
}
