use crate::error::AppResult;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_BLACKLISTED_APP_DIRS: &[&str] = &[
    "/usr/share/locale",
    "/usr/share/app-install",
    "/usr/share/kservices5",
    "/usr/share/kf5",
    "/usr/share/kservicetypes5",
    "/usr/share/applications/screensavers",
    "/usr/share/kde4",
    "/usr/share/mimelnk",
];

#[derive(Clone, Debug)]
pub struct DesktopApp {
    pub id: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub keywords: Vec<String>,
    pub terminal: bool,
    pub no_display: bool,
}

pub fn discover_desktop_apps() -> AppResult<Vec<DesktopApp>> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();

    for directory in application_dirs() {
        if !directory.is_dir() {
            continue;
        }

        if is_blacklisted_app_dir(&directory) {
            continue;
        }

        collect_desktop_files(&directory, &mut |path| {
            if is_blacklisted_app_dir(path) {
                return;
            }

            let Some(id) = desktop_id(&directory, path) else {
                return;
            };

            if !seen.insert(id.clone()) {
                return;
            }

            if let Ok(Some(app)) = parse_desktop_app(&id, path) {
                apps.push(app);
            }
        })?;
    }

    apps.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(apps)
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });

    dirs.extend(data_dirs.into_iter().map(|path| path.join("applications")));
    dirs
}

fn is_blacklisted_app_dir(path: &Path) -> bool {
    DEFAULT_BLACKLISTED_APP_DIRS
        .iter()
        .map(Path::new)
        .any(|blacklisted| path.starts_with(blacklisted))
}

fn collect_desktop_files(directory: &Path, visit: &mut impl FnMut(&Path)) -> AppResult<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if !is_blacklisted_app_dir(&path) {
                collect_desktop_files(&path, visit)?;
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "desktop")
        {
            visit(&path);
        }
    }

    Ok(())
}

fn desktop_id(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    Some(
        relative
            .to_string_lossy()
            .replace('/', "-")
            .trim_end_matches(".desktop")
            .to_string(),
    )
}

fn parse_desktop_app(id: &str, path: &Path) -> AppResult<Option<DesktopApp>> {
    let content = fs::read_to_string(path)?;
    let entry = parse_desktop_entry(&content);

    if entry
        .get("Type")
        .is_some_and(|value| value != "Application")
    {
        return Ok(None);
    }

    if entry.get("Hidden").is_some_and(|value| parse_bool(value)) {
        return Ok(None);
    }

    if entry
        .get("OnlyShowIn")
        .is_some_and(|value| !show_in_current_desktop(value))
    {
        return Ok(None);
    }

    if entry
        .get("NotShowIn")
        .is_some_and(|value| show_in_current_desktop(value))
    {
        return Ok(None);
    }

    let Some(name) = entry.get("Name").filter(|name| !name.trim().is_empty()) else {
        return Ok(None);
    };

    let Some(exec) = entry.get("Exec").filter(|exec| !exec.trim().is_empty()) else {
        return Ok(None);
    };

    Ok(Some(DesktopApp {
        id: id.to_string(),
        name: name.to_string(),
        generic_name: entry.get("GenericName").cloned(),
        comment: entry.get("Comment").cloned(),
        exec: exec.to_string(),
        icon: entry.get("Icon").cloned(),
        keywords: entry
            .get("Keywords")
            .map(|value| {
                value
                    .split(';')
                    .filter(|keyword| !keyword.trim().is_empty())
                    .map(|keyword| keyword.trim().to_string())
                    .collect()
            })
            .unwrap_or_default(),
        terminal: entry.get("Terminal").is_some_and(|value| parse_bool(value)),
        no_display: entry
            .get("NoDisplay")
            .is_some_and(|value| parse_bool(value)),
    }))
}

fn parse_desktop_entry(content: &str) -> HashMap<String, String> {
    let mut in_desktop_entry = false;
    let mut values = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.contains('[') {
            continue;
        }

        values.insert(key.to_string(), unescape_value(value));
    }

    values
}

fn parse_bool(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

fn show_in_current_desktop(value: &str) -> bool {
    let current_desktop = env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    if current_desktop.is_empty() {
        return false;
    }

    value
        .split(';')
        .filter(|desktop| !desktop.is_empty())
        .any(|desktop| current_desktop.contains(&desktop.to_lowercase()))
}

fn unescape_value(value: &str) -> String {
    value
        .replace("\\s", " ")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\\", "\\")
}
