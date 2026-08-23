use tauri::{Manager, Window, WindowEvent, async_runtime::Mutex};
#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;

use crate::app::{self, shutdown};

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
                app.deep_link().register_all()?;
            }

            let _app_config =
                app::init_app_config(app.handle(), app.handle().path().app_config_dir()?)?;

            app::init_storage_bridge(app.handle())?;

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
                shutdown(&app_handle).await.expect("failed to shutdown");
                app_handle.exit(0);
            });
        }
        _ => {}
    }
}
