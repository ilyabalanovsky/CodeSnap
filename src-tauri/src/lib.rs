use base64::{engine::general_purpose, Engine as _};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const CAPTURE_SHORTCUT: &str = "Ctrl+Shift+S";
const CAPTURE_EVENT: &str = "codesnap://code-captured";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveSnapshotResult {
    saved: bool,
    path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapturedCodePayload {
    code: String,
    source: &'static str,
}

#[tauri::command]
fn get_clipboard_text(app: AppHandle) -> String {
    read_clipboard_text(&app).unwrap_or_default()
}

#[tauri::command]
fn copy_selected_text_then_open(app: AppHandle) -> String {
    capture_selection_and_open(app, "manual").unwrap_or_default()
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
        .setup(|app| {
            setup_tray(app.handle())?;
            register_capture_shortcut(app.handle())?;
            setup_close_to_tray(app.handle());

            Ok(())
        })
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

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("show", "Open CodeSnap")
        .text("capture", "Capture selected code")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut tray = TrayIconBuilder::with_id("codesnap")
        .tooltip(format!("CodeSnap - {CAPTURE_SHORTCUT}"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let _ = show_main_window(app);
            }
            "capture" => {
                let app = app.clone();
                thread::spawn(move || {
                    let _ = capture_selection_and_open(app, "tray");
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;

    Ok(())
}

fn register_capture_shortcut(app: &AppHandle) -> tauri::Result<()> {
    let is_capturing = Arc::new(AtomicBool::new(false));

    app.global_shortcut()
        .on_shortcut(CAPTURE_SHORTCUT, move |app, _shortcut, event| {
            println!("CodeSnap capture shortcut event: {:?}", event.state);

            if event.state != tauri_plugin_global_shortcut::ShortcutState::Released {
                return;
            }

            if is_capturing.swap(true, Ordering::SeqCst) {
                return;
            }

            let is_capturing = Arc::clone(&is_capturing);
            let app = app.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(120));
                let _ = capture_selection_and_open(app, "hotkey");
                thread::sleep(Duration::from_millis(250));
                is_capturing.store(false, Ordering::SeqCst);
            });
        })
        .map_err(|error| tauri::Error::Anyhow(error.into()))
}

fn setup_close_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let hide_window = window.clone();
        window.on_window_event(move |window_event| {
            if let WindowEvent::CloseRequested { api, .. } = window_event {
                api.prevent_close();
                let _ = hide_window.hide();
            }
        });
    }
}

fn capture_selection_and_open(app: AppHandle, source: &'static str) -> Result<String, String> {
    send_copy_shortcut()?;
    thread::sleep(Duration::from_millis(160));

    let code = read_clipboard_text(&app)?;
    println!("CodeSnap captured clipboard text length: {}", code.len());

    if code.trim().is_empty() {
        show_main_window(&app)?;
        return Ok(String::new());
    }

    show_main_window(&app)?;
    app.emit(
        CAPTURE_EVENT,
        CapturedCodePayload {
            code: code.clone(),
            source,
        },
    )
    .map_err(|error| error.to_string())?;
    println!("CodeSnap emitted captured code event from {source}");

    Ok(code)
}

fn show_main_window(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;

    Ok(())
}

fn read_clipboard_text(app: &AppHandle) -> Result<String, String> {
    app.clipboard().read_text().map_err(|error| error.to_string())
}

fn send_copy_shortcut() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|error| error.to_string())?;
    press_copy_key(&mut enigo)?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn press_copy_key(enigo: &mut Enigo) -> Result<(), String> {
    enigo
        .key(Key::C, Direction::Click)
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "windows"))]
fn press_copy_key(enigo: &mut Enigo) -> Result<(), String> {
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|error| error.to_string())
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
