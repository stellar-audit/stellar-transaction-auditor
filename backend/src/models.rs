//! Serde-compatible data models for API requests and responses.

use serde::{Deserialize, Serialize};

/// Category of audit activity — mirrors the Soroban contract enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Payment,
    ContractCall,
    Compliance,
    Security,
    Other,
}

impl Category {
    pub fn as_u32(self) -> u32 {
        match self {
            Category::Payment => 0,
            Category::ContractCall => 1,
            Category::Compliance => 2,
            Category::Security => 3,
            Category::Other => 4,
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Payment => write!(f, "payment"),
            Category::ContractCall => write!(f, "contract_call"),
            Category::Compliance => write!(f, "compliance"),
            Category::Security => write!(f, "security"),
            Category::Other => write!(f, "other"),
        }
    }
}

/// A single audit-log entry as returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntryDto {
    pub id: u64,
    pub auditor: String,
    pub subject: String,
    pub category: Category,
    pub data_hash: String,
    pub prev_hash: String,
    pub entry_hash: String,
    pub timestamp: u64,
    pub ledger: u32,
    pub expires_at: u64,
    pub memo: String,
}

/// Request body for recording a new audit entry.
#[derive(Debug, Deserialize)]
pub struct RecordRequest {
    pub auditor: String,
    pub subject: String,
    pub category: Category,
    pub data_hash: String,
    pub memo: String,
    pub expires_at: u64,
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedEntries {
    pub entries: Vec<AuditEntryDto>,
    pub count: u64,
    pub start: u64,
    pub limit: u32,
    pub total: u64,
}

/// Chain verification result.
#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub from: u64,
    pub to: u64,
    pub verified: bool,
    pub entries_checked: u64,
}

/// Health-check response (issue #8).
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
}

/// Detailed service status.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub network: String,
    pub contract_id: String,
    pub soroban_rpc_url: String,
    pub horizon_url: String,
    pub entry_count: u64,
}

/// Admin info response.
#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub admin: String,
}

/// Auditor check response.
#[derive(Debug, Serialize)]
pub struct AuditorResponse {
    pub address: String,
    pub is_auditor: bool,
}

/// Request body for granting/revoking auditor role.
#[derive(Debug, Deserialize)]
pub struct AuditorRoleRequest {
    pub admin: String,
    pub address: String,
}

/// Generic operation result.
#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub success: bool,
    pub message: String,
}
