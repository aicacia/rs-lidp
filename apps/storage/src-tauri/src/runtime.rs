use std::{fs, path::PathBuf, sync::Arc};

use axum::{Router, response::IntoResponse};
use storage_server::AppConfig;
use tauri::{AppHandle, Manager, async_runtime::Mutex, http::StatusCode};
use tauri_plugin_cli::CliExt;
#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

use crate::router;

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

            let app_config = init_app_config(app.handle().clone(), config_path)?;

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
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_app_config(
    app_handle: AppHandle,
    config_path: Option<PathBuf>,
) -> tauri::Result<Arc<AppConfig>> {
    let config_dir = app_handle.path().app_config_dir()?;

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = if let Some(config_path) = config_path {
        config_path
    } else {
        config_dir.join("config.yaml")
    };
    log::debug!("config path: {:?}", config_path);

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
