use axum::Json;
use model::contract::{ErrorResponse, VersionResponse};

#[utoipa::path(get, path = "/version", responses((status = 200, description = "Version", body = VersionResponse)))]
pub(crate) async fn version() -> Result<Json<VersionResponse>, ErrorResponse> {
    Ok(Json(VersionResponse::default()))
}