use axum::{
    Json,
    extract::{Query, State},
    response::Redirect,
};
use model::contract::{AuthorizationRequest, ErrorCode, ErrorResponse};

use crate::router::RouterState;

#[utoipa::path(post, path = "/oauth2/auth", responses((status = 302, description = "Authorization response")))]
#[axum::debug_handler]
pub(crate) async fn authorize_json(
    State(state): State<RouterState>,
    Json(query): Json<AuthorizationRequest>,
) -> Result<Redirect, ErrorResponse> {
    internal_authorize(state, query).await
}

#[utoipa::path(get, path = "/oauth2/auth", responses((status = 302, description = "Authorization response")))]
#[axum::debug_handler]
pub(crate) async fn authorize_query(
    State(state): State<RouterState>,
    Query(query): Query<AuthorizationRequest>,
) -> Result<Redirect, ErrorResponse> {
    internal_authorize(state, query).await
}

async fn internal_authorize(
    state: RouterState,
    request: AuthorizationRequest,
) -> Result<Redirect, ErrorResponse> {
    let ui_base_url = state.ui_base_url.clone();
    let query_string = serde_qs::to_string(&request)
        .map_err(|e| ErrorResponse::new(ErrorCode::ServerError).with_description(e.to_string()))?;

    Ok(Redirect::to(&format!("{}?{}", ui_base_url, query_string)))
}
