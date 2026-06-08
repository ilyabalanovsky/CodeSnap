use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveSnapshotResult {
    saved: bool,
    path: Option<String>,
}

#[tauri::command]
fn get_clipboard_text() -> String {
    String::new()
}

#[tauri::command]
fn copy_selected_text_then_open() -> String {
    String::new()
}

#[tauri::command]
async fn save_snapshot_png(app: AppHandle, data_url: String, suggested_file_name: String) -> Result<SaveSnapshotResult, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("PNG image", &["png"])
        .set_file_name(normalize_png_file_name(&suggested_file_name))
        .blocking_save_file();

    let Some(file_path) = file_path else {
        return Ok(SaveSnapshotResult {
            saved: false,
            path: None,
        });
    };

    let path = file_path.into_path().map_err(|error| error.to_string())?;
    let png_bytes = decode_png_data_url(&data_url)?;

    std::fs::write(&path, png_bytes).map_err(|error| error.to_string())?;

    Ok(SaveSnapshotResult {
        saved: true,
        path: Some(path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn copy_snapshot_png(_data_url: String) -> bool {
    false
}

#[tauri::command]
fn hide_to_tray(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| error.to_string())?;
    }

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_clipboard_text,
            copy_selected_text_then_open,
            save_snapshot_png,
            copy_snapshot_png,
            hide_to_tray
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodeSnap");
}

fn normalize_png_file_name(file_name: &str) -> String {
    let trimmed = file_name.trim();
    let base_name = if trimmed.is_empty() { "codesnap.png" } else { trimmed };

    if base_name.to_lowercase().ends_with(".png") {
        base_name.to_string()
    } else {
        format!("{base_name}.png")
    }
}

fn decode_png_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

    let encoded = data_url
        .strip_prefix(PNG_DATA_URL_PREFIX)
        .ok_or_else(|| "Expected a PNG data URL".to_string())?;

    general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| error.to_string())
}
