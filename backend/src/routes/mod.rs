pub mod audit;
pub mod health;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// Build the full API router.
pub fn build_router(state: AppState) -> Router {
    let api_v1 = Router::new()
        .route("/status", get(health::status))
        .route("/entries", get(audit::list_entries).post(audit::record_entry))
        .route("/entries/count", get(audit::entry_count))
        .route("/entries/:id", get(audit::get_entry))
        .route("/verify", get(audit::verify_chain))
        .route("/admin", get(audit::get_admin))
        .route("/auditors/:addr", get(audit::check_auditor))
        .route("/admin/grant", post(audit::grant_auditor))
        .route("/admin/revoke", post(audit::revoke_auditor))
        .route("/admin/pause", post(audit::pause))
        .route("/admin/unpause", post(audit::unpause))
        .route("/admin/paused", get(audit::is_paused));

    Router::new()
        .route("/health", get(health::health))
        .nest("/api/v1", api_v1)
        .with_state(state)
}
