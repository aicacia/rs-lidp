use axum::{
    Json,
    extract::{Path, Query, State},
};
use model::contract::{ErrorCode, ErrorResponse};
use serde::{Deserialize, Serialize};
use service::repo::ManagementRoleRepo;

use crate::router::{RouterState, middleware::ManagementAuthorization};

const ROLES_READ_SCOPES: &[&str] = &["lidp:admin", "lidp:roles:read"];
const ROLES_WRITE_SCOPES: &[&str] = &["lidp:admin", "lidp:roles:write"];
const MANAGEMENT_ADMIN_ROLES: &[&str] = &["admin", "super_admin"];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct RoleResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<model::model::ManagementRole> for RoleResponse {
    fn from(role: model::model::ManagementRole) -> Self {
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            created_at: role.created_at.timestamp(),
            updated_at: role.updated_at.timestamp(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct UserRoleResponse {
    pub id: i64,
    pub user_id: i64,
    pub role_id: i64,
    pub role_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<model::model::ManagementUserRole> for UserRoleResponse {
    fn from(role: model::model::ManagementUserRole) -> Self {
        Self {
            id: role.id,
            user_id: role.user_id,
            role_id: role.role_id,
            role_name: role.role_name,
            created_at: role.created_at.timestamp(),
            updated_at: role.updated_at.timestamp(),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::IntoParams, utoipa::ToSchema,
)]
pub(crate) struct ListRolesQuery {
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[utoipa::path(
    get,
    path = "/roles",
    params(ListRolesQuery),
    responses((status = 200, description = "List roles", body = [RoleResponse])),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn list_roles(
    State(state): State<RouterState>,
    Query(query): Query<ListRolesQuery>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<RoleResponse>>, ErrorResponse> {
    authorization.require_any_scope(ROLES_READ_SCOPES)?;

    let roles = state
        .role_repo
        .list_roles(query.offset, normalize_limit(query.limit))
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(roles.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/roles",
    request_body = CreateRoleRequest,
    responses((status = 201, description = "Create role", body = RoleResponse)),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn create_role(
    State(state): State<RouterState>,
    authorization: ManagementAuthorization,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, ErrorResponse> {
    authorization.require_any_scope(ROLES_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;

    let role = state
        .role_repo
        .create_role(&body.name, body.description.as_deref())
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(role.into()))
}

#[utoipa::path(
    delete,
    path = "/roles/{role_id}",
    params(
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Delete role")),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn delete_role(
    State(state): State<RouterState>,
    Path(role_id): Path<i64>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    authorization.require_any_scope(ROLES_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;

    if state
        .role_repo
        .find_role_by_id(role_id)
        .await
        .map_err(ErrorResponse::from)?
        .is_none()
    {
        return Err(ErrorResponse::new(ErrorCode::NotFound).with_description("Role not found"));
    }

    state
        .role_repo
        .delete_role_by_id(role_id)
        .await
        .map_err(ErrorResponse::from)
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/roles",
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    responses((status = 200, description = "List user roles", body = [UserRoleResponse])),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn list_user_roles(
    State(state): State<RouterState>,
    Path(user_id): Path<i64>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<UserRoleResponse>>, ErrorResponse> {
    authorization.require_any_scope(ROLES_READ_SCOPES)?;

    let assignments = state
        .role_repo
        .list_user_roles(user_id)
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(assignments.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/roles/{role_id}",
    params(
        ("user_id" = i64, Path, description = "User ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Assign role to user")),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn assign_role_to_user(
    State(state): State<RouterState>,
    Path((user_id, role_id)): Path<(i64, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    authorization.require_any_scope(ROLES_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;

    if state
        .role_repo
        .find_role_by_id(role_id)
        .await
        .map_err(ErrorResponse::from)?
        .is_none()
    {
        return Err(ErrorResponse::new(ErrorCode::NotFound).with_description("Role not found"));
    }

    state
        .role_repo
        .assign_role_to_user(user_id, role_id)
        .await
        .map_err(ErrorResponse::from)
}

#[utoipa::path(
    delete,
    path = "/users/{user_id}/roles/{role_id}",
    params(
        ("user_id" = i64, Path, description = "User ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Revoke role from user")),
    security(
        ("Authorization" = [])
    )
)]
pub(crate) async fn revoke_role_from_user(
    State(state): State<RouterState>,
    Path((user_id, role_id)): Path<(i64, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    authorization.require_any_scope(ROLES_WRITE_SCOPES)?;
    require_rbac_admin(state.role_repo.as_ref(), &authorization).await?;

    state
        .role_repo
        .revoke_role_from_user(user_id, role_id)
        .await
        .map_err(ErrorResponse::from)
}

pub(crate) async fn require_rbac_admin(
    role_repo: &impl ManagementRoleRepo,
    authorization: &ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    let assignment_count = role_repo
        .count_user_role_assignments()
        .await
        .map_err(ErrorResponse::from)?;

    if assignment_count == 0 {
        return Ok(());
    }

    let caller_roles = role_repo
        .list_user_roles(authorization.principal_entity_id())
        .await
        .map_err(ErrorResponse::from)?;

    if caller_roles.iter().any(|assignment| {
        MANAGEMENT_ADMIN_ROLES
            .iter()
            .any(|required| assignment.role_name == *required)
    }) {
        return Ok(());
    }

    Err(ErrorResponse::new(ErrorCode::AccessDenied)
        .with_description("missing required management role"))
}

const fn default_limit() -> u32 {
    50
}

fn normalize_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::{
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::AUTHORIZATION},
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::Utc;
    use db::{DatabaseConfig, open_database};
    use libsql::params;
    use model::{
        contract::{EntityType, ErrorCode, ErrorResponse, StandardClaims, TokenType, TokenUse},
        model::{Key, ManagementRole, ManagementUserRole},
    };
    use service::{
        PasswordConfig,
        oauth2::{OAuth2Config, OAuth2Service, Principal},
        repo::{
            KeyRepo, KeyService, LibSqlClientRepo, LibSqlKeyRepo, LibSqlManagementRoleRepo,
            LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo, LibSqlUserRepo,
            ManagementRoleRepo, PrivateKeyKeyringRepo, RepoResult,
        },
    };
    use tower::util::ServiceExt;

    use crate::RouterState;

    use super::{CreateRoleRequest, ManagementAuthorization, RoleResponse, require_rbac_admin};

    static NEXT_TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

    struct TestPrincipal {
        key: Key,
        entity_id: i64,
    }

    impl Principal for TestPrincipal {
        fn get_entity_id(&self) -> i64 {
            self.entity_id
        }

        fn get_entity_type(&self) -> EntityType {
            EntityType::User
        }

        fn get_entity_as_any(&self) -> &dyn Any {
            self
        }

        fn get_key(&self) -> &Key {
            &self.key
        }
    }

    struct TestRoleRepo {
        assignment_count: u64,
        roles_for_caller: Vec<ManagementUserRole>,
        caller_user_id: i64,
    }

    enum RoleSetup {
        Bootstrap,
        NonAdmin,
        Admin,
    }

    impl ManagementRoleRepo for TestRoleRepo {
        async fn list_roles(&self, _offset: u32, _limit: u32) -> RepoResult<Vec<ManagementRole>> {
            Ok(Vec::new())
        }

        async fn create_role(
            &self,
            _name: &str,
            _description: Option<&str>,
        ) -> RepoResult<ManagementRole> {
            panic!("not used in tests")
        }

        async fn find_role_by_id(&self, _role_id: i64) -> RepoResult<Option<ManagementRole>> {
            Ok(None)
        }

        async fn delete_role_by_id(&self, _role_id: i64) -> RepoResult<()> {
            Ok(())
        }

        async fn list_user_roles(&self, user_id: i64) -> RepoResult<Vec<ManagementUserRole>> {
            if user_id == self.caller_user_id {
                Ok(self.roles_for_caller.clone())
            } else {
                Ok(Vec::new())
            }
        }

        async fn assign_role_to_user(&self, _user_id: i64, _role_id: i64) -> RepoResult<()> {
            Ok(())
        }

        async fn revoke_role_from_user(&self, _user_id: i64, _role_id: i64) -> RepoResult<()> {
            Ok(())
        }

        async fn count_user_role_assignments(&self) -> RepoResult<u64> {
            Ok(self.assignment_count)
        }
    }

    fn test_authorization(entity_id: i64, scopes: &[&str]) -> ManagementAuthorization {
        let claims = StandardClaims {
            r#type: TokenType::Bearer,
            r#use: TokenUse::Access,
            exp: 4_102_444_800,
            iat: 1,
            nbf: 1,
            iss: "test-issuer".to_string(),
            aud: "test-audience".to_string(),
            sub: entity_id.to_string(),
            resource: None,
            scope: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        };

        let principal = Box::new(TestPrincipal {
            key: Key {
                id: 1,
                parent_id: None,
                entity_type: EntityType::User,
                entity_id,
                derivation_path: "m/1'".to_string(),
                name: "test-key".to_string(),
                hardened: true,
                revoked_at: None,
                expires_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            entity_id,
        });

        ManagementAuthorization::new(principal, claims)
    }

    fn test_role(user_id: i64, role_name: &str) -> ManagementUserRole {
        ManagementUserRole {
            id: 1,
            user_id,
            role_id: 1,
            role_name: role_name.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
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
    ) -> (Router, String) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
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

        let caller_user_id = insert_test_user(&database, "route-test-user").await;
        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "route-test-user".to_string(),
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

        (router, token)
    }

    async fn post_create_role(router: Router, token: &str) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("POST")
            .uri("/roles")
            .header("content-type", "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&CreateRoleRequest {
                    name: "ops".to_string(),
                    description: Some("ops role".to_string()),
                })
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
    async fn bootstrap_allows_write_without_role_assignments() {
        let authorization = test_authorization(42, &["lidp:roles:write"]);
        let role_repo = TestRoleRepo {
            assignment_count: 0,
            roles_for_caller: Vec::new(),
            caller_user_id: 42,
        };

        let result = require_rbac_admin(&role_repo, &authorization).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn denies_write_when_assignments_exist_and_caller_is_not_admin() {
        let authorization = test_authorization(42, &["lidp:roles:write"]);
        let role_repo = TestRoleRepo {
            assignment_count: 1,
            roles_for_caller: vec![test_role(42, "viewer")],
            caller_user_id: 42,
        };

        let result = require_rbac_admin(&role_repo, &authorization).await;

        assert_eq!(
            result.expect_err("expected access denied error").error,
            ErrorCode::AccessDenied
        );
    }

    #[tokio::test]
    async fn allows_write_when_caller_has_admin_role() {
        let authorization = test_authorization(42, &["lidp:roles:write"]);
        let role_repo = TestRoleRepo {
            assignment_count: 2,
            roles_for_caller: vec![test_role(42, "admin")],
            caller_user_id: 42,
        };

        let result = require_rbac_admin(&role_repo, &authorization).await;

        assert!(result.is_ok());
    }

    #[test]
    fn denies_scope_when_required_scope_is_missing() {
        let authorization = test_authorization(42, &["lidp:users:read"]);
        let result = authorization.require_any_scope(&["lidp:roles:write"]);

        assert_eq!(
            result.expect_err("expected access denied error").error,
            ErrorCode::AccessDenied
        );
    }

    #[tokio::test]
    async fn create_role_route_denies_when_scope_is_missing() {
        let (router, token) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:roles:read"]).await;

        let (status, body) = post_create_role(router, &token).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn create_role_route_allows_bootstrap_without_role_assignments() {
        let (router, token) =
            test_router_with_role_setup(RoleSetup::Bootstrap, &["lidp:roles:write"]).await;

        let (status, body) = post_create_role(router, &token).await;

        assert_eq!(status, StatusCode::OK);
        let role: RoleResponse = serde_json::from_slice(&body).expect("decode role response");
        assert_eq!(role.name, "ops");
    }

    #[tokio::test]
    async fn create_role_route_denies_non_admin_when_assignments_exist() {
        let (router, token) =
            test_router_with_role_setup(RoleSetup::NonAdmin, &["lidp:roles:write"]).await;

        let (status, body) = post_create_role(router, &token).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn create_role_route_allows_admin_when_assignments_exist() {
        let (router, token) =
            test_router_with_role_setup(RoleSetup::Admin, &["lidp:roles:write"]).await;

        let (status, body) = post_create_role(router, &token).await;

        assert_eq!(status, StatusCode::OK);
        let role: RoleResponse = serde_json::from_slice(&body).expect("decode role response");
        assert_eq!(role.name, "ops");
    }
}
