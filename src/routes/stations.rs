use std::collections::HashMap;
use crate::{models::{ConnectorResponse, StationResponse}, AppState};
use axum::extract::Query;
use axum::{extract::{Path, State}, Json};
use axum::{routing::get, Router};
use serde::Deserialize;
use std::sync::Arc;
use crate::error::AppError;

pub fn station_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stations", get(list_stations))
        .route("/stations/:id", get(get_station))
}

#[derive(Debug, Deserialize)]
pub struct StationFilters {
    pub min_power_kw: Option<f64>,
    pub connector_type: Option<String>,
    pub fast_charge_only: Option<bool>,
    pub operational_only: Option<bool>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub radius_km: Option<f64>,
}

async fn list_stations(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<StationFilters>,
) -> Result<Json<Vec<StationResponse>>, AppError> {
    let connector_type_filter = filters.connector_type.as_deref().unwrap_or("");
    let operational_only = filters.operational_only.unwrap_or(true);

    // Calculate Bounding Box if radius is provided
    let mut min_lat: Option<f64> = None;
    let mut max_lat: Option<f64> = None;
    let mut min_lon: Option<f64> = None;
    let mut max_lon: Option<f64> = None;

    if let (Some(lat), Some(lon), Some(radius)) = (filters.lat, filters.lon, filters.radius_km) {
        // Roughly 1 degree of lat = 111km
        let lat_delta = radius / 111.0;
        // Lon delta depends on latitude
        let lon_delta = radius / (111.0 * lat.to_radians().cos().abs());

        min_lat = Some(lat - lat_delta);
        max_lat = Some(lat + lat_delta);
        min_lon = Some(lon - lon_delta);
        max_lon = Some(lon + lon_delta);
    }

    // If coordinates are provided, use bounding box approach
    let stations = sqlx::query!(
        r#"
        SELECT
            id, address_title, address_line1, town, postcode,
            latitude, longitude, operator_title, usage_cost,
            is_operational, number_of_points
        FROM stations
        WHERE (?1 = 0 OR is_operational = 1)
          AND (?2 IS NULL OR latitude >= ?2)
          AND (?3 IS NULL OR latitude <= ?3)
          AND (?4 IS NULL OR longitude >= ?4)
          AND (?5 IS NULL OR longitude <= ?5)
        ORDER BY id
        "#,
        operational_only,
        min_lat,
        max_lat,
        min_lon,
        max_lon
    )
    .fetch_all(&state.db)
    .await?;

    if stations.is_empty() {
        return Ok(Json(vec![]));
    }

    let station_ids: Vec<i64> = stations.iter().map(|s| s.id).collect();
    let ids_json = serde_json::to_string(&station_ids)?;

    let all_connectors = sqlx::query!(
        r#"
        SELECT
            station_id, connection_type, formal_name, power_kw,
            amps, voltage, current_type, is_fast_charge,
            is_operational, quantity
        FROM connections
        WHERE station_id IN (SELECT value FROM json_each(?1))
        "#,
        ids_json
    )
        .fetch_all(&state.db)
        .await?;

    // Group connectors by station_id in memory
    let mut connector_map: HashMap<i64, Vec<ConnectorResponse>> = HashMap::new();
    for c in all_connectors {
        let entry = connector_map.entry(c.station_id).or_default();

        // Apply connector-level filters here
        if let Some(min_kw) = filters.min_power_kw {
            if c.power_kw.unwrap_or(0.0) < min_kw { continue; }
        }

        if !connector_type_filter.is_empty() {
            let matches = c.connection_type.as_deref()
                .map(|t| t.to_lowercase().contains(&connector_type_filter.to_lowercase()))
                .unwrap_or(false);
            if !matches { continue; }
        }

        if filters.fast_charge_only == Some(true) && c.is_fast_charge != Some(1) {
            continue;
        }

        entry.push(ConnectorResponse {
            type_name: c.connection_type,
            formal_name: c.formal_name,
            power_kw: c.power_kw,
            amps: c.amps,
            voltage: c.voltage,
            current_type: c.current_type,
            is_fast_charge: c.is_fast_charge.map(|v| v != 0),
            is_operational: c.is_operational.map(|v| v != 0),
            quantity: c.quantity,
        });
    }

    // 3. Final Assembly
    let mut result = vec![];
    for row in stations {
        let connectors = connector_map.remove(&row.id).unwrap_or_default();
        if connectors.is_empty() { continue; }

        let distance_km = match (filters.lat, filters.lon) {
            (Some(lat), Some(lon)) => Some(haversine_km(lat, lon, row.latitude, row.longitude)),
            _ => None,
        };

        if let (Some(dist), Some(radius)) = (distance_km, filters.radius_km) {
            if dist > radius { continue; }
        }

        let address = format_address(
            &row.address_line1,
            &row.town,
            &row.postcode,
            &row.address_title
        );

        result.push(StationResponse {
            id: row.id,
            name: row.address_title,
            address,
            latitude: row.latitude,
            longitude: row.longitude,
            operator: row.operator_title,
            usage_cost: row.usage_cost,
            is_operational: row.is_operational.map(|v| v != 0),
            number_of_points: row.number_of_points,
            connectors,
            distance_km,
        });
    }

    if filters.lat.is_some() {
        result.sort_by(|a, b| a.distance_km.partial_cmp(&b.distance_km).unwrap_or(std::cmp::Ordering::Equal));
    }

    Ok(Json(result))
}

