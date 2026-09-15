//! # Stellar Transaction Auditor — Backend API
//!
//! A REST API server built with Axum that serves audit log data, provides
//! chain verification endpoints, and exposes health-check and metadata
//! endpoints for operational monitoring.
//!
//! ## Endpoints
//! - `GET  /health`               — liveness probe
//! - `GET  /api/v1/status`        — detailed service status
//! - `GET  /api/v1/entries`       — paginated audit entries (query: `start`, `limit`)
//! - `GET  /api/v1/entries/:id`   — single entry by id
//! - `GET  /api/v1/entries/count` — total entry count
//! - `GET  /api/v1/verify?from=&to=` — verify hash-chain integrity
//! - `GET  /api/v1/admin`         — current admin address
//! - `GET  /api/v1/auditors/:addr`— check auditor role
//! - `POST /api/v1/entries`       — record a new audit entry (mock/relay)
//! - `POST /api/v1/admin/grant`   — grant auditor role (mock/relay)
//! - `POST /api/v1/admin/revoke`  — revoke auditor role (mock/relay)

use stellar_transaction_auditor_backend::{app_router, state::AppState, state::Config};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,stellar_transaction_auditor_backend=debug".into()),
        )
        .with_target(true)
        .init();

    let config = Config::from_env();
    tracing::info!(
        bind = %config.bind_addr,
        contract_id = %config.contract_id,
        soroban_rpc_url = %config.soroban_rpc_url,
        horizon_url = %config.horizon_url,
        network_passphrase = %config.network_passphrase,
        "Starting Stellar Transaction Auditor backend"
    );

    let app_state = AppState::new(config.clone());

    // Seed demo data so the API is functional out of the box for
    // development, demos, and the Drips Wave review.
    if std::env::var("SKIP_SEED").unwrap_or_default() != "1" {
        app_state.store.seed_demo_data();
    }

    let app = app_router(app_state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to bind to {}: {e}", config.bind_addr);
        });

    tracing::info!("Listening on http://{}", config.bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, stopping gracefully...");
}
