use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{Router, http::StatusCode, response::IntoResponse};
use storage_server::{AppConfig, RouterState};
use tauri::{AppHandle, Manager, async_runtime::Mutex};
use tauri_plugin_fetch_api::{Request, Response};
use tauri_plugin_opener::OpenerExt;
use tower_service::Service;

use crate::bridge::{
    StorageBridge, bridge_trust_prompted_path, bridge_trust_url, ensure_storage_bridge_certificate,
};
use crate::bridge_trust::ca_trust_installed_path;

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

async fn bridge_url_for(app_handle: &AppHandle) -> String {
    if let Some(bridge_state) = app_handle.try_state::<Mutex<StorageBridge>>() {
        let bridge = bridge_state.lock().await;
        bridge.url().await
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn get_storage_bridge_url(app_handle: AppHandle) -> String {
    bridge_url_for(&app_handle).await
}

#[tauri::command]
pub async fn get_storage_bridge_trust_url(app_handle: AppHandle) -> String {
    bridge_trust_url(&bridge_url_for(&app_handle).await).unwrap_or_default()
}

#[tauri::command]
pub async fn open_storage_bridge_trust_page(app_handle: AppHandle) -> Result<(), String> {
    let trust_url = bridge_trust_url(&bridge_url_for(&app_handle).await)
        .ok_or("storage bridge URL is not available yet")?;

    app_handle
        .opener()
        .open_url(trust_url, None::<&str>)
        .map_err(|err| err.to_string())
}

async fn prompt_bridge_cert_trust_if_needed(app_handle: AppHandle, data_dir: std::path::PathBuf) {
    if fs::read_to_string(ca_trust_installed_path(&data_dir))
        .ok()
        .as_deref()
        == Some("v3")
    {
        return;
    }

    let prompted_path = bridge_trust_prompted_path(&data_dir);

    for _ in 0..50 {
        let wss_url = bridge_url_for(&app_handle).await;
        if wss_url.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        if prompted_path.exists() {
            return;
        }

        let Some(trust_url) = bridge_trust_url(&wss_url) else {
            return;
        };

        if app_handle
            .opener()
            .open_url(trust_url, None::<&str>)
            .is_err()
        {
            log::warn!("failed to open storage bridge trust page in browser");
            return;
        }

        if fs::write(&prompted_path, b"").is_err() {
            log::warn!(
                "failed to record storage bridge trust prompt at {:?}",
                prompted_path
            );
        }

        return;
    }

    log::warn!("storage bridge URL was not ready for cert trust prompt");
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

    let prompt_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        prompt_bridge_cert_trust_if_needed(prompt_handle, data_dir).await;
    });

    Ok(())
}

pub async fn shutdown(_app_handle: &AppHandle) -> tauri::Result<()> {
    Ok(())
}
