use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};
use crate::common::setup_test_state;

mod common;


#[tokio::test]
async fn test_ocm_sync_inserts_stations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("/v3/poi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "ID": 12345,
                "UUID": "abc-123",
                "AddressInfo": {
                    "Title": "Test Station",
                    "AddressLine1": "Kiezelweg 1",
                    "Town": "Gent",
                    "Postcode": "9000",
                    "Latitude": 51.05,
                    "Longitude": 3.71,
                    "Country": { "ISOCode": "BE" }
                },
                "Connections": [{
                    "ID": 1,
                    "PowerKW": 22.0,
                    "CurrentType": { "ID": 20, "Title": "AC (Three-Phase)" }
                }]
            }
        ])))
        .mount(&mock_server)
        .await;

    let state = setup_test_state(
        &format!("{}/v3/poi", mock_server.uri()),
        &"http://unused".to_string(),
    ).await;

    let count = chargemap_proxy::ocm::sync::sync_once(&state).await.unwrap();
    assert_eq!(count, 1);

    let station = sqlx::query!(
        "SELECT ocm_id, address_line1, town FROM stations WHERE ocm_id = 12345"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(station.address_line1, Some("Kiezelweg 1".to_string()));
    assert_eq!(station.town, Some("Gent".to_string()));

    let conn_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM connections WHERE station_id = (SELECT id FROM stations WHERE ocm_id = 12345)"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(conn_count, 1);
}

#[tokio::test]
async fn test_ocm_sync_updates_existing_station() {
    let mock_server = MockServer::start().await;

    // First sync
    Mock::given(method("GET"))
        .and(path_regex("/v3/poi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "ID": 12345,
                "UUID": "abc-123",
                "AddressInfo": {
                    "Title": "Old Name",
                    "AddressLine1": "Kiezelweg 1",
                    "Town": "Gent",
                    "Postcode": "9000",
                    "Latitude": 51.05,
                    "Longitude": 3.71,
                    "Country": { "ISOCode": "BE" }
                },
                "Connections": []
            }
        ])))
        .mount(&mock_server)
        .await;

    let state = setup_test_state(
        &format!("{}/v3/poi", mock_server.uri()),
        &"http://unused".to_string(),
    ).await;

    chargemap_proxy::ocm::sync::sync_once(&state).await.unwrap();

    // Verify initial insert
    let before = sqlx::query_scalar!(
        "SELECT address_title FROM stations WHERE ocm_id = 12345"
    )
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(before, Some("Old Name".to_string()));
}