use std::{fs, sync::Arc};

use axum::{Router, response::IntoResponse};
use lidp_server::AppConfig;
use tauri::{AppHandle, Manager, async_runtime::Mutex, http::StatusCode};
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

mod router;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }
            let app_config = init_app_config(app.handle().clone())?;

            let app_handle = app.handle().clone();
            let _async_handle = tauri::async_runtime::spawn(async move {
                let router = router::init(app_config)
                    .await
                    .expect("router must be initted");
                app_handle.manage(Mutex::new(router));
            });

            app.handle()
                .plugin(tauri_plugin_fetch_api::init(request_handler))?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_app_config(app_handle: AppHandle) -> tauri::Result<Arc<AppConfig>> {
    let config_dir = app_handle.path().app_config_dir()?;

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join("config.yaml");
    log::info!("config path: {:?}", config_path);

    let app_config = if config_path.exists() {
        log::debug!("loading config from {:?}", config_path);
        AppConfig::try_from(config_path.as_path())
            .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?
    } else {
        log::debug!(
            "config file not found, creating default config at {:?}",
            config_path
        );
        let mut default_config = AppConfig::default();
        default_config.database.url =
            format!("file://{}", config_dir.join("lidp.db").to_string_lossy());
        let json_str = yaml_serde::to_string(&default_config)
            .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?;
        fs::write(&config_path, json_str)?;
        default_config
    };

    let app_config = Arc::new(app_config);

    app_handle.manage(app_config.clone());

    Ok(app_config)
}

async fn request_handler(app: AppHandle, request: Request) -> Response {
    let router_state = app.state::<Mutex<Router>>();
    let mut router = router_state.lock().await;

    log::debug!("handling request: {:?}", request);

    match router.call(request).await {
        Ok(response) => {
            log::debug!("request handled successfully: {:?}", response);
            response
        }
        Err(err) => {
            log::error!("error handling request: {}", err);
            let mut response = format!("Internal Server Error: {}", err).into_response();
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
    }
}
