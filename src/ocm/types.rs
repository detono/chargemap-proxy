// src/ocm_types.rs

use serde::Deserialize;

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmStation {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "UUID")]
    pub uuid: String,
    pub operator_info: Option<OcmOperator>,
    pub usage_type: Option<OcmUsageType>,
    pub usage_cost: Option<String>,
    pub status_type: Option<OcmStatusType>,
    pub address_info: OcmAddress,
    pub connections: Option<Vec<OcmConnection>>,
    pub number_of_points: Option<i64>,
    pub general_comments: Option<String>,
    pub is_recently_verified: Option<bool>,
    pub date_last_verified: Option<String>,
    pub date_last_status_update: Option<String>,
    pub date_created: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmOperator {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmUsageType {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmStatusType {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
    pub is_operational: Option<bool>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmAddress {
    pub title: Option<String>,
    pub address_line1: Option<String>,
    pub town: Option<String>,
    pub state_or_province: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<OcmCountry>,
    pub latitude: f64,
    pub longitude: f64,
    pub access_comments: Option<String>,
    pub related_url: Option<String>,
    pub contact_telephone1: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmCountry {
    pub iso_code: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmConnection {
    #[serde(rename = "ID")]
    pub id: i64,
    pub connection_type: Option<OcmConnectionType>,
    pub level: Option<OcmLevel>,
    pub current_type: Option<OcmCurrentType>,
    pub status_type: Option<OcmStatusType>,
    pub amps: Option<f64>,
    pub voltage: Option<f64>,
    #[serde(rename = "PowerKW")]
    pub power_kw: Option<f64>,
    pub quantity: Option<i64>,
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmConnectionType {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
    pub formal_name: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmLevel {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
    pub is_fast_charge_capable: Option<bool>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OcmCurrentType {
    #[serde(rename = "ID")]
    pub id: Option<i64>,
    pub title: Option<String>,
}