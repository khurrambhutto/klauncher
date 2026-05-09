use crate::core::item::{ItemKind, LauncherItem};
use crate::launcher::ranking;
use crate::platform::linux::desktop_entries::DesktopApp;
use crate::platform::linux::icon_resolver;

pub fn search_apps(apps: &[DesktopApp], query: &str, limit: usize) -> Vec<LauncherItem> {
    ranking::rank_apps(apps, query)
        .into_iter()
        .take(limit)
        .map(|ranked| {
            let resolved_icon = ranked
                .app
                .icon
                .as_deref()
                .and_then(icon_resolver::resolve_icon);

            LauncherItem {
                id: ranked.app.id,
                title: ranked.app.name,
                subtitle: ranked.app.comment.or(ranked.app.generic_name),
                icon: resolved_icon,
                kind: ItemKind::App,
            }
        })
        .collect()
}
