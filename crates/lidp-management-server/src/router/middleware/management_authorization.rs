use std::marker::PhantomData;

use axum::extract::{FromRef, FromRequestParts};
use http::{HeaderValue, header::AUTHORIZATION, request::Parts};
use model::contract::{ErrorCode, ErrorResponse, StandardClaims};
use service::oauth2::{Principal, decode_jwt};

use crate::RouterState;

pub const AUTHORIZATION_BEARER_PREFIX: &str = "Bearer ";

pub struct ManagementAuthorization {
    pub _principal: Box<dyn Principal>,
    pub principal_entity_id: i64,
    pub claims: StandardClaims,
    _phantom_data: PhantomData<StandardClaims>,
}

impl ManagementAuthorization {
    pub fn new(principal: Box<dyn Principal>, claims: StandardClaims) -> Self {
        let principal_entity_id = principal.get_entity_id();

        Self {
            _principal: principal,
            principal_entity_id,
            claims,
            _phantom_data: PhantomData,
        }
    }

    pub fn require_any_scope(&self, required_scopes: &[&str]) -> Result<(), ErrorResponse> {
        if self.claims.scope.iter().any(|scope| {
            required_scopes
                .iter()
                .any(|required_scope| scope == required_scope)
        }) {
            return Ok(());
        }

        Err(ErrorResponse::new(ErrorCode::AccessDenied)
            .with_description("missing required management scope"))
    }

    pub const fn principal_entity_id(&self) -> i64 {
        self.principal_entity_id
    }
}

impl<S> FromRequestParts<S> for ManagementAuthorization
where
    RouterState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(authorization_header_value) = parts.headers.get(AUTHORIZATION) {
            let authorization_string = authorization_from_header(authorization_header_value)?;
            let (jwt_header, claims) = decode_jwt::<StandardClaims>(authorization_string)?;

            let router_state = RouterState::from_ref(state);
            let principal = router_state
                .oauth2_service
                .find_principal(jwt_header.kid)
                .await?
                .ok_or_else(|| {
                    ErrorResponse::new(ErrorCode::NotAuthorized)
                        .with_description("principal not found for key id")
                })?;

            return Ok(Self::new(principal, claims));
        }

        Err(ErrorResponse::new(ErrorCode::NotAuthorized)
            .with_description("missing authorization header"))
    }
}

fn authorization_from_header(
    authorization_header_value: &HeaderValue,
) -> Result<&str, ErrorResponse> {
    log::debug!("parsing authorization header");
    match authorization_header_value.to_str() {
        Ok(authorization_string) => {
            if authorization_string.len() < AUTHORIZATION_BEARER_PREFIX.len() {
                log::warn!(
                    "invalid authorization header is too short: length={}",
                    authorization_string.len()
                );
                return Err(ErrorResponse::new(ErrorCode::NotAuthorized)
                    .with_description("authorization header is too short"));
            }
            if !authorization_string.starts_with(AUTHORIZATION_BEARER_PREFIX) {
                log::warn!(
                    "authorization header does not start with 'Bearer ', starts with: {}",
                    authorization_string.chars().take(10).collect::<String>()
                );
                return Err(ErrorResponse::new(ErrorCode::NotAuthorized)
                    .with_description("authorization header does not start with 'Bearer '"));
            }
            log::debug!("authorization header parsed successfully");
            Ok(&authorization_string[AUTHORIZATION_BEARER_PREFIX.len()..])
        }
        Err(e) => {
            log::warn!(
                "invalid authorization header cannot be parsed as string: {}",
                e
            );
            Err(ErrorResponse::new(ErrorCode::NotAuthorized)
                .with_description("invalid authorization header cannot be parsed as string"))
        }
    }
}
