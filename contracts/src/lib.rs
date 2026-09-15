#![no_std]
//! # Stellar Transaction Auditor — Soroban contract
//!
//! A **tamper-evident, append-only audit log** for activity on the Stellar
//! network. Authorized auditors append entries describing a transaction,
//! contract call, or compliance/security observation. Every entry is
//! **hash-chained** to the one before it: each entry stores the hash of its
//! predecessor and its own hash is computed over that link. As a result the
//! whole log can be independently re-derived and any mutation of a historical
//! entry breaks the chain from that point forward — this is the "comprehensive
//! logging and verification" the project promises.
//!
//! ## Roles
//! - **Admin** — set once at [`initialize`]. Grants/revokes auditors and
//!   pauses/unpauses recording.
//! - **Auditor** — may append entries via [`record`].
//!
//! ## Integrity model
//! `entry_hash = sha256(id ‖ auditor ‖ subject ‖ category ‖ data_hash ‖ prev_hash ‖ timestamp)`
//!
//! `prev_hash` of entry *n* is the `entry_hash` of entry *n-1* (the genesis
//! entry links to 32 zero bytes). [`verify_chain`] recomputes each hash and
//! confirms the linkage, returning `false` if any entry was tampered with.

use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype, panic_with_error,
    symbol_short, xdr::ToXdr, Address, Bytes, BytesN, Env, String, Vec,
};

// Contract metadata (surfaced by tooling / explorers).
contractmeta!(
    key = "Description",
    val = "Tamper-evident hash-chained audit log for Stellar activity"
);

/// Maximum length (in bytes) allowed for an entry's free-form `memo`.
const MAX_MEMO_LEN: u32 = 512;
/// Upper bound on how many entries a single [`get_entries`] page may return.
const MAX_PAGE_LIMIT: u32 = 100;

/// Category of the activity an audit entry describes.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Payment = 0,
    ContractCall = 1,
    Compliance = 2,
    Security = 3,
    Other = 4,
}

/// A single, immutable audit-log entry.
#[contracttype]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AuditEntry {
    /// Zero-based sequential id (also the storage key).
    pub id: u64,
    /// Auditor who recorded the entry.
    pub auditor: Address,
    /// Account/contract the entry is about.
    pub subject: Address,
    /// What kind of activity this entry describes.
    pub category: Category,
    /// Caller-supplied hash of the off-chain payload being attested to.
    pub data_hash: BytesN<32>,
    /// `entry_hash` of the previous entry (32 zero bytes for the genesis entry).
    pub prev_hash: BytesN<32>,
    /// Hash of this entry — the chain link. See module docs for preimage.
    pub entry_hash: BytesN<32>,
    /// Ledger timestamp (seconds) when the entry was recorded.
    pub timestamp: u64,
    /// Ledger sequence when the entry was recorded.
    pub ledger: u32,
    /// Timestamp after which the attestation is considered stale. Must be
    /// strictly greater than the recording time.
    pub expires_at: u64,
    /// Free-form human-readable note (bounded by [`MAX_MEMO_LEN`]).
    pub memo: String,
}

/// Storage keys.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Paused,
    Count,
    LastHash,
    Auditor(Address),
    Entry(u64),
}

/// Contract error codes.
#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    Paused = 4,
    InvalidDataHash = 5,
    InvalidExpiry = 6,
    MemoTooLong = 7,
    EntryNotFound = 8,
    InvalidRange = 9,
    InvalidLimit = 10,
    SelfAudit = 11,
}

#[contract]
pub struct AuditorContract;

#[contractimpl]
impl AuditorContract {
    /// One-time initialization. Sets the admin. Panics if already initialized.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::Count, &0u64);
        let zero = BytesN::from_array(&env, &[0u8; 32]);
        env.storage().instance().set(&DataKey::LastHash, &zero);

