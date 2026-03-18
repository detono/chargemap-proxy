use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use chargemap_proxy::build_router;

mod common;

fn parse_paginated(body: &[u8]) -> serde_json::Value {
    let json: serde_json::Value = serde_json::from_slice(body).unwrap();
    json
}

async fn insert_test_station(
    db: &sqlx::SqlitePool,
    ocm_id: i64,
    address: &str,
    postcode: &str,
    town: &str,
    lat: f64,
    lon: f64,
    operator: &str,
) -> i64 {
    sqlx::query_scalar!(
        r#"INSERT INTO stations (ocm_id, address_line1, postcode, town, operator_title, latitude, longitude, is_operational, primary_source)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'ocm')
           RETURNING id"#,
        ocm_id, address, postcode, town, operator, lat, lon
    )
        .fetch_one(db)
        .await
        .unwrap()
}

async fn insert_test_connection(
    db: &sqlx::SqlitePool,
    station_id: i64,
    connection_type: &str,
    power_kw: f64,
    is_fast_charge: i64,
) {
    sqlx::query!(
        "INSERT INTO connections (station_id, connection_type, power_kw, is_fast_charge, is_operational) VALUES (?1, ?2, ?3, ?4, 1)",
        station_id, connection_type, power_kw, is_fast_charge
    )
        .execute(db)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_list_stations_returns_200() {
    let state = common::setup_test_state("http://unused", "http://unused").await;
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_stations_requires_auth() {
    let state = common::setup_test_state("http://unused", "http://unused").await;
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_stations_radius_filter() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    // Station within 5km of Ghent center
    let id1 = insert_test_station(&state.db, 1, "Kiezelweg 1", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id1, "Type 2 (Socket Only)", 22.0, 0).await;

    // Station far away (Brussels)
    let id2 = insert_test_station(&state.db, 2, "Grote Markt 1", "1000", "Brussel", 50.85, 4.35, "Allego").await;
    insert_test_connection(&state.db, id2, "Type 2 (Socket Only)", 22.0, 0).await;

    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?lat=51.0543&lon=3.7174&radius_km=5")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);

    assert_eq!(json["data"].as_array().unwrap().len(), 1);
    assert_eq!(json["total"], 1);
    assert_eq!(json["data"][0]["address"], "Kiezelweg 1, Gent, 9000");
}

#[tokio::test]
async fn test_list_stations_min_power_filter() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    let id1 = insert_test_station(&state.db, 1, "Kiezelweg 1", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id1, "Type 2 (Socket Only)", 22.0, 0).await;

    let id2 = insert_test_station(&state.db, 2, "Stationstraat 5", "9000", "Gent", 51.06, 3.72, "Allego").await;
    insert_test_connection(&state.db, id2, "CCS (Type 2)", 150.0, 1).await;

    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?min_power_kw=100")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
    assert_eq!(json["total"], 1);
    assert_eq!(json["data"][0]["address"], "Stationstraat 5, Gent, 9000");
}

#[tokio::test]
async fn test_list_stations_fast_charge_filter() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    let id1 = insert_test_station(&state.db, 1, "Kiezelweg 1", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id1, "Type 2 (Socket Only)", 22.0, 0).await;

    let id2 = insert_test_station(&state.db, 2, "Stationstraat 5", "9000", "Gent", 51.06, 3.72, "Allego").await;
    insert_test_connection(&state.db, id2, "CCS (Type 2)", 150.0, 1).await;

    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?fast_charge_only=true")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
    assert_eq!(json["total"], 1);
    assert_eq!(json["data"][0]["address"], "Stationstraat 5, Gent, 9000");
}

#[tokio::test]
async fn test_pagination_limit_and_offset() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    // Insert 5 stations
    for i in 1..=5 {
        let id = insert_test_station(&state.db, i, &format!("Straat {}", i), "9000", "Gent", 51.05 + i as f64 * 0.01, 3.71, "Allego").await;
        insert_test_connection(&state.db, id, "Type 2 (Socket Only)", 22.0, 0).await;
    }

    let app = build_router(state.clone());

    // First page
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/stations?limit=2&offset=0")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["total"], 5);
    assert_eq!(json["limit"], 2);
    assert_eq!(json["offset"], 0);
    assert_eq!(json["data"].as_array().unwrap().len(), 2);

    // Second page
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/stations?limit=2&offset=2")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["total"], 5);
    assert_eq!(json["data"].as_array().unwrap().len(), 2);

    // Last page (partial)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?limit=2&offset=4")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["total"], 5);
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_pagination_offset_beyond_total() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    let id = insert_test_station(&state.db, 1, "Straat 1", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id, "Type 2 (Socket Only)", 22.0, 0).await;

    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?limit=10&offset=999")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    assert_eq!(json["total"], 1);
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_total_reflects_connector_filter() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    // 3 stations: 2 with fast charge, 1 without
    for i in 1..=2 {
        let id = insert_test_station(&state.db, i, &format!("Fast Street {}", i), "9000", "Gent", 51.05, 3.71, "Allego").await;
        insert_test_connection(&state.db, id, "CCS (Type 2)", 150.0, 1).await;
    }
    let id = insert_test_station(&state.db, 3, "Slow Street 1", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id, "Type 2 (Socket Only)", 22.0, 0).await;

    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations?fast_charge_only=true&limit=1&offset=0")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json = parse_paginated(&body);
    // total should be 2 (post-filter), not 3 (raw)
    assert_eq!(json["total"], 2);
    assert_eq!(json["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_get_station_by_id() {
    let state = common::setup_test_state("http://unused", "http://unused").await;

    let id = insert_test_station(&state.db, 99, "Testlaan 10", "9000", "Gent", 51.05, 3.71, "Allego").await;
    insert_test_connection(&state.db, id, "Type 2 (Socket Only)", 22.0, 0).await;

    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/stations/{}", id))
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let station: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(station["address"], "Testlaan 10, Gent, 9000");
    assert_eq!(station["connectors"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_get_station_not_found() {
    let state = common::setup_test_state("http://unused", "http://unused").await;
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stations/99999")
                .header("x-api-key", "test-app-key")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let station: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(station.is_null());
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = common::setup_test_state("http://unused", "http://unused").await;
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}