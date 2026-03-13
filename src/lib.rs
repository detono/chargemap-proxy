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