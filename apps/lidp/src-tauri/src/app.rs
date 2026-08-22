use std::{fs, io, path::Path, sync::Arc};

use axum::{Router, http::StatusCode, response::IntoResponse};
use db::{close_database, open_database};
use libsql::Database;
use lidp_server::{AppConfig, RouterState};
use lidp_service::{
    bootstrap::BootstrapService,
    oauth2::OAuth2Service,
    repo::{
        KeyService, LibSqlApplicationRepo, LibSqlClientRepo, LibSqlKeyRepo,
        LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo, LibSqlPermissionRepo,
        LibSqlRoleRepo, LibSqlUserRepo, PrivateKeyKeyringRepo,
    },
};
use tauri::{AppHandle, Manager, async_runtime::Mutex};
use tauri_plugin_fetch_api::{Request, Response};
use tower_service::Service;

pub fn init_router(app_config: Arc<AppConfig>, database: Arc<Database>) -> io::Result<Router> {
    let key_service = Arc::new(KeyService::new(
        LibSqlKeyRepo::new(database.clone()),
        PrivateKeyKeyringRepo::new(&app_config.oauth2.issuer),
        app_config.key_namespace.clone(),
    ));

    let oauth2_config = app_config.oauth2.clone();
    let oauth2_service = Arc::new(OAuth2Service::new(
        LibSqlClientRepo::new(database.clone(), key_service.clone()),
        LibSqlOAuth2AuthorizationCodeRepo::new(database.clone()),
        LibSqlUserRepo::new(
            database.clone(),
            key_service.clone(),
            app_config.password.clone(),
        ),
        LibSqlOAuth2UserConsentRepo::new(database.clone()),
        key_service,
        oauth2_config,
        app_config.key_namespace.clone(),
    ));

    let router_state = RouterState::new("lidp://app", "lidp://app", database, oauth2_service);

    let openapi_router = lidp_server::openapi_router(router_state, "");

    Ok(openapi_router.split_for_parts().0)
}

pub async fn init_datebase(
    app_handle: AppHandle,
    app_config: Arc<AppConfig>,
) -> io::Result<Arc<Database>> {
    let database = Arc::new(open_database(&app_config.database).await.map_err(|e| {
        log::error!("failed to create database pool: {}", e);
        io::Error::other(e)
    })?);

    lidp_model::migrate::up(&database).await.map_err(|e| {
        log::error!("failed to run database migrations: {}", e);
        io::Error::other(e)
    })?;

    let key_service = Arc::new(KeyService::new(
        LibSqlKeyRepo::new(database.clone()),
        PrivateKeyKeyringRepo::new(&app_config.oauth2.issuer),
        app_config.key_namespace.clone(),
    ));

    let bootstrap_service = BootstrapService::new(
        LibSqlApplicationRepo::new(database.clone()),
        LibSqlClientRepo::new(database.clone(), key_service.clone()),
        LibSqlUserRepo::new(
            database.clone(),
            key_service.clone(),
            app_config.password.clone(),
        ),
        LibSqlRoleRepo::new(database.clone()),
        LibSqlPermissionRepo::new(database.clone()),
        key_service.clone(),
        app_config.bootstrap.clone(),
    );

    bootstrap_service
        .ensure_system_baseline()
        .await
        .map_err(io::Error::other)?;

    app_handle.manage(database.clone());

    Ok(database)
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

        default_config.bootstrap.is_master = true;
        default_config.bootstrap.web = false;
        default_config.bootstrap.desktop = true;
        default_config.database.url = format!(
            "file://{}",
            data_dir.as_ref().join("lidp.db").to_string_lossy()
        );
        default_config.oauth2.issuer = "lidp://app".to_string();
        default_config.ui_public_uri = "lidp://app".to_string();
        default_config.api_public_uri = "lidp://app".to_string();

        let json_str = yaml_serde::to_string(&default_config)
            .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?;

        fs::write(&config_path, json_str)?;

        default_config
    };

    let app_config = Arc::new(app_config);

    app_handle.manage(app_config.clone());

    Ok(app_config)
}

pub async fn request_handler(app: AppHandle, request: Request) -> Response {
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

pub async fn close(app_handle: &AppHandle) -> io::Result<()> {
    if let Some(database) = app_handle.try_state::<Database>() {
        close_database(database.inner())
            .await
            .map_err(io::Error::other)?;
    }

    Ok(())
}
