# Architecture

## Overview

The Stellar Transaction Auditor is a full-stack Web3 application that provides
a **tamper-evident, hash-chained audit log** for activity on the Stellar
blockchain. It enables authorized auditors to record attestations about
transactions, contract calls, and compliance/security observations, with
cryptographic guarantees that any historical tampering is detectable.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (React)                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│
│  │Dashboard │ │Audit Log │ │ Record   │ │ Verify   │ │ Admin  ││
│  │  Stats   │ │ + Search │ │  Form    │ │  Chain   │ │ Panel  ││
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └───┬────┘│
│       └────────────┴────────────┴────────────┴────────────┘     │
│                              │ HTTP / Fetch                     │
└──────────────────────────────┼──────────────────────────────────┘
                               │
┌──────────────────────────────┼──────────────────────────────────┐
│                     Backend API (Axum)                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Routes: /health, /entries, /verify, /admin, /auditors   │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │  AuditStore: in-memory hash-chained log (mirrors contract)│  │
│  ├──────────────────────────────────────────────────────────┤   │
│  │  Config: env-based (CONTRACT_ID, RPC_URL, HORIZON_URL)   │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────┼──────────────────────────────────┘
                               │ (production: Soroban RPC)
┌──────────────────────────────┼──────────────────────────────────┐
│                Stellar Testnet (Soroban)                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  AuditorContract (Soroban smart contract)                │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────────────┐ │   │
│  │  │initialize│ │ record  │ │verify   │ │ grant/revoke   │ │   │
│  │  │ (admin)  │ │ (entry) │ │ _chain  │ │ _auditor       │ │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Smart Contract (`contracts/`)

**Language:** Rust + Soroban SDK 22.0  
**Type:** Soroban smart contract (compiled to WASM)

The core of the system — a tamper-evident audit log stored on-chain.

**Key design decisions:**
- **Hash-chaining:** Each entry stores `prev_hash` (the hash of the previous entry) and `entry_hash` (SHA-256 of all entry fields including `prev_hash`). This creates an append-only chain where modifying any historical entry breaks all subsequent hashes.
- **Role-based access:** Admin (set once at initialization) manages auditor roles. Only auditors can record entries.
- **Pause/unpause:** Admin can temporarily halt recording (e.g., during investigations).
- **Input validation:** Rejects zero data hashes, self-audits, past-due expiry times, and oversized memos.
- **Pagination:** `get_entries(start, limit)` for efficient retrieval (max 100 per page).

**Hash preimage:**
```
entry_hash = SHA-256(id ‖ auditor ‖ subject ‖ category ‖ data_hash ‖ prev_hash ‖ timestamp)
```

**Entry struct:**
```rust
pub struct AuditEntry {
    id: u64,           // sequential identifier
    auditor: Address,  // who recorded it
    subject: Address,  // who/what it's about
    category: Category,// payment, contract_call, compliance, security, other
    data_hash: BytesN<32>, // hash of off-chain payload
    prev_hash: BytesN<32>, // chain link to previous entry
    entry_hash: BytesN<32>,// this entry's hash
    timestamp: u64,    // ledger timestamp
    ledger: u32,       // ledger sequence
    expires_at: u64,   // attestation expiry
    memo: String,      // human-readable note (max 512 bytes)
}
```

**Contract functions:**
| Function | Access | Description |
|---|---|---|
| `initialize(admin)` | Public (once) | Set admin address |
| `record(...)` | Auditor | Append a new audit entry |
| `get_entry(id)` | Public | Fetch single entry |
| `get_entries(start, limit)` | Public | Paginated entry list |
| `get_entry_count()` | Public | Total entries |
| `get_last_hash()` | Public | Chain head hash |
| `verify_chain(from, to)` | Public | Verify hash chain integrity |
| `grant_auditor(who)` | Admin | Grant auditor role |
| `revoke_auditor(who)` | Admin | Revoke auditor role |
| `is_auditor(who)` | Public | Check auditor role |
| `pause()` / `unpause()` | Admin | Toggle recording |
| `is_paused()` | Public | Check pause status |

### 2. Backend API (`backend/`)

**Language:** Rust + Axum 0.7  
**Port:** 3000 (configurable via `PORT` env var)

