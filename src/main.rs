use axum::{Router, middleware};
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use anyhow::Context;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod config;
mod models;
mod ocm;
mod routes;
mod error;

pub struct AppState {
    pub ocm_api_key: String,
    pub app_api_key: String,
    pub db: sqlx::SqlitePool,
    pub config: config::AppConfig,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env and config.toml
    dotenv().ok();

    // Init logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = config::load().map_err(|e| {
        tracing::error!("Configuration error: {}", e);
        e
    })?;

    // Secrets from .env
    let ocm_api_key = env::var("OCM_API_KEY").context("OCM_API_KEY must be set in .env")?;
    let app_api_key = env::var("APP_API_KEY").context("APP_API_KEY must be set .env")?;
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set .env")?;

    // DB pool
    let db = sqlx::SqlitePool::connect(&database_url)
        .await
        .context("Failed to connect to SQLite")?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("Failed to run migrations")?;

    let state = Arc::new(AppState {
        ocm_api_key,
        app_api_key,
        db,
        config: cfg.clone(),
        http_client: reqwest::Client::new(),
    });

    // Build router
    let protected = Router::new()
        .merge(routes::station_routes())
        .merge(routes::admin_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ));

    let app = Router::new()
        .merge(routes::health_routes())  
        .merge(protected)
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on http://{}", addr);
    tokio::spawn(ocm::sync::start_sync_loop(state.clone()));

    axum::serve(listener, app.with_state(state.clone())).await?;

    Ok(())
}