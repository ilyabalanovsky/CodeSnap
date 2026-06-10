use base64::{engine::general_purpose, Engine as _};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{
    menu::MenuBuilder,
    path::BaseDirectory,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

const DEFAULT_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+S";
const CAPTURE_EVENT: &str = "codesnap://code-captured";
const UI_HIDDEN_EVENT: &str = "codesnap://ui-hidden";
const UI_SHOWN_EVENT: &str = "codesnap://ui-shown";
const SETTINGS_FILE_NAME: &str = "settings.json";

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

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct AppSettings {
    capture_hotkey: String,
    launch_at_login: bool,
    start_in_tray: bool,
    disable_animations: bool,
    welcome_completed: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            capture_hotkey: DEFAULT_CAPTURE_SHORTCUT.to_string(),
            launch_at_login: false,
            start_in_tray: false,
            disable_animations: false,
            welcome_completed: false,
        }
    }
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
        emit_ui_hidden(&app);
        window.hide().map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    let mut settings = read_app_settings(&app);

    if let Ok(is_enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_login = is_enabled;
    }

    Ok(settings)
}

#[tauri::command]
fn set_app_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let settings = normalize_app_settings(settings);

    apply_app_settings(&app, &settings)?;
    write_app_settings(&app, &settings)?;

    Ok(settings)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let settings = read_app_settings(app.handle());

            setup_tray(app.handle(), &settings.capture_hotkey)?;
            register_capture_shortcut(app.handle(), &settings.capture_hotkey)?;
            setup_close_to_tray(app.handle());
            apply_startup_visibility(app.handle(), settings.start_in_tray);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_text,
            copy_selected_text_then_open,
            save_snapshot_png,
            copy_snapshot_png,
            hide_to_tray,
            get_app_settings,
            set_app_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodeSnap");
}

fn setup_tray(app: &AppHandle, capture_shortcut: &str) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("show", "Open CodeSnap")
        .text("capture", "Capture selected code")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut tray = TrayIconBuilder::with_id("codesnap")
        .tooltip(format!("CodeSnap - {capture_shortcut}"))
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

fn register_capture_shortcut(app: &AppHandle, capture_shortcut: &str) -> Result<(), String> {
    Shortcut::from_str(capture_shortcut).map_err(|error| error.to_string())?;

    let is_capturing = Arc::new(AtomicBool::new(false));

    app.global_shortcut()
        .unregister_all()
        .map_err(|error| error.to_string())?;

    app.global_shortcut()
        .on_shortcut(capture_shortcut, move |app, _shortcut, event| {
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
        .map_err(|error| error.to_string())?;

    if let Some(tray) = app.tray_by_id("codesnap") {
        let _ = tray.set_tooltip(Some(format!("CodeSnap - {capture_shortcut}")));
    }

    Ok(())
}

fn setup_close_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let hide_window = window.clone();
        window.on_window_event(move |window_event| {
            if let WindowEvent::CloseRequested { api, .. } = window_event {
                api.prevent_close();
                emit_ui_hidden(&app_handle);
                let _ = hide_window.hide();
            }
        });
    }
}

fn apply_startup_visibility(app: &AppHandle, start_in_tray: bool) {
    if !start_in_tray {
        return;
    }

    if let Some(window) = app.get_webview_window("main") {
        emit_ui_hidden(app);
        let _ = window.hide();
    }
}

fn apply_app_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    register_capture_shortcut(app, &settings.capture_hotkey)?;

    if settings.launch_at_login {
        app.autolaunch()
            .enable()
            .map_err(|error| error.to_string())?;
    } else {
        match app.autolaunch().is_enabled() {
            Ok(true) => app
                .autolaunch()
                .disable()
                .map_err(|error| error.to_string())?,
            Ok(false) => {}
            Err(error) => {
                println!("CodeSnap could not check autostart state: {error}");
            }
        }
    }

    Ok(())
}

fn normalize_app_settings(settings: AppSettings) -> AppSettings {
    let capture_hotkey = settings.capture_hotkey.trim();

    AppSettings {
        capture_hotkey: if capture_hotkey.is_empty() {
            DEFAULT_CAPTURE_SHORTCUT.to_string()
        } else {
            capture_hotkey.to_string()
        },
        launch_at_login: settings.launch_at_login,
        start_in_tray: settings.start_in_tray,
        disable_animations: settings.disable_animations,
        welcome_completed: settings.welcome_completed,
    }
}

fn read_app_settings(app: &AppHandle) -> AppSettings {
    let Ok(path) = app_settings_path(app) else {
        return AppSettings::default();
    };

    let Ok(raw_settings) = std::fs::read_to_string(path) else {
        return AppSettings::default();
    };

    serde_json::from_str::<AppSettings>(&raw_settings)
        .map(normalize_app_settings)
        .unwrap_or_default()
}

fn write_app_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = app_settings_path(app)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(path, serialized).map_err(|error| error.to_string())
}

fn app_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve(SETTINGS_FILE_NAME, BaseDirectory::AppConfig)
        .map_err(|error| error.to_string())
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
    emit_ui_shown(app);

    Ok(())
}

fn emit_ui_hidden(app: &AppHandle) {
    let _ = app.emit(UI_HIDDEN_EVENT, ());
}

fn emit_ui_shown(app: &AppHandle) {
    let _ = app.emit(UI_SHOWN_EVENT, ());
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
