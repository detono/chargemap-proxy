use axum::{Router, middleware};
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod cache;
mod config;
mod models;
mod ocm;
mod routes;

pub struct AppState {
    pub ocm_api_key: String,
    pub db: sqlx::SqlitePool,
    pub config: config::AppConfig,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // Load .env and config.toml
    dotenv().ok();
    let cfg = config::load();

    // Init logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Secrets from .env
    let ocm_api_key = env::var("OCM_API_KEY").expect("OCM_API_KEY must be set");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // DB pool
    let db = sqlx::SqlitePool::connect(&database_url)
        .await
        .expect("Failed to connect to SQLite");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    let state = Arc::new(AppState {
        ocm_api_key,
        db,
        config: cfg.clone(),
        http_client: reqwest::Client::new(),
    });

    // Build router
    let app = Router::new()
        .merge(routes::station_routes())
        .merge(routes::admin_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on http://{}", addr);
    tokio::spawn(ocm::sync::start_sync_loop(state.clone()));

    axum::serve(listener, app.with_state(state.clone())).await.unwrap();
}