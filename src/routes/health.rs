use axum::{routing::get, Router, Json};
use http::StatusCode;
use crate::AppState;
use std::sync::Arc;

pub fn health_routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(e) => {
            tracing::error!("Health check failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, "Database connection failed")
        }
    }
}