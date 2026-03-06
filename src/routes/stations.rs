use crate::{models::{ConnectorResponse, StationResponse}, AppState};
use axum::extract::Query;
use axum::{extract::{Path, State}, Json};
use axum::{routing::get, Router};
use serde::Deserialize;
use std::sync::Arc;

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
) -> Json<Vec<StationResponse>> {
    let operational_only = filters.operational_only.unwrap_or(true);

    let stations = sqlx::query!(
        r#"
        SELECT
            id, address_title, address_line1, town, postcode,
            latitude, longitude, operator_title, usage_cost,
            is_operational, number_of_points
        FROM stations
        WHERE (?1 = 0 OR is_operational = 1)
        ORDER BY id
        "#,
        operational_only,
    )
    .fetch_all(&state.db)
    .await
    .expect("Failed to fetch stations");


    let mut result = vec![];

    for row in stations {
        let connectors = fetch_connectors(&state, row.id).await;

        // apply connector-level filters
        let connectors: Vec<_> = connectors.into_iter().filter(|c| {
            if let Some(min_kw) = filters.min_power_kw {
                if c.power_kw.unwrap_or(0.0) < min_kw { return false; }
            }
            if let Some(ref ct) = filters.connector_type {
                let matches = c.type_name.as_deref()
                    .map(|t| t.to_lowercase().contains(&ct.to_lowercase()))
                    .unwrap_or(false);
                if !matches { return false; }
            }
            if filters.fast_charge_only == Some(true) {
                if c.is_fast_charge != Some(true) { return false; }
            }
            true
        }).collect();

        // skip stations with no matching connectors
        if connectors.is_empty() { continue; }

        let address = match (&row.address_line1, &row.town, &row.postcode) {
            (Some(a), Some(t), Some(p)) => Some(format!("{}, {}, {}", a, t, p)),
            (Some(a), Some(t), None)    => Some(format!("{}, {}", a, t)),
            (Some(a), None, _)          => Some(a.clone()),
            _                           => row.address_title.clone(),
        };

        let distance_km = match (filters.lat, filters.lon) {
            (Some(lat), Some(lon)) => Some(haversine_km(lat, lon, row.latitude, row.longitude)),
            _ => None,
        };

        if let (Some(dist), Some(radius)) = (distance_km, filters.radius_km) {
            if dist > radius { continue; }
        }



        result.push(StationResponse {
            id: row.id,
            name: row.address_title.clone(),
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
        result.sort_by(|a, b| {
            a.distance_km.partial_cmp(&b.distance_km).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    Json(result)
}

async fn get_station(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Option<StationResponse>> {
    let row = sqlx::query!(
        r#"
        SELECT
            id, address_title, address_line1, town, postcode,
            latitude, longitude, operator_title, usage_cost,
            is_operational, number_of_points
        FROM stations
        WHERE id = ?1
        "#,
        id
    )
        .fetch_optional(&state.db)
        .await
        .expect("Failed to fetch station");

    let Some(row) = row else {
        return Json(None);
    };

    let address = match (&row.address_line1, &row.town, &row.postcode) {
        (Some(a), Some(t), Some(p)) => Some(format!("{}, {}, {}", a, t, p)),
        (Some(a), Some(t), None)    => Some(format!("{}, {}", a, t)),
        (Some(a), None, _)          => Some(a.clone()),
        _                           => row.address_title.clone(),
    };

    let connectors = fetch_connectors(&state, row.id).await;

    Json(Some(StationResponse {
        id: row.id,
        name: row.address_title.clone(),
        address,
        latitude: row.latitude,
        longitude: row.longitude,
        operator: row.operator_title,
        usage_cost: row.usage_cost,
        is_operational: row.is_operational.map(|v| v != 0),
        number_of_points: row.number_of_points,
        connectors,
        distance_km: None
    }))
}

async fn fetch_connectors(state: &Arc<AppState>, station_id: i64) -> Vec<ConnectorResponse> {
    sqlx::query!(
        r#"
        SELECT
            connection_type, formal_name, power_kw,
            amps, voltage, current_type, is_fast_charge,
            is_operational, quantity
        FROM connections
        WHERE station_id = ?1
        "#,
        station_id
    )
        .fetch_all(&state.db)
        .await
        .expect("Failed to fetch connectors")
        .into_iter()
        .map(|c| ConnectorResponse {
            type_name: c.connection_type,
            formal_name: c.formal_name,
            power_kw: c.power_kw,
            amps: c.amps,
            voltage: c.voltage,
            current_type: c.current_type,
            is_fast_charge: c.is_fast_charge.map(|v| v != 0),
            is_operational: c.is_operational.map(|v| v != 0),
            quantity: c.quantity,
        })
        .collect()
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    r * 2.0 * a.sqrt().asin()
}
