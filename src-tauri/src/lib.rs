mod app;
mod commands;
mod core;
mod error;
mod launcher;
mod platform;
mod providers;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::build()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
