use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tracing::info;
use crate::AppState;
use crate::utils::normalize_connector_type;

#[derive(Debug)]
struct FlandersRow {
    source_id: String,        // full uniek_identificatienummer
    operator: Option<String>, // uitbater
    address_line1: String,    // adres
    postcode: String,
    town: String,
    latitude: f64,
    longitude: f64,
    power_kw: Option<f64>,
    connector: String,        // IEC_62196_T2 etc, may be semicolon-separated
    current_type: String,     // ac_1_phase etc, may be semicolon-separated
}

/// Groups rows into stations by (address_line1, postcode, operator)
/// Returns a map of group_key → Vec<FlandersRow>
fn group_into_stations(rows: Vec<FlandersRow>) -> HashMap<String, Vec<FlandersRow>> {
    let mut map: HashMap<String, Vec<FlandersRow>> = HashMap::new();
    for row in rows {
        let key = format!(
            "{}|{}|{}",
            row.address_line1.to_lowercase().trim(),
            row.postcode.trim(),
            row.operator.as_deref().unwrap_or("").to_lowercase().trim()
        );
        map.entry(key).or_default().push(row);
    }
    map
}

pub async fn sync_flanders(state: Arc<AppState>) -> Result<usize> {
    let csv_path = std::env::var("FLANDERS_CSV_PATH")
        .unwrap_or_else(|_| "./data/chargers.csv".to_string());

    info!("Loading Flanders CSV from {}", csv_path);

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(&csv_path)?;

    let mut rows = Vec::new();

    for result in reader.records() {
        let record = result?;

        let source_id   = record.get(1).unwrap_or("").trim().to_string();
        let operator    = record.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let address     = record.get(8).unwrap_or("").trim().to_string();
        let postcode    = record.get(9).unwrap_or("").trim().to_string();
        let town        = record.get(10).unwrap_or("").trim().to_string();
        let power_kw    = record.get(4).and_then(|s| s.trim().parse::<f64>().ok());
        let connector   = record.get(7).unwrap_or("").trim().to_string();
        let current_type = record.get(6).unwrap_or("").trim().to_string();

        let lat = match record.get(12).and_then(|s| s.trim().parse::<f64>().ok()) {
            Some(v) => v,
            None => continue, // skip rows without coordinates
        };
        let lon = match record.get(13).and_then(|s| s.trim().parse::<f64>().ok()) {
            Some(v) => v,
            None => continue,
        };

        if source_id.is_empty() || address.is_empty() || postcode.is_empty() {
            continue;
        }

        rows.push(FlandersRow {
            source_id,
            operator,
            address_line1: address,
            postcode,
            town,
            latitude: lat,
            longitude: lon,
            power_kw,
            connector,
            current_type,
        });
    }

    info!("Parsed {} Flanders connector rows", rows.len());

    let grouped = group_into_stations(rows);
    info!("Grouped into {} Flanders stations", grouped.len());

    let mut tx = state.db.begin().await?;
    let mut count = 0;

    for (_, connectors) in &grouped {
        let first = &connectors[0];

        // Dedup check
        if let Some(existing_id) = crate::utils::find_nearby_station(&mut tx, first.latitude, first.longitude, 25.0).await? {
            for row in connectors {
                sqlx::query!(
                r#"INSERT INTO station_sources (station_id, source, source_id)
                   VALUES (?1, 'flanders', ?2)
                   ON CONFLICT(source, source_id) DO UPDATE SET last_seen = datetime('now')"#,
                existing_id,
                row.source_id,
            )
                    .execute(&mut *tx)
                    .await?;
            }
            count += 1;
            continue; // skip the station INSERT below
        }

        // Upsert station — conflict on (address_line1, postcode, operator_title)
        sqlx::query!(
            r#"
                INSERT INTO stations (
                    operator_title, address_line1, town, postcode,
                    latitude, longitude, is_operational, primary_source
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 'flanders')
                ON CONFLICT(address_line1, postcode, operator_title, latitude, longitude) DO UPDATE SET
                    cached_at = datetime('now')
            "#,
            first.operator,
            first.address_line1,
            first.town,
            first.postcode,
            first.latitude,
            first.longitude,
        )
            .execute(&mut *tx)
            .await?;

        let station_id = sqlx::query_scalar!(
            r#"
            SELECT id FROM stations
            WHERE address_line1 = ?1 AND postcode = ?2
              AND latitude = ?3 AND longitude = ?4
              AND (operator_title = ?5 OR (operator_title IS NULL AND ?5 IS NULL))
            "#,
            first.address_line1,
            first.postcode,
            first.latitude,
            first.longitude,
            first.operator,
        )
        .fetch_one(&mut *tx)
        .await?;

        // Upsert station_sources per connector row
        for row in connectors {
            sqlx::query!(
                r#"
                INSERT INTO station_sources (station_id, source, source_id)
                VALUES (?1, 'flanders', ?2)
                ON CONFLICT(source, source_id) DO UPDATE SET last_seen = datetime('now')
                "#,
                station_id,
                row.source_id,
            )
                .execute(&mut *tx)
                .await?;

            // Split multi-value connector/current_type fields
            for (conn_type, curr_type) in split_connector_pairs(&row.connector, &row.current_type) {
                sqlx::query!(
                    r#"
                    INSERT INTO connections (
                        station_id, connection_type, current_type, power_kw, is_operational
                    ) VALUES (?1, ?2, ?3, ?4, 1)
                    "#,
                    station_id,
                    conn_type,
                    curr_type,
                    row.power_kw,
                )
                    .execute(&mut *tx)
                    .await?;
            }
        }

        count += 1;
    }

    tx.commit().await?;

    // Update sync_state
    sqlx::query!(
        r#"
        INSERT INTO sync_state (source, last_synced_at)
        VALUES ('flanders', datetime('now'))
        ON CONFLICT(source) DO UPDATE SET last_synced_at = datetime('now')
        "#
    )
        .execute(&state.db)
        .await?;

    info!("Flanders sync complete: {} stations upserted", count);
    Ok(count)
}

