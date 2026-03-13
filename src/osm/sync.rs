use std::sync::Arc;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};
use crate::AppState;
use crate::utils::{find_nearby_station, normalize_connector_type};

#[derive(Debug, Deserialize)]
struct OverpassResponse {
    elements: Vec<OverpassNode>,
}

#[derive(Debug, Deserialize)]
struct OverpassNode {
    id: i64,
    lat: f64,
    lon: f64,
    #[serde(default)]
    tags: HashMap<String, String>,
}

pub async fn start_sync_loop(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(60 * 60 * 24)); // 24h

    loop {
        ticker.tick().await;
        info!("Starting OSM sync...");
        match sync_osm(state.clone()).await {
            Ok(n)  => info!("OSM sync complete: {} stations", n),
            Err(e) => error!("OSM sync failed: {e}"),
        }
    }
}

pub async fn sync_osm(state: Arc<AppState>) -> Result<usize> {
    let cfg = &state.config.location;

    // Derive bounding box from center + radius
    let lat_delta = cfg.radius_km as f64 / 111.0;
    let lon_delta = cfg.radius_km as f64 / (111.0 * cfg.latitude.to_radians().cos().abs());

    let min_lat = cfg.latitude - lat_delta;
    let max_lat = cfg.latitude + lat_delta;
    let min_lon = cfg.longitude - lon_delta;
    let max_lon = cfg.longitude + lon_delta;

    let query = format!(
        "[out:json][timeout:60];node[\"amenity\"=\"charging_station\"]({},{},{},{});out body;",
        min_lat, min_lon, max_lat, max_lon
    );

    info!("Fetching OSM charging stations...");

    let response = state.http_client
        .post(&state.config.osm.url)
        .body(query)
        .send()
        .await?
        .error_for_status()?
        .json::<OverpassResponse>()
        .await?;

    info!("Got {} OSM nodes", response.elements.len());

    let mut tx = state.db.begin().await?;
    let mut count = 0;

    for node in &response.elements {
        let osm_id = node.id.to_string();
        let operator = node.tags.get("operator").cloned();
        let name     = node.tags.get("name").cloned();
        let address  = node.tags.get("addr:street").and_then(|street| {
            node.tags.get("addr:housenumber")
                .map(|num| format!("{} {}", street, num))
        });
        let postcode = node.tags.get("addr:postcode").cloned();
        let town     = node.tags.get("addr:city")
            .or_else(|| node.tags.get("addr:municipality"))
            .cloned();

        // Parse capacity as number_of_points
        let number_of_points = node.tags.get("capacity")
            .and_then(|c| c.parse::<i64>().ok());

        // Parse socket power — take first socket:*:output tag we find
        let power_kw = node.tags.iter()
            .find(|(k, _)| k.starts_with("socket:") && k.ends_with(":output"))
            .and_then(|(_, v)| parse_power_kw(v));

        // Connector type — take first socket:* key that isn't a sub-property (:output, :number etc)
        let connection_type = node.tags.iter()
            .find(|(k, _)| {
                k.starts_with("socket:") && k.matches(':').count() == 1
            })
            .map(|(k, _)| normalize_connector_type(&k.replace("socket:", "")).to_string());

        let is_fast_charge: Option<i64> = power_kw.map(|kw| if kw >= 50.0 { 1 } else { 0 });

        let address_title = name.as_deref().or(address.as_deref()).map(|s| s.to_string());

        // Dedup check before inserting
        if let Some(existing_id) = find_nearby_station(&mut tx, node.lat, node.lon, 25.0).await? {
            sqlx::query!(
            r#"INSERT INTO station_sources (station_id, source, source_id)
               VALUES (?1, 'osm', ?2)
               ON CONFLICT(source, source_id) DO UPDATE SET last_seen = datetime('now')"#,
            existing_id,
            osm_id,
        )
                .execute(&mut *tx)
                .await?;
            count += 1;
            continue;
        }

        sqlx::query!(
            r#"
            INSERT INTO stations (
                address_title, address_line1, town, postcode,
                operator_title, latitude, longitude,
                number_of_points, is_operational, primary_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 'osm')
            ON CONFLICT(address_line1, postcode, operator_title, latitude, longitude) DO UPDATE SET
                cached_at = datetime('now')
            "#,
            address_title,
            address,
            town,
            postcode,
            operator,
            node.lat,
            node.lon,
            number_of_points,
        )
            .execute(&mut *tx)
            .await?;

        let station_id = sqlx::query_scalar!(
            "SELECT id FROM stations WHERE latitude = ?1 AND longitude = ?2",
            node.lat,
            node.lon,
        )
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query!(
            r#"
            INSERT INTO station_sources (station_id, source, source_id)
            VALUES (?1, 'osm', ?2)
            ON CONFLICT(source, source_id) DO UPDATE SET last_seen = datetime('now')
            "#,
            station_id,
            osm_id,
        )
            .execute(&mut *tx)
            .await?;

        if let Some(conn_type) = connection_type {
            sqlx::query!(
                r#"
                INSERT INTO connections (
                    station_id, connection_type, power_kw, is_fast_charge, is_operational
                ) VALUES (?1, ?2, ?3, ?4, 1)
                "#,
                station_id,
                conn_type,
                power_kw,
                is_fast_charge,
            )
                .execute(&mut *tx)
                .await?;
        }

        count += 1;
    }

    tx.commit().await?;

    sqlx::query!(
        r#"
        INSERT INTO sync_state (source, last_synced_at)
        VALUES ('osm', datetime('now'))
        ON CONFLICT(source) DO UPDATE SET last_synced_at = datetime('now')
        "#
    )
        .execute(&state.db)
        .await?;

    info!("OSM sync complete: {} nodes inserted/updated", count);
    Ok(count)
}

/// Parses "22 kW" or "22kW" into 22.0
fn parse_power_kw(s: &str) -> Option<f64> {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse::<f64>().ok())
}