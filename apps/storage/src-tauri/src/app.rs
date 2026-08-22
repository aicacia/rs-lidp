use std::{fs, io, path::PathBuf, sync::Arc};

use axum::{Router, http::StatusCode, response::IntoResponse};
use storage_server::{AppConfig, RouterState};
use tauri::{AppHandle, Manager, async_runtime::Mutex};
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

pub fn init_router() -> io::Result<Router> {
    let router_state = RouterState::new("storage://app");

    let openapi_router = storage_server::openapi_router(router_state, "");

    Ok(openapi_router.split_for_parts().0)
}

pub fn init_app_config(
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

pub async fn request_handler(app_handle: AppHandle, request: Request) -> Response {
    let router_state = app_handle.state::<Mutex<Router>>();
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

pub async fn close(_app_handle: &AppHandle) -> io::Result<()> {
    Ok(())
}