REST API that serves audit log data and provides operational endpoints.

**Endpoints:**
| Method | Path | Description |
|---|---|---|
| GET | `/health` | Liveness probe (issue #8) |
| GET | `/api/v1/status` | Detailed service status |
| GET | `/api/v1/entries` | Paginated entries (`?start=0&limit=20`) |
| GET | `/api/v1/entries/:id` | Single entry by ID |
| GET | `/api/v1/entries/count` | Total entry count |
| POST | `/api/v1/entries` | Record a new entry |
| GET | `/api/v1/verify` | Verify chain (`?from=0&to=4`) |
| GET | `/api/v1/admin` | Get admin address |
| GET | `/api/v1/auditors/:addr` | Check auditor role |
| POST | `/api/v1/admin/grant` | Grant auditor role |
| POST | `/api/v1/admin/revoke` | Revoke auditor role |
| POST | `/api/v1/admin/pause` | Pause recording |
| POST | `/api/v1/admin/unpause` | Resume recording |
| GET | `/api/v1/admin/paused` | Check pause status |

**Architecture:**
- `state.rs` — Config from env vars, shared AppState
- `error.rs` — Unified ApiError → HTTP status mapping
- `models.rs` — Serde-compatible DTOs
- `services/audit_store.rs` — In-memory hash-chained store mirroring contract logic
- `routes/health.rs` — Health & status endpoints
- `routes/audit.rs` — Audit CRUD & verification endpoints

**Environment variables:**
| Var | Default | Description |
|---|---|---|
| `PORT` | `3000` | Listen port |
| `CONTRACT_ID` | (empty) | Soroban contract ID |
| `SOROBAN_RPC_URL` | `https://soroban-testnet.stellar.org:443` | RPC endpoint |
| `HORIZON_URL` | `https://horizon-testnet.stellar.org` | Horizon URL |
| `NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` | Network passphrase |
| `NETWORK_NAME` | `TESTNET` | Network display name |
| `SKIP_SEED` | (unset) | Set to `1` to skip demo data seeding |

### 3. Frontend (`frontend/`)

**Stack:** React 18 + TypeScript + Vite  
**Port:** 5173 (dev server, proxies API to backend)

A full audit application with 6 views:

1. **Dashboard** — Overview stats (entry count, status, network, uptime) + recent entries
2. **Audit Log** — Paginated, searchable table with entry detail modal
3. **Record** — Form to submit new audit entries
4. **Verify** — Chain integrity verification with visual pass/fail result
5. **Admin** — Grant/revoke auditor roles, pause/unpause recording, check auditor status
6. **Wallet** — Freighter wallet connection, balance, network info

**Features:**
- 🌙 Dark mode (persisted to localStorage) — issue #1
- 📱 Mobile-responsive layout — issue #2
- 🔐 Freighter wallet integration
- ⚡ API proxy to backend via Vite dev server
- 🔔 Toast notifications for user feedback

### 4. Tests

- **Contract:** 13 unit tests covering initialization, roles, recording, validation, pagination, and tamper detection
- **Backend:** 10 integration tests covering all API endpoints
- **Frontend:** TypeScript type checking + build verification

### 5. CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`) runs on every push/PR:
- Contract: `cargo test` + `cargo build --release`
- Backend: `cargo test` + `cargo clippy`
- Frontend: `npm ci` + `tsc --noEmit` + `npm run build`

## Data Flow

1. **Recording:** Auditor calls `record()` on the contract → entry hash-chained → stored on-chain → event emitted
2. **Reading:** API reads entries from contract/storage → returns JSON → frontend displays in table
3. **Verification:** API calls `verify_chain()` → recomputes all hashes → returns boolean → frontend shows pass/fail
4. **Admin:** Admin calls grant/revoke/pause/unpause → contract updates state → events emitted

## Security Model

- **require_auth:** All state-changing calls require Stellar auth from the caller
- **Role-based:** Only auditors can record; only admin can manage roles/pause
- **Immutable log:** Entries are append-only; no update/delete functions exist
- **Tamper-evident:** Hash chaining ensures any modification is detectable via `verify_chain`
- **Input validation:** Zero hashes, self-audits, past-due expiry, and oversized memos are rejected
