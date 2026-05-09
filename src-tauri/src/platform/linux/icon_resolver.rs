use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const ICON_SIZES: &[&str] = &[
    "48x48", "64x64", "128x128", "32x32", "24x24", "22x22", "16x16", "256x256", "512x512",
    "scalable",
];

const ICON_EXTENSIONS: &[&str] = &["png", "svg", "xpm"];

static RESOLVED_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
static THEME_NAME: OnceLock<Option<String>> = OnceLock::new();
static BASE_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    RESOLVED_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn theme_name() -> &'static Option<String> {
    THEME_NAME.get_or_init(detect_icon_theme)
}

fn base_dirs() -> &'static Vec<PathBuf> {
    BASE_DIRS.get_or_init(|| {
        let mut dirs = Vec::new();
        if let Ok(home) = env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/icons"));
            dirs.push(PathBuf::from(&home).join(".icons"));
        }
        let data_dirs = env::var("XDG_DATA_DIRS")
            .map(|value| env::split_paths(&value).collect::<Vec<_>>())
            .unwrap_or_else(|_| vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]);
        for data_dir in data_dirs {
            dirs.push(data_dir.join("icons"));
        }
        dirs
    })
}

pub fn resolve_icon(icon_value: &str) -> Option<String> {
    if icon_value.is_empty() {
        return None;
    }

    if let Some(cached) = cache().lock().unwrap().get(icon_value) {
        return cached.clone();
    }

    let result = resolve_icon_uncached(icon_value);
    cache().lock().unwrap().insert(icon_value.to_string(), result.clone());
    result
}

fn resolve_icon_uncached(icon_value: &str) -> Option<String> {
    let path = Path::new(icon_value);
    if path.is_absolute() {
        return resolve_absolute_icon(path);
    }

    let mut themes = Vec::new();
    if let Some(current) = theme_name() {
        themes.push(current.as_str());
    }
    themes.push("hicolor");

    for theme in &themes {
        for basedir in base_dirs() {
            let theme_root = basedir.join(theme);
            if !theme_root.is_dir() {
                continue;
            }
            if let Some(found) = lookup_in_theme(&theme_root, icon_value, base_dirs(), &mut HashMap::new()) {
                return Some(found);
            }
        }
    }

    lookup_in_pixmaps(icon_value)
}

fn resolve_absolute_icon(path: &Path) -> Option<String> {
    if path.is_file() {
        return Some(path.to_string_lossy().into_owned());
    }

    for ext in ICON_EXTENSIONS {
        let mut p = path.to_path_buf();
        p.set_extension(ext);
        if p.is_file() {
            return Some(p.to_string_lossy().into_owned());
        }
    }

    None
}

fn detect_icon_theme() -> Option<String> {
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout)
                .trim()
                .trim_matches('\'')
                .to_string();
            if !value.is_empty() {
                let exists = base_dirs().iter().any(|dir| dir.join(&value).is_dir());
                if exists {
                    return Some(value);
                }
            }
        }
    }

    if let Ok(home) = env::var("HOME") {
        let settings_ini = PathBuf::from(&home).join(".config/gtk-3.0/settings.ini");
        if let Ok(content) = fs::read_to_string(&settings_ini) {
            for line in content.lines() {
                if let Some(value) = line.strip_prefix("gtk-icon-theme-name=") {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }

    None
}

fn lookup_in_theme(
    theme_root: &Path,
    icon_name: &str,
    basedirs: &[PathBuf],
    parent_cache: &mut HashMap<PathBuf, Vec<String>>,
) -> Option<String> {
    let parents = if let Some(cached) = parent_cache.get(theme_root) {
        cached.clone()
    } else {
        let parents = parse_theme_parents(&theme_root.join("index.theme"));
        parent_cache.insert(theme_root.to_path_buf(), parents.clone());
        parents
    };

    for size in ICON_SIZES {
        let dir = theme_root.join(size).join("apps");
        if !dir.is_dir() {
            continue;
        }
        for ext in ICON_EXTENSIONS {
            let path = dir.join(format!("{icon_name}.{ext}"));
            if path.is_file() {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }

    for parent_name in &parents {
        for basedir in basedirs {
            let parent_root = basedir.join(parent_name);
            if parent_root.is_dir() && parent_root != theme_root {
                if let Some(found) = lookup_in_theme(&parent_root, icon_name, basedirs, parent_cache) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn parse_theme_parents(index_theme_path: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(index_theme_path) else {
        return Vec::new();
    };

    let mut in_icon_theme = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.eq_ignore_ascii_case("[Icon Theme]") {
            in_icon_theme = true;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            break;
        }
        if !in_icon_theme {
            continue;
        }
        if let Some(value) = line.strip_prefix("Inherits=") {
            return value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    Vec::new()
}

fn lookup_in_pixmaps(icon_name: &str) -> Option<String> {
    let pixmaps = PathBuf::from("/usr/share/pixmaps");

    for ext in ICON_EXTENSIONS {
        let path = pixmaps.join(format!("{icon_name}.{ext}"));
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    let path = pixmaps.join(icon_name);
    if path.is_file() {
        return Some(path.to_string_lossy().into_owned());
    }

    None
}
