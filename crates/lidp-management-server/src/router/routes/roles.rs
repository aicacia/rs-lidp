use axum::{
    Json,
    extract::{Path, Query, State},
};
use model::contract::{ErrorCode, ErrorResponse};
use serde::{Deserialize, Serialize};
use service::management::ManagementService;

use crate::router::{RouterState, middleware::ManagementAuthorization};

const ROLES_READ_PERMISSION: &str = "roles.read";
const ROLES_WRITE_PERMISSION: &str = "roles.write";
pub(crate) const MANAGEMENT_APPLICATION_ID: &str = "lidp-management";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub(crate) struct RoleResponse {
    pub id: i64,
    pub application_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<model::model::Role> for RoleResponse {
    fn from(role: model::model::Role) -> Self {
        Self {
            id: role.id,
            application_id: role.application_id,
            name: role.name,
            description: role.description,
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
    path = "/applications/{application_id}/roles",
    params(
        ("application_id" = String, Path, description = "Application ID"),
        ListRolesQuery
    ),
    responses((status = 200, description = "List roles", body = [RoleResponse])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_roles(
    State(state): State<RouterState>,
    Path(application_id): Path<String>,
    Query(query): Query<ListRolesQuery>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<RoleResponse>>, ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_READ_PERMISSION,
    )
    .await?;

    let roles = state
        .management_service
        .list_roles(&application_id, query.offset, normalize_limit(query.limit))
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(roles.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/applications/{application_id}/roles",
    params(
        ("application_id" = String, Path, description = "Application ID")
    ),
    request_body = CreateRoleRequest,
    responses((status = 201, description = "Create role", body = RoleResponse)),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn create_role(
    State(state): State<RouterState>,
    Path(application_id): Path<String>,
    authorization: ManagementAuthorization,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_WRITE_PERMISSION,
    )
    .await?;

    let role = state
        .management_service
        .create_role(&application_id, &body.name, body.description.as_deref())
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(role.into()))
}

#[utoipa::path(
    delete,
    path = "/applications/{application_id}/roles/{role_id}",
    params(
        ("application_id" = String, Path, description = "Application ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Delete role")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn delete_role(
    State(state): State<RouterState>,
    Path((application_id, role_id)): Path<(String, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_WRITE_PERMISSION,
    )
    .await?;

    if state
        .management_service
        .find_role_by_id(&application_id, role_id)
        .await
        .map_err(ErrorResponse::from)?
        .is_none()
    {
        return Err(ErrorResponse::new(ErrorCode::NotFound).with_description("Role not found"));
    }

    state
        .management_service
        .delete_role_by_id(&application_id, role_id)
        .await
        .map_err(ErrorResponse::from)
}

#[utoipa::path(
    get,
    path = "/applications/{application_id}/users/{user_id}/roles",
    params(
        ("application_id" = String, Path, description = "Application ID"),
        ("user_id" = i64, Path, description = "User ID")
    ),
    responses((status = 200, description = "List user roles", body = [RoleResponse])),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn list_user_roles(
    State(state): State<RouterState>,
    Path((application_id, user_id)): Path<(String, i64)>,
    authorization: ManagementAuthorization,
) -> Result<Json<Vec<RoleResponse>>, ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_READ_PERMISSION,
    )
    .await?;

    let roles = state
        .management_service
        .list_user_roles(&application_id, user_id)
        .await
        .map_err(ErrorResponse::from)?;

    Ok(Json(roles.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/applications/{application_id}/users/{user_id}/roles/{role_id}",
    params(
        ("application_id" = String, Path, description = "Application ID"),
        ("user_id" = i64, Path, description = "User ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Assign role to user")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn assign_role_to_user(
    State(state): State<RouterState>,
    Path((application_id, user_id, role_id)): Path<(String, i64, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_WRITE_PERMISSION,
    )
    .await?;

    if state
        .management_service
        .find_role_by_id(&application_id, role_id)
        .await
        .map_err(ErrorResponse::from)?
        .is_none()
    {
        return Err(ErrorResponse::new(ErrorCode::NotFound).with_description("Role not found"));
    }

    state
        .management_service
        .add_role_to_user(&application_id, user_id, role_id)
        .await
        .map_err(ErrorResponse::from)
}

#[utoipa::path(
    delete,
    path = "/applications/{application_id}/users/{user_id}/roles/{role_id}",
    params(
        ("application_id" = String, Path, description = "Application ID"),
        ("user_id" = i64, Path, description = "User ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Revoke role from user")),
    security(
        ("authorization" = [])
    )
)]
pub(crate) async fn revoke_role_from_user(
    State(state): State<RouterState>,
    Path((application_id, user_id, role_id)): Path<(String, i64, i64)>,
    authorization: ManagementAuthorization,
) -> Result<(), ErrorResponse> {
    require_application_permission(
        state.management_service.as_ref(),
        &authorization,
        &application_id,
        ROLES_WRITE_PERMISSION,
    )
    .await?;

    state
        .management_service
        .remove_role_from_user(&application_id, user_id, role_id)
        .await
        .map_err(ErrorResponse::from)
}

pub(crate) async fn require_application_permission(
    management_service: &ManagementService,
    authorization: &ManagementAuthorization,
    application_id: &str,
    permission: &str,
) -> Result<(), ErrorResponse> {
    let has_permission = management_service
        .has_user_application_permission(
            authorization.principal.get_entity_id(),
            application_id,
            permission,
        )
        .await
        .map_err(ErrorResponse::from)?;

    if has_permission {
        return Ok(());
    }

    Err(ErrorResponse::new(ErrorCode::AccessDenied)
        .with_description("missing required application permission"))
}

pub(crate) use require_application_permission as require_client_permission;

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
    use model::contract::{
        EntityType, ErrorCode, ErrorResponse, StandardClaims, TokenType, TokenUse,
    };
    use service::{
        management::ManagementService,
        PasswordConfig,
        oauth2::{OAuth2Config, OAuth2Service},
        repo::{
            KeyRepo, KeyService, LibSqlApplicationRepo, LibSqlClientRepo, LibSqlKeyRepo,
            LibSqlOAuth2AuthorizationCodeRepo, LibSqlOAuth2UserConsentRepo,
            LibSqlPermissionRepo, LibSqlRoleRepo, LibSqlUserRepo, PermissionRepo,
            PrivateKeyKeyringRepo, RoleRepo,
        },
    };
    use tower::util::ServiceExt;

    use super::{ROLES_WRITE_PERMISSION, RoleResponse};
    use crate::RouterState;

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

    async fn test_router_with_role_setup(
        role_setup: RoleSetup,
        application_id: &str,
    ) -> (Router, String) {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let sequence = NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let database_path = std::env::temp_dir().join(format!(
            "lidp-management-roles-route-tests-{process_id}-{unique_suffix}-{sequence}.sqlite"
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

        let role_repo = Arc::new(LibSqlRoleRepo::new(database.clone()));
        let permission_repo = Arc::new(LibSqlPermissionRepo::new(database.clone()));
        let key_repo = LibSqlKeyRepo::new(database.clone());

        let caller_user_id = insert_test_user(&database, "roles-route-caller").await;
        let caller_key = key_repo
            .create_key(
                None,
                EntityType::User,
                caller_user_id,
                true,
                "roles-route-caller".to_string(),
                None,
            )
            .await
            .expect("create key failed");

        if let RoleSetup::Admin = role_setup {
            let admin_role = role_repo
                .create_role(application_id, "admin", None)
                .await
                .expect("create admin role failed");
            grant_permission_to_role(
                permission_repo.as_ref(),
                application_id,
                admin_role.id,
                ROLES_WRITE_PERMISSION,
            )
            .await;
            role_repo
                .add_role_to_user(application_id, caller_user_id, admin_role.id)
                .await
                .expect("assign admin role failed");
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

        (router, token)
    }

    async fn post_create_role(
        router: Router,
        token: &str,
        application_id: &str,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method("POST")
            .uri(format!("/applications/{application_id}/roles"))
            .header("content-type", "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "name": "support" }))
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
    async fn create_role_route_denies_without_permission_assignments() {
        let application_id = "roles-test-app";
        let (router, token) =
            test_router_with_role_setup(RoleSetup::Bootstrap, application_id).await;

        let (status, body) = post_create_role(router, &token, application_id).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let error: ErrorResponse = serde_json::from_slice(&body).expect("decode error response");
        assert_eq!(error.error, ErrorCode::AccessDenied);
    }

    #[tokio::test]
    async fn create_role_route_allows_when_permission_assigned() {
        let application_id = "roles-test-app";
        let (router, token) = test_router_with_role_setup(RoleSetup::Admin, application_id).await;

        let (status, body) = post_create_role(router, &token, application_id).await;

        assert_eq!(status, StatusCode::OK);
        let response: RoleResponse = serde_json::from_slice(&body).expect("decode role response");
        assert_eq!(response.application_id, application_id);
        assert_eq!(response.name, "support");
    }
}
