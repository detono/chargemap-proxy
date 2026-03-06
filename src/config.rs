use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub cache: CacheConfig,
    pub location: LocationConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocationConfig {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub radius_km: u32,
}

pub fn load() -> Result<AppConfig, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("APP").separator("_"))
        .build()?
        .try_deserialize()
}