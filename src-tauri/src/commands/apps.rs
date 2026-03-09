use crate::output::{list_running_apps, RunningApp};

#[tauri::command]
pub fn get_running_apps() -> Vec<RunningApp> {
    list_running_apps()
}
