use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;

mod common;

#[tokio::test]
async fn test_osm_sync_inserts_nodes() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": [
                {
                    "type": "node",
                    "id": 111111,
                    "lat": 51.05,
                    "lon": 3.71,
                    "tags": {
                        "amenity": "charging_station",
                        "operator": "Allego",
                        "capacity": "2",
                        "socket:type2": "2",
                        "socket:type2:output": "22 kW"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let state = common::setup_test_state("http://unused", &mock_server.uri()).await;

    let count = chargemap_proxy::osm::sync::sync_osm(state.clone()).await.unwrap();
    assert_eq!(count, 1);

    let station_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM stations WHERE primary_source = 'osm'"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(station_count, 1);

    let conn = sqlx::query!(
        "SELECT connection_type, power_kw, is_fast_charge FROM connections LIMIT 1"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(conn.connection_type, Some("Type 2 (Socket Only)".to_string()));
    assert_eq!(conn.power_kw, Some(22.0));
    assert_eq!(conn.is_fast_charge, Some(0)); // 22kW < 50kW
}

#[tokio::test]
async fn test_osm_deduplicates_against_existing_station() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": [
                {
                    "type": "node",
                    "id": 222222,
                    "lat": 51.05001,  // ~10m away from the Flanders station
                    "lon": 3.71001,
                    "tags": { "amenity": "charging_station" }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let state = common::setup_test_state("http://unused", &mock_server.uri()).await;

    // Pre-insert a Flanders station at nearby coordinates
    sqlx::query!(
        "INSERT INTO stations (address_line1, postcode, operator_title, latitude, longitude, is_operational, primary_source)
         VALUES ('Kiezelweg 1', '9000', 'Allego', 51.05, 3.71, 1, 'flanders')"
    )
        .execute(&state.db)
        .await
        .unwrap();

    let count = chargemap_proxy::osm::sync::sync_osm(state.clone()).await.unwrap();
    assert_eq!(count, 1);

    // Should still be only 1 station — OSM node was deduplicated
    let station_count = sqlx::query_scalar!("SELECT COUNT(*) FROM stations")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(station_count, 1);

    // But station_sources should have the OSM entry linked to the existing station
    let source_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM station_sources WHERE source = 'osm'"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(source_count, 1);
}