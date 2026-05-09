use crate::commands;

pub fn build() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::launcher::search_apps,
            commands::launcher::launch_app,
        ])
}