fn format_address(line1: &Option<String>, town: &Option<String>, post: &Option<String>, title: &Option<String>) -> Option<String> {
    match (line1, town, post) {
        (Some(a), Some(t), Some(p)) => Some(format!("{}, {}, {}", a, t, p)),
        (Some(a), Some(t), None)    => Some(format!("{}, {}", a, t)),
        (Some(a), None, _)          => Some(a.clone()),
        _                           => title.clone(),
    }
}

async fn get_station(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Option<StationResponse>>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            s.id as station_id, s.address_title, s.address_line1, s.town, s.postcode,
            s.latitude, s.longitude, s.operator_title, s.usage_cost,
            s.is_operational as station_operational, s.number_of_points,
            c.connection_type, c.formal_name, c.power_kw, c.amps, c.voltage,
            c.current_type, c.is_fast_charge, c.is_operational as conn_operational, c.quantity
        FROM stations s
        LEFT JOIN connections c ON s.id = c.station_id
        WHERE s.id = ?1
        "#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    if rows.is_empty() {
        return Ok(Json(None));
    }

    let first = &rows[0];

    let connectors: Vec<ConnectorResponse> = rows.iter()
        .filter_map(|r| {
            // filter_map handles the case where a station has 0 connectors (LEFT JOIN returns NULLs)
            r.connection_type.as_ref()?;

            Some(ConnectorResponse {
                type_name: r.connection_type.clone(),
                formal_name: r.formal_name.clone(),
                power_kw: r.power_kw,
                amps: r.amps,
                voltage: r.voltage,
                current_type: r.current_type.clone(),
                is_fast_charge: r.is_fast_charge.map(|v| v != 0),
                is_operational: r.conn_operational.map(|v| v != 0),
                quantity: r.quantity,
            })
        })
        .collect();

    let address = format_address(
        &first.address_line1,
        &first.town,
        &first.postcode,
        &first.address_title
    );

    Ok(Json(Some(StationResponse {
        id: first.station_id,
        name: first.address_title.clone(),
        address,
        latitude: first.latitude,
        longitude: first.longitude,
        operator: first.operator_title.clone(),
        usage_cost: first.usage_cost.clone(),
        is_operational: first.station_operational.map(|v| v != 0),
        number_of_points: first.number_of_points,
        connectors,
        distance_km: None,
    })))
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    r * 2.0 * a.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_same_point() {
        let dist = haversine_km(51.0543, 3.7174, 51.0543, 3.7174);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn test_haversine_ghent_to_brussels() {
        let dist = haversine_km(51.0543, 3.7174, 50.8503, 4.3517);
        assert!((dist - 50.0).abs() < 2.0, "Expected ~50km, got {}", dist);
    }

    #[test]
    fn test_format_address_full() {
        let result = format_address(
            &Some("Kiezelweg".to_string()),
            &Some("Nijlen".to_string()),
            &Some("2560".to_string()),
            &None,
        );
        assert_eq!(result, Some("Kiezelweg, Nijlen, 2560".to_string()));
    }

    #[test]
    fn test_format_address_no_postcode() {
        let result = format_address(
            &Some("Kiezelweg".to_string()),
            &Some("Nijlen".to_string()),
            &None,
            &None,
        );
        assert_eq!(result, Some("Kiezelweg, Nijlen".to_string()));
    }

    #[test]
    fn test_format_address_falls_back_to_title() {
        let result = format_address(&None, &None, &None, &Some("Some Title".to_string()));
        assert_eq!(result, Some("Some Title".to_string()));
    }

    #[test]
    fn test_format_address_all_none() {
        let result = format_address(&None, &None, &None, &None);
        assert_eq!(result, None);
    }
}