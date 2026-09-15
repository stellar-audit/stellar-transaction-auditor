# Deployment Guide

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 20+
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) (`stellar` command)
- A funded testnet account (use [Friendbot](https://friendbot.stellar.org))

## 1. Build & Deploy the Smart Contract

```bash
cd contracts

# Build the contract
stellar contract build

# Deploy to testnet
stellar contract deploy \
  --network testnet \
  --source <YOUR_SECRET_KEY> \
  --wasm target/wasm32v1-none/release/stellar_transaction_auditor_contract.wasm

# Initialize (set admin address)
stellar contract invoke \
  --network testnet \
  --source <YOUR_SECRET_KEY> \
  --id <CONTRACT_ID> \
  -- initialize --admin <ADMIN_PUBLIC_KEY>
```

## 2. Run the Backend API

```bash
cd backend

# Set environment variables
export CONTRACT_ID=<CONTRACT_ID>
export PORT=3000

# Build and run
cargo run --release
```

The API will be available at `http://localhost:3000`.

Verify with:
```bash
curl http://localhost:3000/health
curl http://localhost:3000/api/v1/status
```

## 3. Run the Frontend

```bash
cd frontend

# Install dependencies
npm install

# Start dev server (proxies /api to backend at :3000)
npm run dev
```

The app will be available at `http://localhost:5173`.

## 4. Production Build

### Frontend
```bash
cd frontend
npm run build
# Output in frontend/dist/ — serve with any static file server
```

### Backend
```bash
cd backend
cargo build --release
# Binary at backend/target/release/stellar-transaction-auditor-backend
```

## 5. Docker (Optional)

### Backend Dockerfile
```dockerfile
FROM rust:slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/stellar-transaction-auditor-backend /usr/local/bin/
EXPOSE 3000
CMD ["stellar-transaction-auditor-backend"]
```

### Frontend Dockerfile
```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
```

## Environment Variables Reference

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3000` | Backend listen port |
| `CONTRACT_ID` | (empty) | Deployed Soroban contract ID |
| `SOROBAN_RPC_URL` | `https://soroban-testnet.stellar.org:443` | Soroban RPC endpoint |
| `HORIZON_URL` | `https://horizon-testnet.stellar.org` | Horizon server URL |
| `NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` | Network passphrase |
| `NETWORK_NAME` | `TESTNET` | Network display name |
| `SKIP_SEED` | (unset) | Set to `1` to skip demo data |
