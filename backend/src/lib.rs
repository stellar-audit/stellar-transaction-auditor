//! Library exports for integration tests.

pub mod error;
pub mod models;
pub mod routes;
pub mod services;
pub mod state;

/// Build the full API router (for testing).
pub fn app_router(state: state::AppState) -> axum::Router {
    routes::build_router(state)
}
