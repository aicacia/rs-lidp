use std::{fs, path::Path, sync::Arc};

use axum::{Router, http::StatusCode, response::IntoResponse};
use storage_server::{AppConfig, RouterState};
use tauri::{AppHandle, Manager, async_runtime::Mutex};
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

use crate::bridge::{StorageBridge, ensure_storage_bridge_certificate};

pub fn init_router() -> tauri::Result<Router> {
    let router_state = RouterState::new("storage://app");

    let openapi_router = storage_server::openapi_router(router_state, "");

    Ok(openapi_router.split_for_parts().0)
}

pub fn init_app_config(
    app_handle: &AppHandle,
    data_dir: impl AsRef<Path>,
) -> tauri::Result<Arc<AppConfig>> {
    if !data_dir.as_ref().exists() {
        fs::create_dir_all(&data_dir)?;
    }

    let config_path = data_dir.as_ref().join("config.yaml");
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
        let default_config = AppConfig::default();
        let default_config_yaml = yaml_serde::to_string(&default_config)
            .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?;
        fs::write(&config_path, default_config_yaml)?;
        default_config
    };

    let app_config = Arc::new(app_config);

    app_handle.manage(app_config.clone());

    Ok(app_config)
}

pub async fn request_handler(app_handle: AppHandle, request: Request) -> Response {
    // Handle bridge URL endpoint
    let path = request.uri().path();
    if path.contains("/bridge-url") {
        let bridge_url = if let Some(bridge_state) = app_handle.try_state::<Mutex<StorageBridge>>()
        {
            let bridge = bridge_state.lock().await;
            bridge.url().await
        } else {
            String::new()
        };

        let json_response = format!(r#"{{"bridgeUrl":"{}"}}"#, bridge_url);
        let mut response = json_response.into_response();
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        return response;
    }

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

#[tauri::command]
pub async fn get_storage_bridge_url(app_handle: AppHandle) -> String {
    if let Some(bridge_state) = app_handle.try_state::<Mutex<StorageBridge>>() {
        let bridge = bridge_state.lock().await;
        bridge.url().await
    } else {
        String::new()
    }
}

pub fn init_storage_bridge(app_handle: &AppHandle) -> tauri::Result<()> {
    let data_dir = app_handle.path().app_data_dir()?;
    let _ = tauri::async_runtime::block_on(ensure_storage_bridge_certificate(&data_dir))
        .map_err(|err| tauri::Error::Io(std::io::Error::other(err)))?;

    let files_dir = data_dir.join("files");
    if !files_dir.exists() {
        fs::create_dir_all(&files_dir)?;
    }

    let bridge = StorageBridge::new(files_dir);
    let server_bridge = bridge.clone();
    let bridge_data_dir = data_dir.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(err) = server_bridge.start_server(&bridge_data_dir).await {
            log::error!("storage websocket bridge failed to start: {err}");
        }
    });

    app_handle.manage(Mutex::new(bridge));
    Ok(())
}

pub async fn shutdown(_app_handle: &AppHandle) -> tauri::Result<()> {
    Ok(())
}
