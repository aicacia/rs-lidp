use std::{fs, path::Path, sync::Arc};

use axum::{Router, http::StatusCode, response::IntoResponse};
use storage_server::{AppConfig, RouterState};
use tauri::{AppHandle, Manager, async_runtime::Mutex};
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

use crate::relay::LocalRelay;

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
        let mut default_config = AppConfig::default();
        default_config.database.url = format!(
            "file://{}",
            data_dir.as_ref().join("lidp.db").to_string_lossy()
        );
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

pub fn init_iroh_rely(app_handle: &AppHandle, _app_config: &AppConfig) -> tauri::Result<()> {
    let data_dir = app_handle.path().data_dir()?;

    let async_app_handle = app_handle.clone();
    let _async_handle = tauri::async_runtime::spawn(async move {
        let relay = LocalRelay::start("storage.local", data_dir)
            .await
            .expect("failed to start local relay");

        async_app_handle.manage(Mutex::new(Some(relay)));
    });

    Ok(())
}

pub async fn shutdown(app_handle: &AppHandle) -> tauri::Result<()> {
    if let Some(relay_mutex) = app_handle.try_state::<Mutex<Option<LocalRelay>>>() {
        if let Some(relay) = relay_mutex.lock().await.take() {
            relay.shutdown().await?;
        }
    }

    Ok(())
}
