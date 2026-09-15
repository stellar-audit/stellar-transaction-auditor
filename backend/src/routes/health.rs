//! Health-check and status endpoints (issue #8).

use axum::extract::State;
use axum::Json;

use crate::models::{HealthResponse, StatusResponse};
use crate::state::AppState;

/// `GET /health` — lightweight liveness probe.
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

/// `GET /api/v1/status` — detailed service status with configuration info.
pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let config = &state.config;
    Json(StatusResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        network: config.network_name.clone(),
        contract_id: config.contract_id.clone(),
        soroban_rpc_url: config.soroban_rpc_url.clone(),
        horizon_url: config.horizon_url.clone(),
        entry_count: state.store.entry_count(),
    })
}
