use crate::core::item::{ItemKind, LauncherItem};
use crate::launcher::ranking;
use crate::platform::linux::desktop_entries::DesktopApp;

pub fn search_apps(apps: &[DesktopApp], query: &str, limit: usize) -> Vec<LauncherItem> {
    ranking::rank_apps(apps, query)
        .into_iter()
        .take(limit)
        .map(|ranked| LauncherItem {
            id: ranked.app.id,
            title: ranked.app.name,
            subtitle: ranked.app.comment.or(ranked.app.generic_name),
            icon: ranked.app.icon,
            kind: ItemKind::App,
        })
        .collect()
}
