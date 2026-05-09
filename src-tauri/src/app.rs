use crate::commands;
use tauri::http;

pub fn build() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_uri_scheme_protocol("icon", |_ctx, request| {
            let file_path = request.uri().path();

            let (data, mime, status) = match std::fs::read(file_path) {
                Ok(bytes) => (
                    bytes,
                    mime_for_path(file_path),
                    http::StatusCode::OK,
                ),
                Err(_) => (
                    vec![],
                    "text/plain",
                    http::StatusCode::NOT_FOUND,
                ),
            };

            http::Response::builder()
                .status(status)
                .header(http::header::CONTENT_TYPE, mime)
                .header("Cache-Control", "public, max-age=86400")
                .body(data)
                .unwrap()
        })
        .invoke_handler(tauri::generate_handler![
            commands::launcher::search_apps,
            commands::launcher::launch_app,
            commands::launcher::resolve_icon,
            commands::launcher::hide_window,
        ])
}

fn mime_for_path(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".svg") || lower.ends_with(".svgz") {
        "image/svg+xml"
    } else if lower.ends_with(".xpm") {
        "image/x-xpixmap"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}
