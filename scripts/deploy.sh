#!/usr/bin/env bash
set -euo pipefail

# Deploy the Stellar Transaction Auditor contract to testnet.
# Usage: ./scripts/deploy.sh <SECRET_KEY>
# Requires: stellar CLI (cargo install stellar-cli)

KEY="${1:?Usage: $0 <secret_key>}"

echo "Building contract..."
cd contracts
stellar contract build

WASM=$(ls target/wasm32v1-none/release/stellar_transaction_auditor_contract.wasm 2>/dev/null || ls target/wasm32v1-none/release/*.wasm 2>/dev/null | head -1)
if [ -z "$WASM" ]; then
  echo "ERROR: No WASM found. Run 'stellar contract build' first."
  exit 1
fi
echo "WASM: $WASM"

echo "Deploying to testnet..."
CONTRACT_ID=$(stellar contract deploy \
  --network testnet \
  --source "$KEY" \
  --wasm "$WASM" \
  --json | jq -r '.contract_id')

echo "Contract deployed: $CONTRACT_ID"

echo "Initializing with admin = source account..."
ADMIN=$(stellar keys address "$KEY")
stellar contract invoke \
  --network testnet \
  --source "$KEY" \
  --id "$CONTRACT_ID" \
  -- initialize --admin "$ADMIN"

echo ""
echo "=========================================="
echo "Deployment complete!"
echo "  Contract ID: $CONTRACT_ID"
echo "  Admin:       $ADMIN"
echo ""
echo "  Set these env vars for the backend:"
echo "  export CONTRACT_ID=$CONTRACT_ID"
echo "=========================================="
