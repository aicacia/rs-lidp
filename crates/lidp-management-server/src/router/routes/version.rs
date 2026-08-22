use axum::Json;
use lidp_model::contract::{ErrorResponse, VersionResponse};

#[utoipa::path(get, path = "/version", responses((status = 200, description = "Version", body = VersionResponse)))]
pub(crate) async fn version() -> Result<Json<VersionResponse>, ErrorResponse> {
    Ok(Json(VersionResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        ..VersionResponse::default()
    }))
}
