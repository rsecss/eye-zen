#![deny(clippy::all)]
#![warn(clippy::pedantic)]

#[tauri::command]
fn get_state_snapshot() -> String {
    "Working".to_string()
}

#[allow(clippy::missing_errors_doc)]
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_state_snapshot])
        .run(tauri::generate_context!())
}
