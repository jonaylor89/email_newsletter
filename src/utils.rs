use axum::http::StatusCode;
use axum::response::Redirect;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug)]
pub struct AppError {
    error: anyhow::Error,
    status: StatusCode,
}

impl AppError {
    pub fn new(error: anyhow::Error, status: StatusCode) -> Self {
        Self { error, status }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{:?}", self.error);
        let body = serde_json::json!({
            "error": self.error.to_string(),
        });
        (self.status, Json(body)).into_response()
    }
}

pub fn e500<T>(e: T) -> AppError
where
    T: Into<anyhow::Error>,
{
    AppError::new(e.into(), StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn e400<T>(e: T) -> AppError
where
    T: Into<anyhow::Error>,
{
    AppError::new(e.into(), StatusCode::BAD_REQUEST)
}

pub fn see_other(location: &str) -> Response {
    Redirect::to(location).into_response()
}
