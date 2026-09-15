# 🔐 Stellar Transaction Auditor

A full-stack Web3 application that provides a **tamper-evident, hash-chained
audit log** for activity on the Stellar blockchain. Built with Soroban smart
contracts, an Axum REST API, and a React frontend.

![License](https://img.shields.io/badge/license-MIT-blue)
![Network](https://img.shields.io/badge/network-Stellar%20Testnet-7d4cdb)
![Rust](https://img.shields.io/badge/Rust-1.75+-dea584)
![React](https://img.shields.io/badge/React-18-61dafb)

## What It Does

Authorized auditors record attestations about on-chain activity (payments,
contract calls, compliance checks, security observations). Every entry is
**cryptographically hash-chained** to the previous one, creating an append-only
log where any historical tampering is immediately detectable.

### Key Features

- **Hash-chained audit log** — SHA-256 chaining ensures tamper-evidence
- **Role-based access** — Admin manages auditor roles; only auditors can record
- **Chain verification** — Recompute and verify the full hash chain on-demand
- **Pause/unpause** — Admin can halt recording during investigations
- **Pagination** — Efficient entry retrieval (up to 100 per page)
- **Input validation** — Rejects zero hashes, self-audits, past-due expiry, oversized memos
- **Event emission** — All state changes emit Soroban events for off-chain indexers
- **Dark mode** — Persisted theme toggle (issue #1)
- **Mobile responsive** — Works on phones and tablets (issue #2)
- **Health check** — REST endpoint for monitoring (issue #8)
- **Comprehensive tests** — 13 contract tests + 10 API integration tests

## Project Structure

```
stellar-transaction-auditor/
├── contracts/          # Soroban smart contract (Rust)
│   ├── src/
│   │   ├── lib.rs      # Contract logic: record, verify, roles, pause
│   │   └── test.rs     # 13 unit tests
│   └── Cargo.toml
├── backend/            # REST API server (Rust/Axum)
│   ├── src/
│   │   ├── lib.rs      # Library exports
│   │   ├── main.rs     # Server entry point
│   │   ├── state.rs    # Config & shared state
│   │   ├── error.rs    # Unified error handling
│   │   ├── models.rs   # Serde DTOs
│   │   ├── routes/     # Health & audit endpoints
│   │   └── services/   # Hash-chained audit store
│   ├── tests/          # 10 API integration tests
│   └── Cargo.toml
├── frontend/           # React/TypeScript frontend
│   ├── src/
│   │   ├── App.tsx     # Full audit app (dashboard, log, record, verify, admin)
│   │   ├── api.ts      # Backend API client
│   │   ├── wallet.ts   # Freighter wallet integration
│   │   ├── stellar.ts  # Stellar SDK helpers
│   │   └── styles.css  # Dark mode + responsive styles
│   └── package.json
├── docs/               # Documentation
│   ├── ARCHITECTURE.md # System architecture & design decisions
│   ├── API.md          # REST API reference
│   └── DEPLOYMENT.md   # Deployment guide
└── .github/workflows/  # CI/CD
    └── ci.yml          # Contract + backend + frontend CI
```

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [Freighter](https://www.freighter.app/) browser extension (for wallet features)

### Quick Start (Development)

1. **Start the backend:**
   ```bash
   cd backend && cargo run
   # API at http://localhost:3000 with seeded demo data
   ```

2. **Start the frontend:**
   ```bash
   cd frontend && npm install && npm run dev
   # App at http://localhost:5173
   ```

3. **Open the app** and explore the dashboard, audit log, verification, and admin panel.

### Deploy the Smart Contract

```bash
cd contracts
stellar contract build
stellar contract deploy --network testnet --source <KEY> --wasm target/wasm32v1-none/release/*.wasm
stellar contract invoke --network testnet --id <CONTRACT_ID> -- initialize --admin <ADMIN_ADDR>
```

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for full instructions.

## Testing

```bash
# Smart contract tests (13 tests)
cd contracts && cargo test

# Backend API tests (10 tests)
cd backend && cargo test

# Frontend type check & build
cd frontend && npx tsc --noEmit && npm run build
```

## API Endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/health` | Liveness probe |
| GET | `/api/v1/status` | Service status |
| GET | `/api/v1/entries` | Paginated entries |
| GET | `/api/v1/entries/:id` | Single entry |
| GET | `/api/v1/entries/count` | Entry count |
| POST | `/api/v1/entries` | Record entry |
| GET | `/api/v1/verify` | Verify chain |
| GET | `/api/v1/admin` | Admin address |
| GET | `/api/v1/auditors/:addr` | Check auditor |
| POST | `/api/v1/admin/grant` | Grant role |
| POST | `/api/v1/admin/revoke` | Revoke role |
| POST | `/api/v1/admin/pause` | Pause recording |
| POST | `/api/v1/admin/unpause` | Resume recording |

See [docs/API.md](docs/API.md) for full reference.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — System design, data flow, security model
- [API Reference](docs/API.md) — All endpoints with examples
- [Deployment](docs/DEPLOYMENT.md) — Contract deployment & production setup

## License

MIT
