// src/ocm/client.rs

use reqwest::Client;
use anyhow::Result;
use tracing::info;
use crate::ocm::types::OcmStation;

pub async fn fetch_stations(
    client: &Client,
    base_url: &str,
    api_key: &str,
    latitude: f64,
    longitude: f64,
    distance_km: u32,
    modified_since: Option<&str>,
) -> Result<Vec<OcmStation>> {
    let mut url = format!(
        "{}?output=json&latitude={}&longitude={}&distance={}&distanceunit=KM&maxresults=1000&compact=false&verbose=true&key={}",
        base_url, latitude, longitude, distance_km, api_key
    );


    if let Some(since) = modified_since {
        url.push_str(&format!("&modifiedsince={}", since));
    }

    let stations: Vec<OcmStation> = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(stations)
}