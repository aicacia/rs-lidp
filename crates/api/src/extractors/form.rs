use axum::extract::{FromRequest, Request, rejection::FormRejection};

use crate::error::{HttpError, REQUEST_BODY};

pub struct Form<T>(pub T);

impl<S, T> FromRequest<S> for Form<T>
where
  axum::Form<T>: FromRequest<S, Rejection = FormRejection>,
  S: Send + Sync,
{
  type Rejection = HttpError;

  async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
    let axum::Form(value) = match axum::Form::<T>::from_request(request, state).await {
      Ok(value) => value,
      Err(rejection) => {
        return Err(HttpError::bad_request().with_error(REQUEST_BODY, rejection.to_string()));
      }
    };
    Ok(Self(value))
  }
}
