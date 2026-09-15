//! In-memory audit-log store that mirrors the Soroban contract's hash-chaining
//! logic. This allows the API to function standalone (for development and
//! demos) and can be replaced with on-chain reads via Soroban RPC in production.

use std::collections::HashMap;
use std::sync::RwLock;

use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::models::{AuditEntryDto, Category};

/// Internal representation of a stored entry.
#[derive(Debug, Clone)]
struct StoredEntry {
    id: u64,
    auditor: String,
    subject: String,
    category: Category,
    data_hash: String,
    prev_hash: String,
    entry_hash: String,
    timestamp: u64,
    ledger: u32,
    expires_at: u64,
    memo: String,
}

/// Thread-safe in-memory audit log.
pub struct AuditStore {
    entries: RwLock<Vec<StoredEntry>>,
    auditors: RwLock<HashMap<String, bool>>,
    admin: RwLock<Option<String>>,
    paused: RwLock<bool>,
    mock_ledger: RwLock<u32>,
    mock_timestamp: RwLock<u64>,
}

impl AuditStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            auditors: RwLock::new(HashMap::new()),
            admin: RwLock::new(None),
            paused: RwLock::new(false),
            mock_ledger: RwLock::new(1),
            mock_timestamp: RwLock::new({
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }),
        }
    }

    /// Initialize the store with an admin address (mirrors contract `initialize`).
    pub fn initialize(&self, admin: String) -> Result<(), ApiError> {
        let mut current = self.admin.write().unwrap();
        if current.is_some() {
            return Err(ApiError::InvalidInput(
                "Already initialized".into(),
            ));
        }
        *current = Some(admin);
        Ok(())
    }

    /// Check if the store has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.admin.read().unwrap().is_some()
    }

    /// Get the admin address.
    pub fn get_admin(&self) -> Result<String, ApiError> {
        self.admin
            .read()
            .unwrap()
            .clone()
            .ok_or(ApiError::ContractNotConfigured)
    }

    /// Grant auditor role.
    pub fn grant_auditor(&self, _admin: &str, address: &str) -> Result<(), ApiError> {
        let mut auditors = self.auditors.write().unwrap();
        auditors.insert(address.to_string(), true);
        Ok(())
    }

    /// Revoke auditor role.
    pub fn revoke_auditor(&self, _admin: &str, address: &str) -> Result<(), ApiError> {
        let mut auditors = self.auditors.write().unwrap();
        auditors.remove(address);
        Ok(())
    }

    /// Check if an address is an auditor.
    pub fn is_auditor(&self, address: &str) -> bool {
        *self.auditors.read().unwrap().get(address).unwrap_or(&false)
    }

    /// Pause recording.
    pub fn pause(&self) {
        *self.paused.write().unwrap() = true;
    }

    /// Unpause recording.
    pub fn unpause(&self) {
        *self.paused.write().unwrap() = false;
    }

    /// Is recording paused?
    pub fn is_paused(&self) -> bool {
        *self.paused.read().unwrap()
    }

    /// Record a new audit entry (mirrors contract `record`).
    pub fn record(
        &self,
        auditor: &str,
        subject: &str,
        category: Category,
        data_hash: &str,
        memo: &str,
        expires_at: u64,
    ) -> Result<u64, ApiError> {
        // Validation (mirrors contract logic).
        if data_hash == &"0".repeat(64) || data_hash.is_empty() {
            return Err(ApiError::InvalidInput(
                "data_hash must not be zero".into(),
            ));
        }
        if auditor == subject {
            return Err(ApiError::InvalidInput(
                "auditor cannot audit themselves".into(),
            ));
        }
        let now = *self.mock_timestamp.read().unwrap();
        if expires_at <= now {
            return Err(ApiError::InvalidInput(
                "expires_at must be strictly in the future".into(),
            ));
        }
        if memo.len() > 512 {
            return Err(ApiError::InvalidInput(
                "memo exceeds 512 bytes".into(),
            ));
        }

        let mut entries = self.entries.write().unwrap();
        let id = entries.len() as u64;

        let prev_hash = entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| "0".repeat(64));

        let entry_hash = compute_entry_hash(
            id,
            auditor,
            subject,
            category,
            data_hash,
            &prev_hash,
            now,
        );

        let entry = StoredEntry {
            id,
            auditor: auditor.to_string(),
            subject: subject.to_string(),
            category,
            data_hash: data_hash.to_string(),
            prev_hash: prev_hash.clone(),
            entry_hash: entry_hash.clone(),
            timestamp: now,
            ledger: *self.mock_ledger.read().unwrap(),
            expires_at,
            memo: memo.to_string(),
        };

        entries.push(entry);
        Ok(id)
    }

    /// Get a single entry by id.
    pub fn get_entry(&self, id: u64) -> Result<AuditEntryDto, ApiError> {
        let entries = self.entries.read().unwrap();
        entries
            .get(id as usize)
            .map(|e| e.to_dto())
            .ok_or(ApiError::EntryNotFound(id))
    }

    /// Get entries with pagination.
    pub fn get_entries(&self, start: u64, limit: u32) -> Result<Vec<AuditEntryDto>, ApiError> {
        if limit == 0 || limit > 100 {
            return Err(ApiError::InvalidLimit(limit));
        }
        let entries = self.entries.read().unwrap();
        let total = entries.len() as u64;
        if start >= total {
            return Ok(Vec::new());
        }
        let end = std::cmp::min(start + limit as u64, total);
        Ok(entries[start as usize..end as usize]
            .iter()
            .map(|e| e.to_dto())
            .collect())
    }

    /// Total entry count.
    pub fn entry_count(&self) -> u64 {
        self.entries.read().unwrap().len() as u64
    }

    /// Verify the hash chain for entries [from, to] inclusive.
    pub fn verify_chain(&self, from: u64, to: u64) -> Result<bool, ApiError> {
        let entries = self.entries.read().unwrap();
        let total = entries.len() as u64;
        if total == 0 || from > to || to >= total {
            return Err(ApiError::InvalidRange { from, to });
        }

        for i in from..=to {
            let entry = &entries[i as usize];

            // Recompute hash.
            let recomputed = compute_entry_hash(
                entry.id,
                &entry.auditor,
                &entry.subject,
                entry.category,
                &entry.data_hash,
                &entry.prev_hash,
                entry.timestamp,
            );
            if recomputed != entry.entry_hash {
                return Ok(false);
            }

            // Check linkage.
            if entry.id == 0 {
                if entry.prev_hash != "0".repeat(64) {
                    return Ok(false);
                }
            } else {
                let prev = &entries[(entry.id - 1) as usize];
                if entry.prev_hash != prev.entry_hash {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Seed the store with sample data for demo purposes.
    pub fn seed_demo_data(&self) {
        if self.is_initialized() {
            return;
        }
        let _ = self.initialize("GDQP2PQPM3XKKD4FQ7LG7N4XQ4FQ7LG7N4XQ4FQ7LG7N4XQ4FQ7".to_string());
        let admin = "GDQP2PQPM3XKKD4FQ7LG7N4XQ4FQ7LG7N4XQ4FQ7LG7N4XQ4FQ7";
        let _ = self.grant_auditor(admin, "GA2C5RFPE6GCKMY3US5GAB29QZYE7DYXTZPGHLDJ5YBJO7VOZKAUBP2X");

        let auditor = "GA2C5RFPE6GCKMY3US5GAB29QZYE7DYXTZPGHLDJ5YBJO7VOZKAUBP2X";
        let subject1 = "GDFX4P3OZO5NHNXHKDI5G4QXTZKQO4NHPKWXDWQZQFILXKZKABZ6WZ2N";
        let subject2 = "GCXV2VWQFJZWCY2PHEJF5XMQ5RZQ3PKXEXZQKOOYZOJXFFAAYQJYTSK";

        let now = *self.mock_timestamp.read().unwrap();

        let _ = self.record(
            auditor,
            subject1,
            Category::Payment,
            &hash_hex(b"tx-001-payment-payload"),
            "Testnet payment audit — entry #0",
            now + 86400,
        );
        let _ = self.record(
            auditor,
            subject2,
            Category::ContractCall,
            &hash_hex(b"tx-002-soroban-invoke"),
            "Soroban contract invocation audit — entry #1",
            now + 172800,
        );
        let _ = self.record(
            auditor,
            subject1,
            Category::Compliance,
            &hash_hex(b"tx-003-kyc-check"),
            "KYC verification attestation — entry #2",
            now + 259200,
        );
        let _ = self.record(
            auditor,
            subject2,
            Category::Security,
            &hash_hex(b"tx-004-anomaly-flag"),
            "Anomaly detection flag — entry #3",
            now + 432000,
        );
        let _ = self.record(
            auditor,
            subject1,
            Category::Other,
            &hash_hex(b"tx-005-manual-review"),
            "Manual compliance review — entry #4",
            now + 604800,
        );

        tracing::info!("Seeded 5 demo audit entries");
    }
}

impl StoredEntry {
    fn to_dto(&self) -> AuditEntryDto {
        AuditEntryDto {
            id: self.id,
            auditor: self.auditor.clone(),
            subject: self.subject.clone(),
            category: self.category,
            data_hash: self.data_hash.clone(),
            prev_hash: self.prev_hash.clone(),
            entry_hash: self.entry_hash.clone(),
            timestamp: self.timestamp,
            ledger: self.ledger,
            expires_at: self.expires_at,
            memo: self.memo.clone(),
        }
    }
}

/// Compute the chain hash for an entry — mirrors the Soroban contract.
fn compute_entry_hash(
    id: u64,
    auditor: &str,
    subject: &str,
    category: Category,
    data_hash: &str,
    prev_hash: &str,
    timestamp: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.to_be_bytes());
    hasher.update(auditor.as_bytes());
    hasher.update(subject.as_bytes());
    hasher.update(category.as_u32().to_be_bytes());
    hasher.update(data_hash.as_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update(timestamp.to_be_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Hash arbitrary bytes to a hex string (for demo seeding).
fn hash_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new()
    }
}
