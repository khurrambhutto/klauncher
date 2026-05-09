use crate::core::item::LauncherItem;
use crate::error::AppResult;
use crate::providers::apps::AppProvider;

#[tauri::command]
pub fn search_apps(query: String) -> Result<Vec<LauncherItem>, String> {
    run_search_apps(&query).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn launch_app(app_id: String) -> Result<(), String> {
    run_launch_app(&app_id).map_err(|error| error.to_string())
}

fn run_search_apps(query: &str) -> AppResult<Vec<LauncherItem>> {
    AppProvider::load()?.search(query)
}

fn run_launch_app(app_id: &str) -> AppResult<()> {
    AppProvider::load()?.launch(app_id)
}
