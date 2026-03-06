use axum::response::{IntoResponse, Response};
use http::StatusCode;

pub struct AppError(anyhow::Error);

// This enables using `?` on any error that can be converted into anyhow::Error
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the actual error for internal debugging
        tracing::error!("Application error: {:#}", self.0);

        // Return a generic 500 to the client to avoid leaking internal details
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong with the request",
        )
            .into_response()
    }
}