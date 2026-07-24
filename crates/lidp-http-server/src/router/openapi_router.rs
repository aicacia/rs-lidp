use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{RefOr, Schema};
use utoipa::{Modify, OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::RouterState;

use super::openapi::{__path_openapi_json, openapi_json};
use super::routes::health::{__path_health, health};
use super::routes::oauth2::auth::{
    __path_authorize_json, __path_authorize_query, authorize_json, authorize_query,
};
use super::routes::oauth2::device::{
    __path_device_auth, __path_device_verify, device_auth, device_verify,
};
use super::routes::oauth2::register::{
    __path_delete_register, __path_get_register, __path_put_register, __path_register,
    delete_register, get_register, put_register, register,
};
use super::routes::oauth2::revoke::{__path_revoke, revoke};
use super::routes::oauth2::sessions::{__path_sessions_logout, sessions_logout};
use super::routes::oauth2::token::{__path_token, token};
use super::routes::userinfo::{__path_userinfo, userinfo};
use super::routes::version::{__path_version, version};
use super::routes::well_known::{
    __path_jwks, __path_openid_configuration, jwks, openid_configuration,
};

pub const AUTHORIZATION_HEADER: &str = "Authorization";

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                AUTHORIZATION_HEADER,
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        } else {
            log::warn!("OpenAPI components is None, cannot add security scheme");
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "OAuth Server", version = env!("CARGO_PKG_VERSION")),
    modifiers(&SecurityAddon)
)]
pub(crate) struct ApiDoc;

pub fn openapi_router(router_state: RouterState, prefix: &str) -> OpenApiRouter {
    let prefix = if prefix == "/" { "" } else { prefix };
    let openapi_router = {
        let openapi_router = OpenApiRouter::with_openapi(ApiDoc::openapi());

        let openapi_routes = OpenApiRouter::new()
            .routes(routes!(health))
            .routes(routes!(authorize_json))
            .routes(routes!(authorize_query))
            .routes(routes!(device_auth))
            .routes(routes!(device_verify))
            .routes(routes!(register))
            .routes(routes!(get_register))
            .routes(routes!(delete_register))
            .routes(routes!(put_register))
            .routes(routes!(token))
            .routes(routes!(revoke))
            .routes(routes!(sessions_logout))
            .routes(routes!(version))
            .routes(routes!(jwks))
            .routes(routes!(openid_configuration))
            .routes(routes!(userinfo))
            .with_state(router_state);

        if prefix.is_empty() {
            openapi_router.merge(openapi_routes)
        } else {
            openapi_router.nest(prefix, openapi_routes)
        }
    };

    let openapi_json_routes = {
        let mut openapi_spec = openapi_router.get_openapi().clone();

        let mut schemas = Vec::<(String, RefOr<Schema>)>::new();
        let (openapi_json_path, openapi_json_item, openapi_json_types) =
            routes!(@resolve_types openapi_json : schemas);

        let openapi_path = format!("{prefix}{openapi_json_path}");

        openapi_spec
            .paths
            .add_path_operation(openapi_path, openapi_json_types, openapi_json_item);

        OpenApiRouter::new()
            .routes(routes!(openapi_json))
            .with_state(openapi_spec)
    };

    if prefix.is_empty() {
        openapi_router.merge(openapi_json_routes)
    } else {
        openapi_router.nest(prefix, openapi_json_routes)
    }
}
