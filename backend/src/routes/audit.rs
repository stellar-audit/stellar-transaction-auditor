//! Audit-log CRUD and verification endpoints.

use axum::extract::{Path, Query, State};
use axum::Json;

use crate::error::ApiError;
use crate::models::{
    AdminResponse, AuditEntryDto, AuditorResponse, AuditorRoleRequest, OperationResult,
    PaginatedEntries, RecordRequest, VerifyResult,
};
use crate::state::AppState;

/// Query parameters for `GET /api/v1/entries`.
#[derive(Debug, serde::Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_start")]
    pub start: u64,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_start() -> u64 {
    0
}
fn default_limit() -> u32 {
    20
}

/// Query parameters for `GET /api/v1/verify`.
#[derive(Debug, serde::Deserialize)]
pub struct VerifyQuery {
    pub from: u64,
    pub to: u64,
}

/// `GET /api/v1/entries` — paginated list of audit entries.
pub async fn list_entries(
    State(state): State<AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<PaginatedEntries>, ApiError> {
    if q.limit == 0 || q.limit > 100 {
        return Err(ApiError::InvalidLimit(q.limit));
    }
    let entries = state.store.get_entries(q.start, q.limit)?;
    let total = state.store.entry_count();
    Ok(Json(PaginatedEntries {
        count: entries.len() as u64,
        entries,
        start: q.start,
        limit: q.limit,
        total,
    }))
}

/// `GET /api/v1/entries/:id` — fetch a single entry.
pub async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<AuditEntryDto>, ApiError> {
    let entry = state.store.get_entry(id)?;
    Ok(Json(entry))
}

/// `GET /api/v1/entries/count` — total number of recorded entries.
pub async fn entry_count(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "count": state.store.entry_count() }))
}

/// `GET /api/v1/verify?from=0&to=4` — verify hash-chain integrity.
pub async fn verify_chain(
    State(state): State<AppState>,
    Query(q): Query<VerifyQuery>,
) -> Result<Json<VerifyResult>, ApiError> {
    let verified = state.store.verify_chain(q.from, q.to)?;
    Ok(Json(VerifyResult {
        from: q.from,
        to: q.to,
        verified,
        entries_checked: q.to - q.from + 1,
    }))
}

/// `POST /api/v1/entries` — record a new audit entry.
pub async fn record_entry(
    State(state): State<AppState>,
    Json(req): Json<RecordRequest>,
) -> Result<Json<OperationResult>, ApiError> {
    let id = state.store.record(
        &req.auditor,
        &req.subject,
        req.category,
        &req.data_hash,
        &req.memo,
        req.expires_at,
    )?;
    tracing::info!(entry_id = id, auditor = %req.auditor, "Recorded audit entry");
    Ok(Json(OperationResult {
        success: true,
        message: format!("Entry {id} recorded successfully"),
    }))
}

/// `GET /api/v1/admin` — return the current admin address.
pub async fn get_admin(State(state): State<AppState>) -> Result<Json<AdminResponse>, ApiError> {
    let admin = state.store.get_admin()?;
    Ok(Json(AdminResponse { admin }))
}

/// `GET /api/v1/auditors/:addr` — check if an address holds the auditor role.
pub async fn check_auditor(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Json<AuditorResponse> {
    Json(AuditorResponse {
        is_auditor: state.store.is_auditor(&addr),
        address: addr,
    })
}

/// `POST /api/v1/admin/grant` — grant auditor role.
pub async fn grant_auditor(
    State(state): State<AppState>,
    Json(req): Json<AuditorRoleRequest>,
) -> Result<Json<OperationResult>, ApiError> {
    state.store.grant_auditor(&req.admin, &req.address)?;
    tracing::info!(address = %req.address, "Auditor role granted");
    Ok(Json(OperationResult {
        success: true,
        message: format!("Auditor role granted to {}", req.address),
    }))
}

/// `POST /api/v1/admin/revoke` — revoke auditor role.
pub async fn revoke_auditor(
    State(state): State<AppState>,
    Json(req): Json<AuditorRoleRequest>,
) -> Result<Json<OperationResult>, ApiError> {
    state.store.revoke_auditor(&req.admin, &req.address)?;
    tracing::info!(address = %req.address, "Auditor role revoked");
    Ok(Json(OperationResult {
        success: true,
        message: format!("Auditor role revoked from {}", req.address),
    }))
}

/// `POST /api/v1/admin/pause` — pause recording.
pub async fn pause(State(state): State<AppState>) -> Json<OperationResult> {
    state.store.pause();
    Json(OperationResult {
        success: true,
        message: "Recording paused".into(),
    })
}

/// `POST /api/v1/admin/unpause` — resume recording.
pub async fn unpause(State(state): State<AppState>) -> Json<OperationResult> {
    state.store.unpause();
    Json(OperationResult {
        success: true,
        message: "Recording resumed".into(),
    })
}

/// `GET /api/v1/admin/paused` — check pause status.
pub async fn is_paused(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "paused": state.store.is_paused() }))
}
