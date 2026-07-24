use std::sync::Arc;

use libsql::Database;
use service::{
    oauth2::OAuth2Service,
    repo::{
        LibSqlClientRepo, LibSqlKeyRepo, LibSqlOAuth2AuthorizationCodeRepo, LibSqlUserRepo,
        MasterKeyKeyringRepo,
    },
};

#[derive(Clone)]
pub struct RouterState {
    pub ui_base_url: String,
    pub database: Arc<Database>,
    pub oauth2_service: Arc<
        OAuth2Service<
            LibSqlClientRepo,
            LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlUserRepo,
            MasterKeyKeyringRepo,
        >,
    >,
}

impl RouterState {
    pub fn new(
        ui_base_url: impl Into<String>,
        database: Arc<Database>,
        oauth2_service: Arc<
            OAuth2Service<
                LibSqlClientRepo,
                LibSqlKeyRepo,
                LibSqlOAuth2AuthorizationCodeRepo,
                LibSqlUserRepo,
                MasterKeyKeyringRepo,
            >,
        >,
    ) -> Self {
        Self {
            ui_base_url: ui_base_url.into(),
            database,
            oauth2_service,
        }
    }
}
