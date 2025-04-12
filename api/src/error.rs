use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);
pub type AppResult<T> = Result<T, AppError>;

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

// Implement conversion from specific error types
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self(err.into())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self(err.into())
    }
}

impl From<std::env::VarError> for AppError {
    fn from(err: std::env::VarError) -> Self {
        Self(err.into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self(err.into())
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Self(err.into())
    }
}
