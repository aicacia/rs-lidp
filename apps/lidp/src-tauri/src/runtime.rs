use tauri::{Manager, Window, WindowEvent, async_runtime::Mutex};
#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;

use crate::app;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            app::get_storage_bridge_url,
            app::open_storage_bridge_trust_page,
        ])
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

            let app_config =
                app::init_app_config(app.handle(), app.handle().path().app_config_dir()?)?;
            tauri::async_runtime::block_on(app::init_storage_bridge(app.handle()))?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let database = app::init_datebase(app_handle.clone(), app_config.clone())
                    .await
                    .expect("database must initialize");
                let router =
                    app::init_router(app_config, database).expect("router must initialize");
                app_handle.manage(Mutex::new(router));
            });

            app.handle()
                .plugin(tauri_plugin_fetch_api::init(app::request_handler))?;
            Ok(())
        })
        .on_window_event(on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let app_handle = window.app_handle().clone();
        tauri::async_runtime::spawn(async move {
            app::close(&app_handle).await.expect("failed to close app");
            app_handle.exit(0);
        });
    }
}