/// Splits "IEC_62196_T2; IEC_62196_T2_COMBO" and "ac_3_phase; dc" into pairs.
/// If counts differ, zip stops at the shorter one.
fn split_connector_pairs(connector: &str, current_type: &str) -> Vec<(String, String)> {
    let connectors: Vec<&str> = connector.split(';').map(|s| s.trim()).collect();
    let currents: Vec<&str>   = current_type.split(';').map(|s| s.trim()).collect();

    connectors
        .into_iter()
        .zip(currents.into_iter())
        .map(|(c, t)| (normalize_connector_type(c).to_string(), t.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_single_connector() {
        let pairs = split_connector_pairs("IEC_62196_T2", "ac_3_phase");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "Type 2 (Socket Only)");
        assert_eq!(pairs[0].1, "ac_3_phase");
    }

    #[test]
    fn test_split_multi_connector() {
        let pairs = split_connector_pairs("IEC_62196_T2; IEC_62196_T2_COMBO", "ac_3_phase; dc");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "Type 2 (Socket Only)");
        assert_eq!(pairs[1].0, "CCS (Type 2)");
    }

    #[test]
    fn test_split_mismatched_counts() {
        // zip stops at shorter — no panic
        let pairs = split_connector_pairs("IEC_62196_T2; IEC_62196_T2_COMBO", "ac_3_phase");
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_group_same_address_same_operator() {
        let rows = vec![
            FlandersRow {
                source_id: "uuid1".to_string(),
                operator: Some("Allego".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
            FlandersRow {
                source_id: "uuid2".to_string(),
                operator: Some("Allego".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
        ];

        let grouped = group_into_stations(rows);
        assert_eq!(grouped.len(), 1);
        let first = grouped.values().next().unwrap();
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn test_group_different_operator_same_address() {
        let rows = vec![
            FlandersRow {
                source_id: "uuid1".to_string(),
                operator: Some("Allego".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
            FlandersRow {
                source_id: "uuid2".to_string(),
                operator: Some("Eneco".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
        ];

        let grouped = group_into_stations(rows);
        assert_eq!(grouped.len(), 2);
    }

    #[test]
    fn test_group_case_insensitive() {
        let rows = vec![
            FlandersRow {
                source_id: "uuid1".to_string(),
                operator: Some("ALLEGO".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
            FlandersRow {
                source_id: "uuid2".to_string(),
                operator: Some("allego".to_string()),
                address_line1: "Kiezelweg 1".to_string(),
                postcode: "9000".to_string(),
                town: "Gent".to_string(),
                latitude: 51.05,
                longitude: 3.71,
                power_kw: Some(22.0),
                connector: "IEC_62196_T2".to_string(),
                current_type: "ac_3_phase".to_string(),
            },
        ];

        let grouped = group_into_stations(rows);
        assert_eq!(grouped.len(), 1);
    }
}