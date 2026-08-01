use std::sync::Arc;

use libsql::Database;
use service::{
    oauth2::OAuth2Service,
    repo::{
        LibSqlClientRepo, LibSqlKeyRepo, LibSqlManagementRoleRepo,
        LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo, LibSqlUserRepo,
    },
};

#[derive(Clone)]
pub struct RouterState {
    pub ui_base_url: String,
    pub api_base_url: String,
    pub database: Arc<Database>,
    pub role_repo: Arc<LibSqlManagementRoleRepo>,
    pub oauth2_service: Arc<
        OAuth2Service<
            LibSqlClientRepo,
            LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlUserRepo,
            LibSqlOAuth2UserConsentRepo,
        >,
    >,
}

impl RouterState {
    pub fn new(
        ui_base_url: impl Into<String>,
        api_base_url: impl Into<String>,
        database: Arc<Database>,
        role_repo: Arc<LibSqlManagementRoleRepo>,
        oauth2_service: Arc<
            OAuth2Service<
                LibSqlClientRepo,
                LibSqlKeyRepo,
                LibSqlOAuth2AuthorizationCodeRepo,
                LibSqlUserRepo,
                LibSqlOAuth2UserConsentRepo,
            >,
        >,
    ) -> Self {
        Self {
            ui_base_url: ui_base_url.into(),
            api_base_url: api_base_url.into(),
            database,
            role_repo,
            oauth2_service,
        }
    }
}
