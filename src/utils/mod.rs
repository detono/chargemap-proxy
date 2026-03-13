
pub fn normalize_connector_type(raw: &str) -> &str {
    match raw {
        "IEC_62196_T2"       => "Type 2 (Socket Only)",
        "IEC_62196_T2_COMBO" => "CCS (Type 2)",
        "IEC_62196_T3A"      => "Type E (French)",
        "CHADEMO"            => "CHAdeMO",
        "DOMESTIC_F"         => "CEE 7/5",
        "DOMESTIC_E"         => "CEE 7/5",
        // OSM socket tags
        "type2"              => "Type 2 (Socket Only)",
        "type2_combo"        => "CCS (Type 2)",
        "chademo"            => "CHAdeMO",
        "tesla_supercharger" => "NACS / Tesla Supercharger",
        "type1"              => "Type 1 (J1772)",
        "type1_combo"        => "CCS (Type 1)",
        other                => other,
    }
}

pub async fn find_nearby_station(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lat: f64,
    lon: f64,
    radius_m: f64,
) -> anyhow::Result<Option<i64>> {
    let lat_delta = radius_m / 111_000.0;
    let lon_delta = radius_m / (111_000.0 * lat.to_radians().cos().abs());

    let min_lat = lat - lat_delta;
    let max_lat = lat + lat_delta;
    let min_lon = lon - lon_delta;
    let max_lon = lon + lon_delta;

    let candidates = sqlx::query!(
        r#"
        SELECT id, latitude, longitude FROM stations
        WHERE latitude  BETWEEN ?1 AND ?2
          AND longitude BETWEEN ?3 AND ?4
        "#,
        min_lat, max_lat,
        min_lon, max_lon,
    )
        .fetch_all(&mut **tx)
        .await?;

    for row in candidates {
        let dist_m = haversine_km(lat, lon, row.latitude, row.longitude) * 1000.0;
        if dist_m <= radius_m {
            return Ok(row.id);
        }
    }

    Ok(None)
}

pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
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
    fn test_normalize_known_types() {
        assert_eq!(normalize_connector_type("IEC_62196_T2"), "Type 2 (Socket Only)");
        assert_eq!(normalize_connector_type("IEC_62196_T2_COMBO"), "CCS (Type 2)");
        assert_eq!(normalize_connector_type("CHADEMO"), "CHAdeMO");
    }

    #[test]
    fn test_normalize_unknown_passthrough() {
        assert_eq!(normalize_connector_type("SOME_UNKNOWN"), "SOME_UNKNOWN");
    }

    #[test]
    fn test_normalize_osm_tags() {
        assert_eq!(normalize_connector_type("type2"), "Type 2 (Socket Only)");
        assert_eq!(normalize_connector_type("chademo"), "CHAdeMO");
    }
}