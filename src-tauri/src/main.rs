// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod commands;
mod settings;
mod state;

use screencap_core::ShotKind;
use settings::Settings;
use state::AppState;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn capture_from_shortcut(app: &AppHandle, kind: ShotKind) {
    let app = app.clone();
    std::thread::spawn(move || {
        // no invoke caller to hand the error to — the main window shows it
        if let Err(e) = capture::do_capture(&app, kind, None) {
            show_main_window(&app);
            let _ = app.emit("capture-error", e);
        }
    });
}

pub fn register_shortcuts(app: &AppHandle, s: &Settings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let pairs = [
        (s.shortcut_region.clone(), ShotKind::Region),
        (s.shortcut_window.clone(), ShotKind::Window),
        (s.shortcut_screen.clone(), ShotKind::Screen),
    ];
    for (combo, kind) in pairs {
        gs.on_shortcut(combo.as_str(), move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                capture_from_shortcut(app, kind);
            }
        })
        .map_err(|e| settings::tr(format!("Kürzel „{combo}“: {e}"), format!("Shortcut “{combo}”: {e}")))?;
    }
    Ok(())
}

fn tray_menu<R: tauri::Runtime, M: Manager<R>>(app: &M, s: &Settings) -> tauri::Result<Menu<R>> {
    let de = s.language == "de";
    let item = |id: &str, de_label: &str, en_label: &str| {
        MenuItem::with_id(app, id, if de { de_label } else { en_label }, true, None::<&str>)
    };
    let cap_region = item("cap_region", "Ausschnitt aufnehmen", "Capture region")?;
    let cap_window = item("cap_window", "Fenster aufnehmen", "Capture window")?;
    let cap_screen = item("cap_screen", "Bildschirm aufnehmen", "Capture screen")?;
    let show = item("show", "screencap öffnen", "Open screencap")?;
    let quit = item("quit", "screencap beenden", "Quit screencap")?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    Menu::with_items(
        app,
        &[&cap_region, &cap_window, &cap_screen, &sep, &show, &sep2, &quit],
    )
}

/// Language switch in Settings: relabel the tray menu right away.
pub fn update_tray_language(app: &AppHandle, s: &Settings) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), tray_menu(app, s)) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_tray(app: &tauri::App, s: &Settings) -> tauri::Result<()> {
    let menu = tray_menu(app, s)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("screencap")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "cap_region" => capture_from_shortcut(app, ShotKind::Region),
            "cap_window" => capture_from_shortcut(app, ShotKind::Window),
            "cap_screen" => capture_from_shortcut(app, ShotKind::Screen),
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let (s, existed) = settings::load(&handle);
            settings::set_message_language(&s.language);
            let lib_dir = s.resolved_library_dir();
            // a vanished custom folder (unplugged drive …) must not kill the
            // app at startup — the folder is re-created on first use
            let _ = std::fs::create_dir_all(&lib_dir);
            let lib = screencap_core::load_library_or_recover(&lib_dir.join("index.json"));
            app.manage(AppState {
                lib: Mutex::new(lib),
                settings: Mutex::new(s.clone()),
                region_tx: Mutex::new(None),
                shortcut_error: Mutex::new(None),
                first_run: !existed,
                capturing: std::sync::atomic::AtomicBool::new(false),
                updating: std::sync::atomic::AtomicBool::new(false),
                io: Mutex::new(()),
            });
            if let Err(e) = commands::sync_library_internal(&handle) {
                eprintln!("screencap: {e}");
            }
            build_tray(app, &s)?;
            if let Err(e) = register_shortcuts(&handle, &s) {
                eprintln!("screencap: {e}");
                *app.state::<AppState>().shortcut_error.lock().unwrap() = Some(e);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // main window: closing = hiding (tray app); the region overlay
            // is transient and really closes
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                } else if window.label() == "region" {
                    // overlay closed by the system (Alt+F4 …) = cancel, so the
                    // waiting capture doesn't hang until its timeout
                    let st = window.state::<AppState>();
                    let tx = st.region_tx.lock().unwrap().take();
                    if let Some(tx) = tx {
                        let _ = tx.send(None);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture,
            commands::region_result,
            commands::list_shots,
            commands::get_thumb,
            commands::get_image,
            commands::rename_shot,
            commands::set_tags,
            commands::set_favorite,
            commands::delete_shot,
            commands::reveal_shot,
            commands::copy_shot,
            commands::save_edit,
            commands::import_files,
            commands::sync_library,
            commands::library_path,
            commands::get_settings,
            commands::set_settings,
            commands::app_status,
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building screencap")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app);
            }
            let _ = (app, &event);
        });
}
