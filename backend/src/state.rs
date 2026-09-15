//! Application state shared across all route handlers.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::services::audit_store::AuditStore;

/// Configuration loaded from environment variables (with sensible defaults).
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub contract_id: String,
    pub soroban_rpc_url: String,
    pub horizon_url: String,
    pub network_passphrase: String,
    pub network_name: String,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        let bind_addr: SocketAddr =
            format!("0.0.0.0:{bind_port}").parse().unwrap_or_else(|_| {
                "0.0.0.0:3000"
                    .parse()
                    .expect("hardcoded fallback is valid")
            });

        Self {
            bind_addr,
            contract_id: std::env::var("CONTRACT_ID").unwrap_or_default(),
            soroban_rpc_url: std::env::var("SOROBAN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org:443".into()),
            horizon_url: std::env::var("HORIZON_URL")
                .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".into()),
            network_passphrase: std::env::var("NETWORK_PASSPHRASE")
                .unwrap_or_else(|_| "Test SDF Network ; September 2015".into()),
            network_name: std::env::var("NETWORK_NAME")
                .unwrap_or_else(|_| "TESTNET".into()),
        }
    }
}

/// Shared state accessible to every handler via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub store: Arc<AuditStore>,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            store: Arc::new(AuditStore::new()),
            started_at: Instant::now(),
        }
    }
}
