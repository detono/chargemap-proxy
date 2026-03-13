use axum::extract::State;
use axum::http::StatusCode;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;

mod common;

#[tokio::test]
async fn test_refresh_cache_returns_accepted() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": []
        })))
        .mount(&mock_server)
        .await;

    let state = common::setup_test_state(
        &format!("{}/v3/poi", mock_server.uri()),
        &mock_server.uri(),
    ).await;

    let status = chargemap_proxy::routes::admin::refresh_cache(
        State(state.clone()),
    ).await;

    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_refresh_cache_triggers_ocm_sync() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "ID": 55555,
                "UUID": "refresh-test",
                "AddressInfo": {
                    "Title": "Refresh Test Station",
                    "AddressLine1": "Teststraat 1",
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

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": []
        })))
        .mount(&mock_server)
        .await;

    let state = common::setup_test_state(
        &format!("{}/v3/poi", mock_server.uri()),
        &mock_server.uri(),
    ).await;

    chargemap_proxy::ocm::sync::sync_once(&state).await.unwrap();

    // Give background tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM stations WHERE ocm_id = 55555")
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(count, 1);
}