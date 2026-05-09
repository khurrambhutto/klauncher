use crate::core::item::LauncherItem;
use crate::error::{AppError, AppResult};
use crate::launcher::search;
use crate::platform::linux::desktop_entries::{discover_desktop_apps, DesktopApp};
use crate::platform::linux::open;

pub struct AppProvider {
    apps: Vec<DesktopApp>,
}

impl AppProvider {
    pub fn load() -> AppResult<Self> {
        Ok(Self {
            apps: discover_desktop_apps()?,
        })
    }

    pub fn search(&self, query: &str) -> AppResult<Vec<LauncherItem>> {
        Ok(search::search_apps(&self.apps, query, 20))
    }

    pub fn launch(&self, app_id: &str) -> AppResult<()> {
        let app = self
            .apps
            .iter()
            .find(|app| app.id == app_id)
            .ok_or_else(|| AppError::NotFound(format!("App not found: {app_id}")))?;

        open::launch_desktop_app(app)
    }
}
