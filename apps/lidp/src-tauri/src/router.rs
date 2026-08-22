use std::{io, sync::Arc};

use axum::Router;
use db::open_database;
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

pub async fn init(app_config: Arc<AppConfig>) -> io::Result<Router> {
    let database = open_database(&app_config.database).await.map_err(|e| {
        log::error!("failed to create database pool: {}", e);
        io::Error::other(e)
    })?;

    lidp_model::migrate::up(&database).await.map_err(|e| {
        log::error!("failed to run database migrations: {}", e);
        io::Error::other(e)
    })?;

    let database = Arc::new(database);
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
        key_service.clone(),
        oauth2_config,
        app_config.key_namespace.clone(),
    ));

    let router_state = RouterState::new("lidp://app", "lidp://app", database, oauth2_service);

    let openapi_router = lidp_server::openapi_router(router_state, "");

    Ok(openapi_router.split_for_parts().0)
}
