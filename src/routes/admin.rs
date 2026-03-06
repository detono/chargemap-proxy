use axum::{extract::State, http::StatusCode, routing::post, Router};
use std::sync::Arc;
use crate::AppState;

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/refresh", post(refresh_cache))
}

async fn refresh_cache(
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    tokio::spawn(async move {
        match crate::ocm::sync::sync_once(&state).await {
            Ok(n)  => tracing::info!("Manual refresh upserted {} stations", n),
            Err(e) => tracing::error!("Manual refresh failed: {e}"),
        }
    });
    StatusCode::ACCEPTED
}