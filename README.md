# stellar-transaction-auditor

A full-stack Web3 application built on the Stellar blockchain using Soroban smart contracts.

## Project Structure
```
stellar-transaction-auditor/
├── contracts/          # Soroban smart contracts (Rust)
├── backend/             # API server (Rust/Axum)
├── frontend/            # React/TypeScript frontend
└── docs/               # Documentation
```

## Features
- Smart contracts on Stellar/Soroban
- REST API backend with authentication
- React frontend with wallet integration
- Comprehensive testing suite

## Getting Started
```bash
# Install dependencies
cd contracts && cargo build
cd backend && cargo build
cd frontend && npm install

# Run the application
cd backend && cargo run
cd frontend && npm run dev
```

## License
MIT
