use axum::{extract::State, http::StatusCode, routing::post, Router};
use std::sync::Arc;
use tokio::spawn;
use crate::{flanders, AppState};

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/refresh", post(refresh_cache))
}

pub async fn refresh_cache(
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    //Refresh cache of OCM
    let ocm_state = state.clone();
    spawn(async move {
        match crate::ocm::sync::sync_once(&ocm_state).await {
            Ok(n)  => tracing::info!("Manual refresh upserted {} stations", n),
            Err(e) => tracing::error!("Manual refresh failed: {e}"),
        }
    });

    //Refresh cache of OSM
    let osm_state = state.clone();
    spawn(async move {
        match crate::osm::sync::sync_osm(osm_state).await {
            Ok(n)  => tracing::info!("Manual refresh upserted {} stations", n),
            Err(e) => tracing::error!("Manual refresh failed: {e}"),
        }
    });

    //Refresh cache of Flanders
    let flanders_state = state.clone();
    spawn(async move {
        match sqlx::query!("DELETE FROM stations WHERE primary_source = 'flanders'")
            .execute(&flanders_state.db)
            .await
        {
            Ok(_) => {
                match flanders::sync::sync_flanders(flanders_state, None).await {
                    Ok(n)  => tracing::info!("Manual Flanders refresh: {} stations", n),
                    Err(e) => tracing::error!("Manual Flanders refresh failed: {e}"),
                }
            }
            Err(e) => tracing::error!("Flanders delete failed: {e}"),
        }
    });

    StatusCode::ACCEPTED
}