use std::sync::Arc;
use chargemap_proxy::AppState;
use chargemap_proxy::config::{AppConfig, ServerConfig, CacheConfig, LocationConfig, OpenChargeMapConfig, OsmConfig};

pub async fn setup_test_state(ocm_url: &str, osm_url: &str) -> Arc<AppState> {
    let db = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&db).await.unwrap();

    Arc::new(AppState {
        ocm_api_key: "test-key".to_string(),
        app_api_key: "test-app-key".to_string(),
        db,
        http_client: reqwest::Client::new(),
        config: AppConfig {
            server: ServerConfig { port: 8083 },
            cache: CacheConfig { refresh_interval_seconds: 300 },
            location: LocationConfig {
                name: "Ghent".to_string(),
                latitude: 51.0543,
                longitude: 3.7174,
                radius_km: 30,
            },
            ocm: OpenChargeMapConfig { url: ocm_url.to_string() },
            osm: OsmConfig { url: osm_url.to_string() },
        },
    })
}