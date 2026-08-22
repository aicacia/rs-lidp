use std::{io, sync::Arc};

use axum::Router;
use db::open_database;
use storage_server::{AppConfig, RouterState};

pub async fn init(app_config: Arc<AppConfig>) -> io::Result<Router> {
    let _database = open_database(&app_config.database).await.map_err(|e| {
        log::error!("failed to create database pool: {}", e);
        io::Error::other(e)
    })?;

    let router_state = RouterState::new("storage://app");

    let openapi_router = storage_server::openapi_router(router_state, "");

    Ok(openapi_router.split_for_parts().0)
}