        env.events()
            .publish((symbol_short!("init"),), admin);
    }

    /// Return the admin address (panics if uninitialized).
    pub fn get_admin(env: Env) -> Address {
        Self::read_admin(&env)
    }

    /// Grant the auditor role to `who`. Admin-only.
    pub fn grant_auditor(env: Env, who: Address) {
        let admin = Self::read_admin(&env);
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Auditor(who.clone()), &true);
        env.events()
            .publish((symbol_short!("granted"), admin), who);
    }

    /// Revoke the auditor role from `who`. Admin-only.
    ///
    /// Emits a `revoked` event (issue #15).
    pub fn revoke_auditor(env: Env, who: Address) {
        let admin = Self::read_admin(&env);
        admin.require_auth();
        env.storage().persistent().remove(&DataKey::Auditor(who.clone()));
        // Issue #15: emit an event on role revocation so off-chain indexers can
        // react to permission changes.
        env.events()
            .publish((symbol_short!("revoked"), admin), who);
    }

    /// Whether `who` currently holds the auditor role.
    pub fn is_auditor(env: Env, who: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Auditor(who))
            .unwrap_or(false)
    }

    /// Pause recording. Admin-only (issue #10).
    pub fn pause(env: Env) {
        let admin = Self::read_admin(&env);
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((symbol_short!("paused"),), admin);
    }

    /// Resume recording. Admin-only (issue #10).
    pub fn unpause(env: Env) {
        let admin = Self::read_admin(&env);
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((symbol_short!("unpaused"),), admin);
    }

    /// Whether recording is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Append a new audit entry. Returns its id.
    ///
    /// Requires `auditor` to hold the auditor role and to have authorized the
    /// call. Rejects when paused. All input validation is consolidated in
    /// [`Self::validate_record`] (issue #14).
    pub fn record(
        env: Env,
        auditor: Address,
        subject: Address,
        category: Category,
        data_hash: BytesN<32>,
        memo: String,
        expires_at: u64,
    ) -> u64 {
        auditor.require_auth();

        Self::require_initialized(&env);
        Self::require_not_paused(&env);

        if !Self::is_auditor(env.clone(), auditor.clone()) {
            panic_with_error!(&env, Error::NotAuthorized);
        }

        let now = env.ledger().timestamp();
        Self::validate_record(&env, &auditor, &subject, &data_hash, &memo, expires_at, now);

        let id: u64 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let prev_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::LastHash)
            .unwrap_or_else(|| BytesN::from_array(&env, &[0u8; 32]));

        let entry_hash = Self::compute_hash(
            &env, id, &auditor, &subject, category, &data_hash, &prev_hash, now,
        );

        let entry = AuditEntry {
            id,
            auditor: auditor.clone(),
            subject,
            category,
            data_hash,
            prev_hash,
            entry_hash: entry_hash.clone(),
            timestamp: now,
            ledger: env.ledger().sequence(),
            expires_at,
            memo,
        };

        env.storage().persistent().set(&DataKey::Entry(id), &entry);
        env.storage().instance().set(&DataKey::Count, &(id + 1));
        env.storage().instance().set(&DataKey::LastHash, &entry_hash);

        env.events()
            .publish((symbol_short!("record"), auditor), (id, entry_hash));

        id
    }

    /// Fetch a single entry by id.
    pub fn get_entry(env: Env, id: u64) -> AuditEntry {
        env.storage()
            .persistent()
            .get(&DataKey::Entry(id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::EntryNotFound))
    }

    /// Total number of entries recorded.
    pub fn get_entry_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }

    /// The most recent entry hash (chain head); 32 zero bytes if empty.
    pub fn get_last_hash(env: Env) -> BytesN<32> {
        env.storage()
            .instance()
            .get(&DataKey::LastHash)
            .unwrap_or_else(|| BytesN::from_array(&env, &[0u8; 32]))
    }

    /// Return up to `limit` entries starting at `start` (inclusive).
    pub fn get_entries(env: Env, start: u64, limit: u32) -> Vec<AuditEntry> {
        if limit == 0 || limit > MAX_PAGE_LIMIT {
            panic_with_error!(&env, Error::InvalidLimit);
        }
        let count: u64 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let mut out = Vec::new(&env);
        if start >= count {
            return out;
        }
        let end = core::cmp::min(start + limit as u64, count);
        let mut i = start;
        while i < end {
            if let Some(e) = env.storage().persistent().get(&DataKey::Entry(i)) {
                out.push_back(e);
            }
            i += 1;
        }
        out
    }

    /// Recompute and verify the hash chain for entries `[from, to]` inclusive.
    ///
    /// Returns `true` iff every entry's stored `entry_hash` matches the hash
    /// recomputed from its fields, and each entry's `prev_hash` equals the
    /// previous entry's `entry_hash`. Any tampering makes this return `false`.
    pub fn verify_chain(env: Env, from: u64, to: u64) -> bool {
        let count: u64 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        if count == 0 || from > to || to >= count {
            panic_with_error!(&env, Error::InvalidRange);
        }

        let mut i = from;
        while i <= to {
            let entry: AuditEntry = match env.storage().persistent().get(&DataKey::Entry(i)) {
                Some(e) => e,
                None => return false,
            };

            // 1. Recompute this entry's hash from its fields.
            let recomputed = Self::compute_hash(
                &env,
                entry.id,
                &entry.auditor,
                &entry.subject,
                entry.category,
                &entry.data_hash,
                &entry.prev_hash,
                entry.timestamp,
            );
            if recomputed != entry.entry_hash {
                return false;
            }

            // 2. Confirm linkage to the previous entry.
            if entry.id == 0 {
                let zero = BytesN::from_array(&env, &[0u8; 32]);
                if entry.prev_hash != zero {
                    return false;
                }
            } else {
                let prev: AuditEntry = match env
                    .storage()
                    .persistent()
                    .get(&DataKey::Entry(entry.id - 1))
                {
                    Some(e) => e,
                    None => return false,
                };
                if entry.prev_hash != prev.entry_hash {
                    return false;
                }
            }

            i += 1;
        }
        true
    }

    // ----- internal helpers -----

    fn read_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
    }

    fn require_initialized(env: &Env) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(env, Error::NotInitialized);
        }
    }

    fn require_not_paused(env: &Env) {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            panic_with_error!(env, Error::Paused);
        }
    }

    /// Consolidated input validation for [`record`] (issue #14).
    ///
    /// Covers the zero-check theme of issue #11 (reject an all-zero
    /// `data_hash`) and the deadline off-by-one of issues #12/#13 (`expires_at`
    /// must be *strictly* in the future — an entry expiring exactly now is
    /// already stale, so `now >= expires_at` is rejected).
    fn validate_record(
        env: &Env,
        auditor: &Address,
        subject: &Address,
        data_hash: &BytesN<32>,
        memo: &String,
        expires_at: u64,
        now: u64,
    ) {
        // #11: a zero data hash carries no attestation value — reject it.
        let zero = BytesN::from_array(env, &[0u8; 32]);
        if data_hash == &zero {
            panic_with_error!(env, Error::InvalidDataHash);
        }
        // An auditor recording about itself is not a meaningful attestation.
        if auditor == subject {
            panic_with_error!(env, Error::SelfAudit);
        }
        // #12/#13: strict future boundary. `<=` here would be the off-by-one:
        // an entry whose expiry equals the current ledger time is already
        // expired and must be rejected.
        if expires_at <= now {
            panic_with_error!(env, Error::InvalidExpiry);
        }
        if memo.len() > MAX_MEMO_LEN {
            panic_with_error!(env, Error::MemoTooLong);
        }
    }

    /// Compute the chain hash for an entry. See module docs for the preimage.
    fn compute_hash(
        env: &Env,
        id: u64,
        auditor: &Address,
        subject: &Address,
        category: Category,
        data_hash: &BytesN<32>,
        prev_hash: &BytesN<32>,
        timestamp: u64,
    ) -> BytesN<32> {
        let mut buf = Bytes::new(env);
        buf.extend_from_array(&id.to_be_bytes());
        buf.append(&auditor.clone().to_xdr(env));
        buf.append(&subject.clone().to_xdr(env));
        buf.extend_from_array(&(category as u32).to_be_bytes());
        buf.extend_from_array(&data_hash.to_array());
        buf.extend_from_array(&prev_hash.to_array());
        buf.extend_from_array(&timestamp.to_be_bytes());
        env.crypto().sha256(&buf).into()
    }
}

#[cfg(test)]
mod test;
