use axum::{
    Json,
    extract::{Path, State},
};
use model::{
    contract::{EntityType, ErrorResponse, JwkPublic},
    model::Key,
};
use serde::{Deserialize, Serialize};

use crate::router::{RouterState, middleware::ManagementAuthorization};

use super::roles::{MANAGEMENT_APPLICATION_ID, require_client_permission};

const KEYS_READ_PERMISSION: &str = "keys.read";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct ManagementKey {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub entity_type: EntityType,
    pub entity_id: i64,
    pub derivation_path: String,
    pub name: String,
    pub hardened: bool,
    pub revoked_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<Key> for ManagementKey {
    fn from(key: Key) -> Self {
        Self {
            id: key.id,
            parent_id: key.parent_id,
            entity_type: key.entity_type,
            entity_id: key.entity_id,
            derivation_path: key.derivation_path,
            name: key.name,
            hardened: key.hardened,
            revoked_at: key.revoked_at.map(|value| value.timestamp()),
            expires_at: key.expires_at.map(|value| value.timestamp()),
            created_at: key.created_at.timestamp(),
            updated_at: key.updated_at.timestamp(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/clients/{client_id}/keys",
    params(
        ("client_id" = String, Path, description = "Client ID")
    ),
    responses((status = 200, description = "List client keys", body = [ManagementKey])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_client_keys(
    State(state): State<RouterState>,
    Path(client_id): Path<String>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<ManagementKey>>, ErrorResponse> {
    require_client_permission(
        state.management_service.as_ref(),
        &authorization,
        &client_id,
        KEYS_READ_PERMISSION,
    )
    .await?;

    let keys = state.oauth2_service.list_client_keys(&client_id).await?;
    Ok(Json(keys.into_iter().map(ManagementKey::from).collect()))
}

#[utoipa::path(
    get,
    path = "/keys/{key_id}/jwk",
    params(
        ("key_id" = u32, Path, description = "Key ID")
    ),
    responses((status = 200, description = "Get public JWK for key", body = JwkPublic)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn get_key_jwk(
    State(state): State<RouterState>,
    Path(key_id): Path<u32>,
    authorization: ManagementAuthorization,
) -> Result<Json<JwkPublic>, ErrorResponse> {
    require_client_permission(
        state.management_service.as_ref(),
        &authorization,
        MANAGEMENT_APPLICATION_ID,
        KEYS_READ_PERMISSION,
    )
    .await?;

    let jwk = state.oauth2_service.find_public_jwk(key_id).await?;
    Ok(Json(jwk))
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
        management::ManagementService,
        PasswordConfig,
        oauth2::{OAuth2Config, OAuth2Service},
        repo::{
            ApplicationRepo, ClientRepo, KeyRepo, KeyService, LibSqlApplicationRepo,
            LibSqlClientRepo, LibSqlKeyRepo, LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlOAuth2UserConsentRepo, LibSqlPermissionRepo, LibSqlRoleRepo, LibSqlUserRepo,
            PermissionRepo, PrivateKeyKeyringRepo, RoleRepo,
        },
    };
    use tower::util::ServiceExt;

    use super::{JwkPublic, KEYS_READ_PERMISSION, ManagementKey};
    use crate::RouterState;
    use crate::router::routes::roles::MANAGEMENT_APPLICATION_ID;

    static NEXT_TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

    enum RoleSetup {
        Bootstrap,
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

    fn test_client_registration(application_id: i64) -> ClientRegistration {
        ClientRegistration {
            application_id,
            client_id: None,
            client_secret: None,
            client_id_issued_at: None,
            client_secret_expires_at: None,
            client_name: "keys-client".to_string(),
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

    async fn insert_test_application(database: &Arc<libsql::Database>) -> i64 {
        LibSqlApplicationRepo::new(database.clone())
            .create_application(
                "keys-route-application".to_string(),
                "https://example.test/applications/keys-route".to_string(),
                None,
            )
            .await
            .expect("create application failed")
            .id
    }

    async fn insert_test_client(
        oauth2_service: &OAuth2Service<
            LibSqlClientRepo,
            LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlUserRepo,
            LibSqlOAuth2UserConsentRepo,
        >,
        client_repo: &LibSqlClientRepo,
        key_repo: &LibSqlKeyRepo,
        application_id: i64,
    ) -> (String, u32) {
        let client = oauth2_service
            .register_client(test_client_registration(application_id))
            .await
            .expect("register client failed");
        let client_id = client.client_id.expect("registered client id missing");
        let client = client_repo
            .find_client_by_client_id(&client_id)
            .await
            .expect("find client failed")
            .expect("missing registered client");
        let key = key_repo
            .find_active_entity_root_key(EntityType::Client, client.id)
            .await
            .expect("find active client key failed")
            .expect("missing client key");

        (client_id, key.id)
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

    async fn test_router_with_role_setup(role_setup: RoleSetup) -> (Router, String, String, u32) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-keys-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
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
        let client_repo = LibSqlClientRepo::new(database.clone(), key_service.clone());

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

        let caller_user_id = insert_test_user(&database, "keys-route-caller").await;
        let (client_id, key_id) = insert_test_client(
            oauth2_service.as_ref(),
            &client_repo,
            &key_repo,
            application_id,
        )
        .await;
        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "keys-route-caller".to_string(),
                None,
            )
            .await
            .expect("create key failed");

        if let RoleSetup::Admin = role_setup {
            let admin_role = role_repo
                .create_role(MANAGEMENT_APPLICATION_ID, "admin", None)
                .await
                .expect("create admin role failed");
            grant_permission_to_role(
                permission_repo.as_ref(),
                MANAGEMENT_APPLICATION_ID,
                admin_role.id,
                KEYS_READ_PERMISSION,
            )
            .await;
            role_repo
                .add_role_to_user(MANAGEMENT_APPLICATION_ID, caller_user_id, admin_role.id)
                .await
                .expect("assign admin role failed");
            role_repo
                .add_role_to_user(&client_id, caller_user_id, admin_role.id)
                .await
                .expect("assign admin client role failed");
        }

        let token = bearer_token_for_key(caller_key.id, caller_user_id);

        let application_repo = Arc::new(LibSqlApplicationRepo::new(database.clone()));
        let permission_repo = Arc::new(LibSqlPermissionRepo::new(database.clone()));
        let management_service = Arc::new(ManagementService::new(
            application_repo,
            permission_repo,
            role_repo,
        ));
        let state = RouterState::new(
            "",
            database,
            management_service,
            oauth2_service,
        );
        let router = crate::openapi_router(state, "/").into();

        (router, token, client_id, key_id)
    }

    async fn get_client_keys(
        router: Router,
        token: &str,
        client_id: &str,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("GET")
            .uri(format!("/clients/{client_id}/keys"))
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

    async fn get_key(router: Router, token: &str, key_id: u32) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("GET")
            .uri(format!("/keys/{key_id}/jwk"))
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
    async fn list_client_keys_route_denies_without_permission_assignments() {
        let (router, token, client_id, _) = test_router_with_role_setup(RoleSetup::Bootstrap).await;

        let (status, body) = get_client_keys(router, &token, &client_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn list_client_keys_route_allows_when_permission_assigned() {
        let (router, token, client_id, _) = test_router_with_role_setup(RoleSetup::Admin).await;

        let (status, body) = get_client_keys(router, &token, &client_id).await;

        assert_eq!(status, StatusCode::OK);
        let keys: Vec<ManagementKey> = serde_json::from_slice(&body).expect("decode keys response");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].entity_type, EntityType::Client);
    }

    #[tokio::test]
    async fn get_key_jwk_route_denies_without_permission_assignments() {
        let (router, token, _, key_id) = test_router_with_role_setup(RoleSetup::Bootstrap).await;

        let (status, body) = get_key(router, &token, key_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn get_key_jwk_route_allows_when_permission_assigned() {
        let (router, token, _, key_id) = test_router_with_role_setup(RoleSetup::Admin).await;

        let (status, body) = get_key(router, &token, key_id).await;

        assert_eq!(status, StatusCode::OK);
        let jwk: JwkPublic = serde_json::from_slice(&body).expect("decode jwk response");
        assert_eq!(jwk.kid, key_id);
    }
}
