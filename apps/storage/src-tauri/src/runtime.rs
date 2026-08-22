use std::path::PathBuf;

use tauri::{Manager, Window, WindowEvent, async_runtime::Mutex};
use tauri_plugin_cli::CliExt;
#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;

use crate::app;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _r: tauri::Result<()> = w.set_focus();
                let _ = _r;
            }
        }));
    }

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_cli::init());
    }

    builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            if cfg!(any(windows, target_os = "linux")) {
                let _ = app.deep_link().register_all();
            }

            let mut config_path = None;
            match app.cli().matches() {
                Ok(matches) => {
                    if let Some(config) = matches.args.get("config")
                        && let Some(path) = config.value.as_str()
                    {
                        config_path = Some(PathBuf::from(path))
                    }
                }
                Err(e) => {
                    log::error!("Failed to parse CLI arguments: {}", e);
                }
            }

            let _app_config = app::init_app_config(app.handle().clone(), config_path)?;

            let router = app::init_router()?;

            app.handle().manage(Mutex::new(router));
            app.handle()
                .plugin(tauri_plugin_fetch_api::init(app::request_handler))?;

            Ok(())
        })
        .on_window_event(on_window_event);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();

            let app_handle = window.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                app::close(&app_handle).await.expect("failed to close app");
                app_handle.exit(0);
            });
        }
        _ => {}
    }
}
