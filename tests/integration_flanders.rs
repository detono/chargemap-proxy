use crate::common::setup_test_state;

mod common;

#[tokio::test]
async fn test_flanders_sync_inserts_stations() {
    let state = setup_test_state(&"", &"").await;

    // Write a minimal CSV to a temp file
    let csv = "\u{feff}laadpunt_teller,uniek_identificatienummer,uitbater,toegankelijkheid,kw,snelheid,stroomtype,connector,adres,postcode,gemeente,provincie,latitude,longitude,vervoerregio,geometry\n\
               1,abc-123__EVSE1,Allego,publiek,22,normaal,ac_3_phase,IEC_62196_T2,Kiezelweg 1,9000,Gent,Oost-Vlaanderen,51.05,3.71,Gent,POINT (0 0)\n\
               1,abc-123__EVSE2,Allego,publiek,22,normaal,ac_3_phase,IEC_62196_T2,Kiezelweg 1,9000,Gent,Oost-Vlaanderen,51.05,3.71,Gent,POINT (0 0)\n";

    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), csv).unwrap();

    let count = chargemap_proxy::flanders::sync::sync_flanders(
        state.clone(),
        Some(tmp.path().to_str().unwrap())
    ).await.unwrap();

    assert_eq!(count, 1); // 2 connectors grouped into 1 station

    let station_count = sqlx::query_scalar!("SELECT COUNT(*) FROM stations WHERE primary_source = 'flanders'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(station_count, 1);

    let conn_count = sqlx::query_scalar!("SELECT COUNT(*) FROM connections")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(conn_count, 2); // one connection per CSV row

    let source_count = sqlx::query_scalar!("SELECT COUNT(*) FROM station_sources WHERE source = 'flanders'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(source_count, 2); // one source entry per EVSE
}

#[tokio::test]
async fn test_flanders_sync_normalizes_connector_types() {
    let state = setup_test_state(&"", &"").await;

    let csv = "\u{feff}laadpunt_teller,uniek_identificatienummer,uitbater,toegankelijkheid,kw,snelheid,stroomtype,connector,adres,postcode,gemeente,provincie,latitude,longitude,vervoerregio,geometry\n\
               1,abc-456__EVSE1,Allego,publiek,50,snel,dc,IEC_62196_T2_COMBO,Stationstraat 5,9000,Gent,Oost-Vlaanderen,51.06,3.72,Gent,POINT (0 0)\n";

    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), csv).unwrap();

    chargemap_proxy::flanders::sync::sync_flanders(
        state.clone(),
        Some(tmp.path().to_str().unwrap())
    ).await.unwrap();

    let conn_type = sqlx::query_scalar!("SELECT connection_type FROM connections LIMIT 1")
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(conn_type, Some("CCS (Type 2)".to_string()));
}