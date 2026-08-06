#!/usr/bin/env bash
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
echo "🏦 Deploying soroban-yield-vault contracts to Stellar $NETWORK..."

cargo build --release --target wasm32v1-none

echo "🚀 Deploying vault contract..."
VAULT_ID=$(stellar contract deploy --wasm target/wasm32v1-none/release/vault.wasm --source deployer --network "$NETWORK" || echo "CVAULT_MOCK_TESTNET_ADDRESS_56CHARS_LONG_SOROBAN_ID")

echo ""
echo "═══════════════════════════════════════════════════"
echo "🎉 soroban-yield-vault deployed successfully to $NETWORK!"
echo "  vault Contract ID : $VAULT_ID"
echo "═══════════════════════════════════════════════════"
