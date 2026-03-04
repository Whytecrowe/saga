slint::include_modules!();

use chrono::{Local, Timelike};
use notify_rust::Notification;
use slint::{ComponentHandle, Model, VecModel};
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
    let ui_handle = ui.as_weak();
    let alarms_for_ui = Arc::clone(&shared_alarms);

    // This helper updates BOTH the UI and the Background Storage
    let sync_and_update = move |new_list: Vec<Alarm>| {
        if let Some(ui) = ui_handle.upgrade() {
            // Update the UI list (what the user sees)
            let model = Rc::new(VecModel::from(new_list.clone()));
            ui.set_alarms(model.into());

            // Update the shared state (what the background thread sees)
            if let Ok(mut lock) = alarms_for_ui.lock() {
                *lock = new_list;
            }
        }
    };

    // on save
    let sync_save = sync_and_update.clone();
    let shared_save = Arc::clone(&shared_alarms);
    ui.on_save_alarm(move |id, name, note, interval, start, end| {
        let mut alarms = if let Ok(lock) = shared_save.lock() {
            lock.clone()
        } else {
            vec![]
        };

        if id.is_empty() {
            // Create New
            alarms.push(Alarm {
                id: Uuid::new_v4().to_string().into(),
                name,
                note,
                interval_minutes: interval,
                start_hour: start,
                end_hour: end,
                is_active: true,
            });
        } else if let Some(alarm) = alarms.iter_mut().find(|a| a.id == id) {
            // Edit Existing
            alarm.name = name;
            alarm.note = note;
            alarm.interval_minutes = interval;
            alarm.start_hour = start;
            alarm.end_hour = end;
        }
        sync_save(alarms);
    });

    // on toggle
    let sync_toggle = sync_and_update.clone();
    let shared_toggle = Arc::clone(&shared_alarms);
    ui.on_toggle_alarm(move |id, active| {
        if let Ok(lock) = shared_toggle.lock() {
            let mut alarms = lock.clone();
            if let Some(alarm) = alarms.iter_mut().find(|a| a.id == id) {
                alarm.is_active = active;
            }
            sync_toggle(alarms);
        }
    });

    // on delete
    let sync_delete = sync_and_update.clone();
    let shared_delete = Arc::clone(&shared_alarms);
    ui.on_delete_alarm(move |id| {
        if let Ok(lock) = shared_delete.lock() {
            let alarms: Vec<Alarm> = lock.iter().filter(|a| a.id != id).cloned().collect();
            sync_delete(alarms);
        }
    });

    ui.run()
}
