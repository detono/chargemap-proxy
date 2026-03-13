use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

#[tokio::test]
async fn test_second_ocm_sync_sends_modifiedsince() {
    let mock_server = MockServer::start().await;

    // Both syncs return empty — we only care about the request params
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let state = common::setup_test_state(
        &format!("{}/v3/poi", mock_server.uri()),
        "http://unused",
    ).await;

    // First sync — no modifiedsince
    chargemap_proxy::ocm::sync::sync_once(&state).await.unwrap();

    // Second sync — should send modifiedsince
    chargemap_proxy::ocm::sync::sync_once(&state).await.unwrap();

    // Verify the second request had modifiedsince
    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);

    let first_url = requests[0].url.to_string();
    let second_url = requests[1].url.to_string();

    assert!(!first_url.contains("modifiedsince"), "First sync should not have modifiedsince");
    assert!(second_url.contains("modifiedsince"), "Second sync should have modifiedsince");
}