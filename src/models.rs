use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StationResponse {
    pub id: i64,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub operator: Option<String>,
    pub usage_cost: Option<String>,
    pub is_operational: Option<bool>,
    pub number_of_points: Option<i64>,
    pub connectors: Vec<ConnectorResponse>,
    pub distance_km: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ConnectorResponse {
    pub type_name: Option<String>,
    pub formal_name: Option<String>,
    pub power_kw: Option<f64>,
    pub amps: Option<f64>,
    pub voltage: Option<f64>,
    pub current_type: Option<String>,
    pub is_fast_charge: Option<bool>,
    pub is_operational: Option<bool>,
    pub quantity: Option<i64>,
}