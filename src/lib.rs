use std::sync::Arc;
use axum::{middleware, Router};
use tower_http::trace::TraceLayer;
use tower_http::compression::CompressionLayer;

pub mod auth;
pub mod config;
pub mod error;
pub mod flanders;
pub mod models;
pub mod ocm;
pub mod osm;
pub mod routes;
pub mod utils;

pub struct AppState {
    pub ocm_api_key: String,
    pub app_api_key: String,
    pub db: sqlx::SqlitePool,
    pub config: config::AppConfig,
    pub http_client: reqwest::Client,
}

// src/lib.rs
pub fn build_router(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .merge(routes::station_routes())
        .merge(routes::admin_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ));

    Router::new()
        .merge(routes::health_routes())
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state)
}