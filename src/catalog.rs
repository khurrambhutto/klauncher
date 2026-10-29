use gio::prelude::*;
use serde::Serialize;
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub description: String,
    pub executable: String,
    pub desktop_file: PathBuf,
    pub icon: Option<String>,
}

pub fn discover() -> Vec<Application> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for info in gio::AppInfo::all() {
        if !info.should_show() {
            continue;
        }
        let Some(id) = info.id().map(|id| id.to_string()) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let Ok(desktop) = info.clone().downcast::<gio::DesktopAppInfo>() else {
            continue;
        };
        let Some(desktop_file) = desktop.filename() else {
            continue;
        };
        apps.push(Application {
            id,
            name: info.display_name().to_string(),
            description: info
                .description()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            executable: info.executable().to_string_lossy().into_owned(),
            desktop_file,
            icon: info
                .icon()
                .and_then(|icon| gio::prelude::IconExt::to_string(&icon))
                .map(|s| s.to_string()),
        });
    }
    apps.sort_by_cached_key(|app| app.name.to_lowercase());
    apps
}

pub fn search(apps: &[Application], query: &str) -> Vec<usize> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return (0..apps.len()).collect();
    }
    let mut matches: Vec<(usize, i64)> = apps
        .iter()
        .enumerate()
        .filter_map(|(index, app)| {
            let name = app.name.to_lowercase();
            let executable = app.executable.to_lowercase();
            let description = app.description.to_lowercase();
            let mut score = 0;
            for term in &terms {
                score += if name == *term {
                    1000
                } else if name.starts_with(term) {
                    700
                } else if name.split_whitespace().any(|word| word.starts_with(term)) {
                    500
                } else if name.contains(term) {
                    300
                } else if executable.contains(term) {
                    150
                } else if description.contains(term) {
                    50
                } else if subsequence(term, &name) {
                    20
                } else {
                    return None;
                };
            }
            Some((index, score))
        })
        .collect();
    matches.sort_by(|(left, ls), (right, rs)| rs.cmp(ls).then_with(|| left.cmp(right)));
    matches.into_iter().map(|(index, _)| index).collect()
}

fn subsequence(needle: &str, haystack: &str) -> bool {
    let mut remaining = needle.chars();
    let mut next = remaining.next();
    for ch in haystack.chars() {
        if next == Some(ch) {
            next = remaining.next();
        }
    }
    next.is_none()
}

pub fn resolve_icon(icon: Option<&str>) -> Option<PathBuf> {
    let icon = icon?;
    let absolute = PathBuf::from(icon);
    if absolute.is_absolute() && absolute.is_file() {
        return Some(absolute);
    }
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/icons"));
    }
    roots.extend(
        [
            "/usr/share/icons",
            "/usr/local/share/icons",
            "/var/lib/flatpak/exports/share/icons",
        ]
        .map(PathBuf::from),
    );
    for name in icon.split_whitespace() {
        for root in &roots {
            for theme in ["hicolor", "Yaru", "Adwaita"] {
                for size in ["48x48", "scalable", "64x64", "128x128", "256x256"] {
                    for extension in ["png", "svg"] {
                        let path = root
                            .join(theme)
                            .join(size)
                            .join("apps")
                            .join(format!("{name}.{extension}"));
                        if path.is_file() {
                            return Some(path);
                        }
                    }
                }
            }
        }
        let path = PathBuf::from("/usr/share/pixmaps").join(format!("{name}.png"));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app(name: &str, description: &str) -> Application {
        Application {
            id: name.into(),
            name: name.into(),
            description: description.into(),
            executable: name.to_lowercase(),
            desktop_file: PathBuf::new(),
            icon: None,
        }
    }
    #[test]
    fn exact_and_prefix_names_rank_above_description_matches() {
        let apps = vec![
            app("Other", "A terminal emulator"),
            app("Terminal Tools", ""),
            app("Terminal", ""),
        ];
        assert_eq!(search(&apps, "terminal"), vec![2, 1, 0]);
    }
    #[test]
    fn every_term_must_match_and_blank_query_preserves_order() {
        let apps = vec![app("Visual Studio Code", "Editor"), app("Studio", "Audio")];
        assert_eq!(search(&apps, "studio editor"), vec![0]);
        assert!(search(&apps, "studio missing").is_empty());
        assert_eq!(search(&apps, "  "), vec![0, 1]);
    }
    #[test]
    fn unicode_subsequence_does_not_split_utf8() {
        assert!(subsequence("é文", "Café 文書"));
        assert!(!subsequence("文é", "Café 文書"));
    }
}
