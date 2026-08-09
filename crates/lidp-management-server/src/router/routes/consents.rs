use axum::{
    Json,
    extract::{Path, Query, State},
};
use model::contract::{ErrorCode, ErrorResponse};
use serde::{Deserialize, Serialize};

use crate::router::{RouterState, middleware::ManagementAuthorization};

use super::roles::{MANAGEMENT_APPLICATION_ID, require_client_permission};

const CONSENTS_READ_PERMISSION: &str = "consents.read";
const CONSENTS_WRITE_PERMISSION: &str = "consents.write";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct UserConsentResponse {
    pub id: i64,
    pub user_id: i64,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<model::model::OAuth2UserConsent> for UserConsentResponse {
    fn from(value: model::model::OAuth2UserConsent) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            client_id: value.client_id,
            redirect_uri: value.redirect_uri,
            scope: value.scope,
            created_at: value.created_at.timestamp(),
            updated_at: value.updated_at.timestamp(),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::IntoParams, utoipa::ToSchema,
)]
pub(crate) struct ListUserConsentsQuery {
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/consents",
    params(
        ("user_id" = i64, Path, description = "User ID"),
        ListUserConsentsQuery
    ),
    responses((status = 200, description = "List user consents", body = [UserConsentResponse])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_user_consents(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    Query(query): Query<ListUserConsentsQuery>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<UserConsentResponse>>, ErrorResponse> {
    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        MANAGEMENT_APPLICATION_ID,
        CONSENTS_READ_PERMISSION,
    )
    .await?;

    let consents = state
        .oauth2_service
        .list_user_consents(user_id, query.offset, normalize_limit(query.limit))
        .await?;

    Ok(Json(consents.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    delete,
    path = "/users/{user_id}/consents/{consent_id}",
    params(
        ("user_id" = i64, Path, description = "User ID"),
        ("consent_id" = i64, Path, description = "Consent ID")
    ),
    responses((status = 204, description = "Revoke user consent")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn revoke_user_consent(
    State(state): State<RouterState>,
    Path((user_id, consent_id)): Path<(i64, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    let consent = state
        .oauth2_service
        .list_user_consents(user_id, 0, 1_000)
        .await?
        .into_iter()
        .find(|item| item.id == consent_id)
        .ok_or_else(|| {
            ErrorResponse::new(ErrorCode::NotFound).with_description("User consent not found")
        })?;

    require_client_permission(
        state.role_repo.as_ref(),
        &authorization,
        &consent.client_id,
        CONSENTS_WRITE_PERMISSION,
    )
    .await?;

    state
        .oauth2_service
        .revoke_user_consent(user_id, consent_id)
        .await
}

const fn default_limit() -> u32 {
    50
}

fn normalize_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
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
    use libsql::params;
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
            LibSqlRoleRepo, LibSqlUserRepo, PrivateKeyKeyringRepo, RoleRepo,
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

    fn test_client_registration(application_id: i64) -> ClientRegistration {
        ClientRegistration {
            application_id,
            client_id: None,
            client_secret: None,
            client_id_issued_at: None,
            client_secret_expires_at: None,
            client_name: "consent-client".to_string(),
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
                params![username],
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
                "consent-route-application".to_string(),
                "https://example.test/applications/consent-route".to_string(),
                None,
            )
            .await
            .expect("create application failed")
            .id
    }

    async fn insert_test_consent(
        database: &libsql::Database,
        oauth2_service: &OAuth2Service<
            LibSqlClientRepo,
            LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo,
            LibSqlUserRepo,
            LibSqlOAuth2UserConsentRepo,
        >,
        application_id: i64,
        user_id: i64,
    ) -> (String, i64) {
        let client = oauth2_service
            .register_client(test_client_registration(application_id))
            .await
            .expect("register client failed");

        let client_id = client.client_id.expect("registered client id missing");
        let consent_client_id = client_id.clone();
        let connection = database.connect().expect("database connect failed");
        let mut rows = connection
            .query(
                "INSERT INTO oauth2_user_consents (user_id, client_id, redirect_uri, scope) VALUES (?, ?, ?, ?) RETURNING id",
                params![user_id, consent_client_id, "https://example.test/callback", "openid"],
            )
            .await
            .expect("insert consent failed");
        let row = rows
            .next()
            .await
            .expect("failed to read consent row")
            .expect("missing inserted consent row");

        (client_id, row.get(0).expect("missing inserted consent id"))
    }

    async fn test_router_with_role_setup(
        role_setup: RoleSetup,
    ) -> (Router, String, i64, i64, String) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-consents-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
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
        let key_repo = LibSqlKeyRepo::new(database.clone());

        let caller_user_id = insert_test_user(&database, "consent-route-caller").await;
        let target_user_id = insert_test_user(&database, "consent-route-target").await;
        let (consent_client_id, consent_id) = insert_test_consent(
            &database,
            oauth2_service.as_ref(),
            application_id,
            target_user_id,
        )
        .await;

        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "consent-route-caller".to_string(),
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
                role_repo
                    .upsert_role_permission(viewer_role.id, "users.write")
                    .await
                    .expect("grant viewer permission failed");
                role_repo
                    .assign_role_to_user_for_client(
                        MANAGEMENT_APPLICATION_ID,
                        caller_user_id,
                        viewer_role.id,
                    )
                    .await
                    .expect("assign viewer role failed");
            }
            RoleSetup::Admin => {
                let admin_role = role_repo
                    .create_role(MANAGEMENT_APPLICATION_ID, "admin", None)
                    .await
                    .expect("create admin role failed");
                role_repo
                    .upsert_role_permission(admin_role.id, "consents.write")
                    .await
                    .expect("grant admin permission failed");
                role_repo
                    .assign_role_to_user_for_client(
                        MANAGEMENT_APPLICATION_ID,
                        caller_user_id,
                        admin_role.id,
                    )
                    .await
                    .expect("assign admin role failed");
                role_repo
                    .assign_role_to_user_for_client(
                        &consent_client_id,
                        caller_user_id,
                        admin_role.id,
                    )
                    .await
                    .expect("assign admin client role failed");
            }
        }

        let token = bearer_token_for_key(caller_key.id, caller_user_id);

        let state = RouterState::new("", "", database, role_repo, oauth2_service);
        let router = crate::openapi_router(state, "/").into();

        (router, token, target_user_id, consent_id, consent_client_id)
    }

    async fn delete_user_consent(
        router: Router,
        token: &str,
        user_id: i64,
        consent_id: i64,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/users/{user_id}/consents/{consent_id}"))
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
    async fn revoke_user_consent_route_denies_without_permission_assignments() {
        let (router, token, target_user_id, consent_id, _) =
            test_router_with_role_setup(RoleSetup::Bootstrap).await;

        let (status, body) = delete_user_consent(router, &token, target_user_id, consent_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn revoke_user_consent_route_denies_when_wrong_permission_assigned() {
        let (router, token, target_user_id, consent_id, _) =
            test_router_with_role_setup(RoleSetup::NonAdmin).await;

        let (status, body) = delete_user_consent(router, &token, target_user_id, consent_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn revoke_user_consent_route_allows_when_permission_assigned() {
        let (router, token, target_user_id, consent_id, _) =
            test_router_with_role_setup(RoleSetup::Admin).await;

        let (status, _) = delete_user_consent(router, &token, target_user_id, consent_id).await;

        assert_eq!(status, StatusCode::OK);
    }
}
