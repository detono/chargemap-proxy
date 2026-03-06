use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};
use crate::AppState;
use crate::ocm::client::fetch_stations;

pub async fn sync_once(state: &Arc<AppState>) -> anyhow::Result<usize> {
    let stations = fetch_stations(
        &state.http_client,
        &state.ocm_api_key,
        state.config.location.latitude,
        state.config.location.longitude,
        state.config.location.radius_km,
    ).await?;

    let count = upsert_stations(state, stations).await?;
    Ok(count)
}

pub async fn start_sync_loop(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(state.config.cache.refresh_interval_seconds));

    loop {
        ticker.tick().await;
        info!("Starting OCM sync...");
        match sync_once(&state).await {
            Ok(n)  => info!("Upserted {} stations into DB", n),
            Err(e) => error!("OCM sync failed: {e}"),
        }
    }
}


async fn upsert_stations(state: &Arc<AppState>, stations: Vec<crate::ocm::types::OcmStation>) -> anyhow::Result<usize> {
    let mut tx = state.db.begin().await?;
    let mut count = 0;

    for station in &stations {
        // station-level let bindings
        let operator_id    = station.operator_info.as_ref().and_then(|o| o.id);
        let usage_type_id    = station.usage_type.as_ref().and_then(|u| u.id);
        let usage_type_title = station.usage_type.as_ref().and_then(|u| u.title.as_deref());
        let status_type_id   = station.status_type.as_ref().and_then(|s| s.id);
        let is_operational   = station.status_type.as_ref().and_then(|s| s.is_operational);
        let country_iso      = station.address_info.country.as_ref().and_then(|c| c.iso_code.as_deref());

        let operator_title = station.operator_info
            .as_ref()
            .and_then(|o| o.title.as_deref())
            .filter(|t| *t != "(Unknown Operator)");

        sqlx::query!(
            r#"
            INSERT INTO stations (
                id, uuid, operator_id, operator_title,
                usage_type_id, usage_type_title, usage_cost,
                status_type_id, is_operational,
                address_title, address_line1, town, state_or_province,
                postcode, country_iso, latitude, longitude,
                access_comments, related_url, contact_telephone,
                number_of_points, general_comments,
                is_recently_verified, date_last_verified,
                date_last_status_update, date_created, cached_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26,
                datetime('now')
            )
            ON CONFLICT(id) DO UPDATE SET
                uuid                    = excluded.uuid,
                operator_id             = excluded.operator_id,
                operator_title          = excluded.operator_title,
                usage_type_id           = excluded.usage_type_id,
                usage_type_title        = excluded.usage_type_title,
                usage_cost              = excluded.usage_cost,
                status_type_id          = excluded.status_type_id,
                is_operational          = excluded.is_operational,
                address_title           = excluded.address_title,
                address_line1           = excluded.address_line1,
                town                    = excluded.town,
                state_or_province       = excluded.state_or_province,
                postcode                = excluded.postcode,
                country_iso             = excluded.country_iso,
                latitude                = excluded.latitude,
                longitude               = excluded.longitude,
                access_comments         = excluded.access_comments,
                related_url             = excluded.related_url,
                contact_telephone       = excluded.contact_telephone,
                number_of_points        = excluded.number_of_points,
                general_comments        = excluded.general_comments,
                is_recently_verified    = excluded.is_recently_verified,
                date_last_verified      = excluded.date_last_verified,
                date_last_status_update = excluded.date_last_status_update,
                date_created            = excluded.date_created,
                cached_at               = datetime('now')
            "#,
            station.id,
            station.uuid,
            operator_id,
            operator_title,
            usage_type_id,
            usage_type_title,
            station.usage_cost,
            status_type_id,
            is_operational,
            station.address_info.title,
            station.address_info.address_line1,
            station.address_info.town,
            station.address_info.state_or_province,
            station.address_info.postcode,
            country_iso,
            station.address_info.latitude,
            station.address_info.longitude,
            station.address_info.access_comments,
            station.address_info.related_url,
            station.address_info.contact_telephone1,
            station.number_of_points,
            station.general_comments,
            station.is_recently_verified,
            station.date_last_verified,
            station.date_last_status_update,
            station.date_created,
        )
            .execute(&mut *tx)
            .await?;

        sqlx::query!("DELETE FROM connections WHERE station_id = ?1", station.id)
            .execute(&mut *tx)
            .await?;

        if let Some(conns) = &station.connections {
            for conn in conns {
                let conn_type_id    = conn.connection_type.as_ref().and_then(|c| c.id);
                let conn_type_title = conn.connection_type.as_ref().and_then(|c| c.title.as_deref());
                let formal_name     = conn.connection_type.as_ref().and_then(|c| c.formal_name.as_deref());
                let level_id        = conn.level.as_ref().and_then(|l| l.id);
                let level_title     = conn.level.as_ref().and_then(|l| l.title.as_deref());
                let is_fast_charge  = conn.level.as_ref().and_then(|l| l.is_fast_charge_capable);
                let current_type_id = conn.current_type.as_ref().and_then(|c| c.id);
                let current_type    = conn.current_type.as_ref().and_then(|c| c.title.as_deref());
                let conn_status_id  = conn.status_type.as_ref().and_then(|s| s.id);
                let conn_operational = conn.status_type.as_ref().and_then(|s| s.is_operational);

                sqlx::query!(
                    r#"
                    INSERT INTO connections (
                        id, station_id,
                        connection_type_id, connection_type, formal_name,
                        level_id, level_title, is_fast_charge,
                        current_type_id, current_type,
                        amps, voltage, power_kw, quantity,
                        status_type_id, is_operational, comments
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                        ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
                    )
                    "#,
                    conn.id,
                    station.id,
                    conn_type_id,
                    conn_type_title,
                    formal_name,
                    level_id,
                    level_title,
                    is_fast_charge,
                    current_type_id,
                    current_type,
                    conn.amps,
                    conn.voltage,
                    conn.power_kw,
                    conn.quantity,
                    conn_status_id,
                    conn_operational,
                    conn.comments,
                )
                    .execute(&mut *tx)
                    .await?;
            }
        }

        count += 1;
    }

    tx.commit().await?;
    Ok(count)
}