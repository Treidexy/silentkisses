use std::str::FromStr;

use axum::{http::StatusCode, response::{IntoResponse, Response}};

pub type AppResult<T> = Result<T, AppError>;
pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}\n\n{}", self.0, self.0.backtrace()),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}