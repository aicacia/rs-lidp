use axum::{
    Json,
    extract::{Path, Query, State},
};
use model::contract::{ClientRegistration, ErrorResponse};
use serde::{Deserialize, Serialize};

use crate::router::{RouterState, middleware::ManagementAuthorization};

use super::roles::{MANAGEMENT_APPLICATION_ID, require_client_permission};

const CLIENTS_READ_PERMISSION: &str = "clients.read";
const CLIENTS_WRITE_PERMISSION: &str = "clients.write";

#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::IntoParams, utoipa::ToSchema,
)]
pub(crate) struct ListClientsQuery {
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[utoipa::path(
    get,
    path = "/clients",
    params(ListClientsQuery),
    responses((status = 200, description = "List clients", body = [ClientRegistration])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_clients(
    State(state): State<RouterState>,
    Query(query): Query<ListClientsQuery>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<ClientRegistration>>, ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        MANAGEMENT_APPLICATION_ID,
        CLIENTS_READ_PERMISSION,
    )
    .await?;

    let mut clients = state
        .oauth2_service
        .list_clients(query.offset, normalize_limit(query.limit))
        .await?;
    clients.iter_mut().for_each(sanitize_client_secret);

    Ok(Json(clients))
}

#[utoipa::path(
    post,
    path = "/clients",
    request_body = ClientRegistration,
    responses((status = 201, description = "Create client", body = ClientRegistration)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn create_client(
    State(state): State<RouterState>,
    authorization: ManagementAuthorization,
    Json(body): Json<ClientRegistration>,
) -> Result<Json<ClientRegistration>, ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        MANAGEMENT_APPLICATION_ID,
        CLIENTS_WRITE_PERMISSION,
    )
    .await?;

    let response = state.oauth2_service.register_client(body).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/clients/{client_id}",
    params(
        ("client_id" = String, Path, description = "Client ID")
    ),
    responses((status = 200, description = "Get client", body = ClientRegistration)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn get_client(
    State(state): State<RouterState>,
    Path(client_id): Path<String>,
    authorization: ManagementAuthorization,
) -> Result<Json<ClientRegistration>, ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        &client_id,
        CLIENTS_READ_PERMISSION,
    )
    .await?;

    let mut response = state.oauth2_service.get_client(&client_id).await?;
    sanitize_client_secret(&mut response);
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/clients/{client_id}",
    params(
        ("client_id" = String, Path, description = "Client ID")
    ),
    request_body = ClientRegistration,
    responses((status = 200, description = "Update client", body = ClientRegistration)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn update_client(
    State(state): State<RouterState>,
    Path(client_id): Path<String>,
    authorization: ManagementAuthorization,
    Json(body): Json<ClientRegistration>,
) -> Result<Json<ClientRegistration>, ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        &client_id,
        CLIENTS_WRITE_PERMISSION,
    )
    .await?;

    let mut response = state.oauth2_service.update_client(&client_id, body).await?;
    sanitize_client_secret(&mut response);
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/clients/{client_id}",
    params(
        ("client_id" = String, Path, description = "Client ID")
    ),
    responses((status = 204, description = "Delete client")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn delete_client(
    State(state): State<RouterState>,
    Path(client_id): Path<String>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        &client_id,
        CLIENTS_WRITE_PERMISSION,
    )
    .await?;
    state.oauth2_service.delete_client(&client_id).await
}

const fn default_limit() -> u32 {
    50
}

fn normalize_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

fn sanitize_client_secret(client: &mut ClientRegistration) {
    client.client_secret = None;
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::AUTHORIZATION},
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use db::{DatabaseConfig, open_database};
    use model::contract::{
        ClientProfile, ClientRegistration, EntityType, ErrorCode, ErrorResponse, GrantType,
        ResponseType, StandardClaims, TokenEndpointAuthMethod, TokenType, TokenUse,
    };
    use service::{
        PasswordConfig,
        oauth2::{OAuth2Config, OAuth2Service},
        repo::{
            ApplicationRepo, KeyRepo, KeyService, LibSqlApplicationRepo, LibSqlClientRepo,
            LibSqlKeyRepo, LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo,
            LibSqlPermissionRepo, LibSqlRoleRepo, LibSqlUserRepo, PermissionRepo,
            PrivateKeyKeyringRepo, RoleRepo,
        },
    };
    use tower::util::ServiceExt;

    use crate::RouterState;
    use crate::router::routes::roles::MANAGEMENT_APPLICATION_ID;

    static NEXT_TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

    enum RoleSetup {
        Bootstrap,
        NonAdmin,
        Admin,
    }

    fn encode_json_token_part<T: serde::Serialize>(value: &T) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).expect("token JSON serialization failed"))
    }

    fn bearer_token_for_key(kid: u32, sub: i64) -> String {
        let header = serde_json::json!({
            "alg": "ES256K",
            "typ": "JWT",
            "kid": kid,
        });

        let claims = StandardClaims {
            r#type: TokenType::Bearer,
            r#use: TokenUse::Access,
            exp: 4_102_444_800,
            iat: 1,
            nbf: 1,
            iss: "test-issuer".to_string(),
            aud: "test-audience".to_string(),
            sub: sub.to_string(),
            resource: None,
            scope: Vec::new(),
        };

        format!(
            "{}.{}.signature",
            encode_json_token_part(&header),
            encode_json_token_part(&claims)
        )
    }

    async fn insert_test_user(database: &libsql::Database, username: &str) -> i64 {
        let connection = database.connect().expect("database connect failed");
        let mut rows = connection
            .query(
                "INSERT INTO users (name) VALUES (?) RETURNING id",
                libsql::params![username],
            )
            .await
            .expect("insert user failed");
        let row = rows
            .next()
            .await
            .expect("failed to read user row")
            .expect("missing inserted user row");

        row.get(0).expect("missing inserted user id")
    }

    async fn insert_test_client(
        oauth2_service: &OAuth2Service<
            LibSqlClientRepo,
            LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlUserRepo,
            LibSqlOAuth2UserConsentRepo,
        >,
        application_id: i64,
    ) -> String {
        let client = oauth2_service
            .register_client(test_client_registration(application_id))
            .await
            .expect("register client failed");

        client.client_id.expect("registered client id missing")
    }

    async fn grant_permission_to_role(
        permission_repo: &LibSqlPermissionRepo,
        application_id: &str,
        role_id: i64,
        permission_name: &str,
    ) {
        let permission = permission_repo
            .create_permission(application_id, permission_name, None)
            .await
            .expect("create permission failed");
        permission_repo
            .add_permission_to_role(application_id, role_id, permission.id)
            .await
            .expect("assign permission to role failed");
    }

    async fn insert_test_application(database: &Arc<libsql::Database>) -> i64 {
        LibSqlApplicationRepo::new(database.clone())
            .create_application(
                "client-route-application".to_string(),
                "https://example.test/applications/client-route".to_string(),
                None,
            )
            .await
            .expect("create application failed")
            .id
    }

    async fn test_router_with_role_setup(role_setup: RoleSetup) -> (Router, String, i64, String) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-clients-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
        ));
        let database_url = format!("file://{}", database_path.display());

        let database = Arc::new(
            open_database(&DatabaseConfig {
                url: database_url,
                ..DatabaseConfig::default()
            })
            .await
            .expect("open database failed"),
        );

        model::migrate::up(&database)
            .await
            .expect("migrations failed");

        let application_id = insert_test_application(&database).await;

        let key_service = Arc::new(KeyService::new(
            LibSqlKeyRepo::new(database.clone()),
            PrivateKeyKeyringRepo::new("lidp-management-test"),
            "lidp-management-test",
        ));

        let oauth2_service = Arc::new(OAuth2Service::new(
            LibSqlClientRepo::new(database.clone(), key_service.clone()),
            LibSqlOAuth2AuthorizationCodeRepo::new(database.clone()),
            LibSqlUserRepo::new(
                database.clone(),
                key_service.clone(),
                PasswordConfig::default(),
            ),
            LibSqlOAuth2UserConsentRepo::new(database.clone()),
            key_service,
            OAuth2Config::default(),
            "lidp-management-test".to_string(),
        ));

        let role_repo = Arc::new(LibSqlRoleRepo::new(database.clone()));
        let permission_repo = Arc::new(LibSqlPermissionRepo::new(database.clone()));
        let key_repo = LibSqlKeyRepo::new(database.clone());

        let caller_user_id = insert_test_user(&database, "client-route-test-user").await;
        let target_client_id = insert_test_client(oauth2_service.as_ref(), application_id).await;
        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "client-route-test-user".to_string(),
                None,
            )
            .await
            .expect("create key failed");

        match role_setup {
            RoleSetup::Bootstrap => {}
            RoleSetup::NonAdmin => {
                let viewer_role = role_repo
                    .create_role(MANAGEMENT_APPLICATION_ID, "viewer", None)
                    .await
                    .expect("create viewer role failed");
                grant_permission_to_role(
                    permission_repo.as_ref(),
                    MANAGEMENT_APPLICATION_ID,
                    viewer_role.id,
                    "users.write",
                )
                .await;
                role_repo
                    .add_role_to_user(MANAGEMENT_APPLICATION_ID, caller_user_id, viewer_role.id)
                    .await
                    .expect("assign viewer role failed");
            }
            RoleSetup::Admin => {
                let admin_role = role_repo
                    .create_role(MANAGEMENT_APPLICATION_ID, "admin", None)
                    .await
                    .expect("create admin role failed");
                grant_permission_to_role(
                    permission_repo.as_ref(),
                    MANAGEMENT_APPLICATION_ID,
                    admin_role.id,
                    "clients.write",
                )
                .await;
                role_repo
                    .add_role_to_user(MANAGEMENT_APPLICATION_ID, caller_user_id, admin_role.id)
                    .await
                    .expect("assign admin role failed");
                role_repo
                    .add_role_to_user(&target_client_id, caller_user_id, admin_role.id)
                    .await
                    .expect("assign admin client role failed");
            }
        }

        let token = bearer_token_for_key(caller_key.id, caller_user_id);

        let state = RouterState::new("", "", database, role_repo, oauth2_service);
        let router = crate::openapi_router(state, "/").into();

        (router, token, application_id, target_client_id)
    }

    fn test_client_registration(application_id: i64) -> ClientRegistration {
        ClientRegistration {
            application_id,
            client_id: None,
            client_secret: None,
            client_id_issued_at: None,
            client_secret_expires_at: None,
            client_name: "client-under-test".to_string(),
            client_uri: Some("https://example.test".to_string()),
            logo_uri: None,
            contacts: Vec::new(),
            terms_of_service_uri: None,
            policy_uri: None,
            client_type: model::contract::ClientType::Public,
            profile: ClientProfile::Web,
            redirect_uris: vec!["https://example.test/callback".to_string()],
            allowed_grant_types: vec![GrantType::AuthorizationCode],
            response_types: vec![ResponseType::Code],
            allowed_scopes: vec!["openid".to_string()],
            token_endpoint_auth_method: TokenEndpointAuthMethod::None,
            software_statement: None,
            software_id: None,
            software_version: None,
        }
    }

    async fn post_create_client(
        router: Router,
        token: &str,
        application_id: i64,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("POST")
            .uri("/clients")
            .header("content-type", "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&test_client_registration(application_id))
                    .expect("serialize body failed"),
            ))
            .expect("build request failed");

        let response = router
            .oneshot(request)
            .await
            .expect("router request failed");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body failed")
            .to_vec();

        (status, body)
    }

    async fn delete_client_by_id(
        router: Router,
        token: &str,
        client_id: &str,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/clients/{client_id}"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("build request failed");

        let response = router
            .oneshot(request)
            .await
            .expect("router request failed");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body failed")
            .to_vec();

        (status, body)
    }

    #[tokio::test]
    async fn create_client_route_denies_without_permission_assignments() {
        let (router, token, application_id, _) =
            test_router_with_role_setup(RoleSetup::Bootstrap).await;

        let (status, body) = post_create_client(router, &token, application_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn create_client_route_denies_non_admin_when_assignments_exist() {
        let (router, token, application_id, _) =
            test_router_with_role_setup(RoleSetup::NonAdmin).await;

        let (status, body) = post_create_client(router, &token, application_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn create_client_route_allows_admin_when_assignments_exist() {
        let (router, token, application_id, _) =
            test_router_with_role_setup(RoleSetup::Admin).await;

        let (status, body) = post_create_client(router, &token, application_id).await;

        assert_eq!(status, StatusCode::OK);
        let client: ClientRegistration =
            serde_json::from_slice(&body).expect("decode client response");
        assert_eq!(client.client_name, "client-under-test");
    }

    #[tokio::test]
    async fn delete_client_route_denies_without_permission_assignments() {
        let (router, token, _, client_id) = test_router_with_role_setup(RoleSetup::Bootstrap).await;

        let (status, body) = delete_client_by_id(router, &token, &client_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn delete_client_route_denies_non_admin_when_assignments_exist() {
        let (router, token, _, client_id) = test_router_with_role_setup(RoleSetup::NonAdmin).await;

        let (status, body) = delete_client_by_id(router, &token, &client_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn delete_client_route_allows_admin_when_assignments_exist() {
        let (router, token, _, client_id) = test_router_with_role_setup(RoleSetup::Admin).await;

        let (status, _) = delete_client_by_id(router, &token, &client_id).await;

        assert_eq!(status, StatusCode::OK);
    }
}
