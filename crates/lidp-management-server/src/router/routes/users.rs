use axum::{
    Json,
    extract::{Path, Query, State},
};
use model::contract::{ErrorResponse, UserInfo};
use serde::{Deserialize, Serialize};
use service::oauth2::UpdateUserInfoRequest;

use crate::router::{RouterState, middleware::ManagementAuthorization};

use super::roles::require_rbac_admin;

const USERS_READ_SCOPES: &[&str] = &["lidp:admin", "lidp:users:read"];
const USERS_WRITE_SCOPES: &[&str] = &["lidp:admin", "lidp:users:write"];

#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::IntoParams, utoipa::ToSchema,
)]
pub(crate) struct ListUsersQuery {
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[utoipa::path(
    get,
    path = "/users",
    params(ListUsersQuery),
    responses((status = 200, description = "List users", body = [UserInfo])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_users(
    State(state): State<RouterState>,
    Query(query): Query<ListUsersQuery>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<UserInfo>>, ErrorResponse> {
    authorization.require_any_scope(USERS_READ_SCOPES)?;

    let users = state
        .oauth2_service
        .list_user_info(query.offset, normalize_limit(query.limit))
        .await?;

    Ok(Json(users))
}

#[utoipa::path(
    get,
    path = "/users/{user_id}",
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    responses((status = 200, description = "Get user", body = UserInfo)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn get_user(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    authorization: ManagementAuthorization,
) -> Result<Json<UserInfo>, ErrorResponse> {
    authorization.require_any_scope(USERS_READ_SCOPES)?;

    let user = state.oauth2_service.find_user_info(user_id).await?;
    Ok(Json(user))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct UpdateUserRequest {
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub profile: Option<String>,
    pub picture: Option<String>,
    pub website: Option<String>,
    pub sex: Option<String>,
    pub birthdate: Option<String>,
    pub zoneinfo: Option<String>,
    pub locale: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone_number: Option<String>,
    pub phone_number_verified: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct ResetUserPasswordRequest {
    pub password: String,
}

#[utoipa::path(
    patch,
    path = "/users/{user_id}",
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses((status = 200, description = "Update user", body = UserInfo)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn update_user(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    authorization: ManagementAuthorization,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserInfo>, ErrorResponse> {
    authorization.require_any_scope(USERS_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;

    let user = state
        .oauth2_service
        .update_user_info(
            user_id,
            UpdateUserInfoRequest {
                name: body.name,
                given_name: body.given_name,
                family_name: body.family_name,
                middle_name: body.middle_name,
                nickname: body.nickname,
                profile: body.profile,
                picture: body.picture,
                website: body.website,
                sex: body.sex,
                birthdate: body.birthdate,
                zoneinfo: body.zoneinfo,
                locale: body.locale,
                email: body.email,
                email_verified: body.email_verified,
                phone_number: body.phone_number,
                phone_number_verified: body.phone_number_verified,
            },
        )
        .await?;

    Ok(Json(user))
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/password",
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    request_body = ResetUserPasswordRequest,
    responses((status = 204, description = "Reset user password")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn reset_user_password(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    authorization: ManagementAuthorization,
    Json(body): Json<ResetUserPasswordRequest>,
) -> Result<(), ErrorResponse> {
    authorization.require_any_scope(USERS_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;
    state
        .oauth2_service
        .reset_user_password(user_id, &body.password)
        .await
}

#[utoipa::path(
    delete,
    path = "/users/{user_id}",
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    responses((status = 204, description = "Delete user")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn delete_user(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    authorization.require_any_scope(USERS_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;
    state.oauth2_service.delete_user(user_id).await
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
        EntityType, ErrorCode, ErrorResponse, StandardClaims, TokenType, TokenUse,
    };
    use service::{
        PasswordConfig,
        oauth2::{OAuth2Config, OAuth2Service},
        repo::{
            KeyRepo, KeyService, LibSqlClientRepo, LibSqlKeyRepo, LibSqlManagementRoleRepo,
            LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo, LibSqlUserRepo,
            ManagementRoleRepo, PrivateKeyKeyringRepo,
        },
    };
    use tower::util::ServiceExt;

    use crate::RouterState;

    static NEXT_TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

    enum RoleSetup {
        Bootstrap,
        NonAdmin,
        Admin,
    }

    fn encode_json_token_part<T: serde::Serialize>(value: &T) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).expect("token JSON serialization failed"))
    }

    fn bearer_token_for_key(kid: u32, sub: i64, scopes: &[&str]) -> String {
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
            scope: scopes.iter().map(|scope| (*scope).to_string()).collect(),
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

    async fn test_router_with_role_setup(
        role_setup: RoleSetup,
        scopes: &[&str],
    ) -> (Router, String, i64) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-users-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
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

        let role_repo = Arc::new(LibSqlManagementRoleRepo::new(database.clone()));
        let key_repo = LibSqlKeyRepo::new(database.clone());

        let caller_user_id = insert_test_user(&database, "user-route-caller").await;
        let target_user_id = insert_test_user(&database, "user-route-target").await;

        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "user-route-caller".to_string(),
                None,
            )
            .await
            .expect("create key failed");

        match role_setup {
            RoleSetup::Bootstrap => {}
            RoleSetup::NonAdmin => {
                let viewer_role = role_repo
                    .create_role("viewer", None)
                    .await
                    .expect("create viewer role failed");
                role_repo
                    .assign_role_to_user(caller_user_id, viewer_role.id)
                    .await
                    .expect("assign viewer role failed");
            }
            RoleSetup::Admin => {
                let admin_role = role_repo
                    .create_role("admin", None)
                    .await
                    .expect("create admin role failed");
                role_repo
                    .assign_role_to_user(caller_user_id, admin_role.id)
                    .await
                    .expect("assign admin role failed");
            }
        }

        let token = bearer_token_for_key(caller_key.id, caller_user_id, scopes);

        let state = RouterState::new("", "", database, role_repo, oauth2_service);
        let router = crate::openapi_router(state, "/").into();

        (router, token, target_user_id)
    }

    async fn patch_update_user(router: Router, token: &str, user_id: i64) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("PATCH")
            .uri(format!("/users/{user_id}"))
            .header("content-type", "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "updated-user-name"
                }))
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

    async fn post_reset_user_password(
        router: Router,
        token: &str,
        user_id: i64,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("POST")
            .uri(format!("/users/{user_id}/password"))
            .header("content-type", "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "password": "new-password-123"
                }))
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

    #[tokio::test]
    async fn update_user_route_denies_when_scope_is_missing() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:users:read"]).await;

        let (status, body) = patch_update_user(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn update_user_route_allows_bootstrap_without_role_assignments() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:users:write"]).await;

        let (status, body) = patch_update_user(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::OK);
        let response: serde_json::Value =
            serde_json::from_slice(&body).expect("decode user response");
        assert_eq!(response["name"], "updated-user-name");
    }

    #[tokio::test]
    async fn update_user_route_denies_non_admin_when_assignments_exist() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::NonAdmin, &["lidp:users:write"]).await;

        let (status, body) = patch_update_user(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn update_user_route_allows_admin_when_assignments_exist() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Admin, &["lidp:users:write"]).await;

        let (status, body) = patch_update_user(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::OK);
        let response: serde_json::Value =
            serde_json::from_slice(&body).expect("decode user response");
        assert_eq!(response["name"], "updated-user-name");
    }

    #[tokio::test]
    async fn reset_user_password_route_denies_when_scope_is_missing() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:users:read"]).await;

        let (status, body) = post_reset_user_password(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn reset_user_password_route_allows_bootstrap_without_role_assignments() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:users:write"]).await;

        let (status, _) = post_reset_user_password(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn reset_user_password_route_denies_non_admin_when_assignments_exist() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::NonAdmin, &["lidp:users:write"]).await;

        let (status, body) = post_reset_user_password(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn reset_user_password_route_allows_admin_when_assignments_exist() {
        let (router, token, target_user_id) =
            test_router_with_role_setup(RoleSetup::Admin, &["lidp:users:write"]).await;

        let (status, _) = post_reset_user_password(router, &token, target_user_id).await;

        assert_eq!(status, StatusCode::OK);
    }
}
