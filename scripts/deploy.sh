#!/usr/bin/env bash
# Deploys the vault contract to Stellar testnet, initialized against testnet's real native
# XLM Stellar Asset Contract, and records the resulting contract ID in
# deployments/<network>.json. Requires the `stellar` CLI already installed and on PATH.
#
# contracts/adapters/blend, contracts/adapters/phoenix, and contracts/strategy_router are
# NOT deployed here — each is still a bare `version() -> 1` stub with no real Blend/Phoenix
# integration (see the README's Current Status section). Deploying stub contracts and
# presenting them as a working yield strategy would be exactly the kind of fabrication this
# project was rejected for the first time around, so only the real, tested vault ships.
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$REPO_ROOT/deployments"
OUT_FILE="$OUT_DIR/$NETWORK.json"
mkdir -p "$OUT_DIR"

echo "Deploying soroban-yield-vault to Stellar $NETWORK..."

if ! stellar keys address deployer >/dev/null 2>&1; then
  echo "Generating deployer key..."
  stellar keys generate deployer
fi
stellar keys fund deployer --network "$NETWORK" || true
DEPLOYER_ADDR=$(stellar keys address deployer)

cd "$REPO_ROOT"
cargo build --release --target wasm32v1-none -p vault

echo "Resolving testnet native XLM Stellar Asset Contract..."
NATIVE_TOKEN_ID=$(stellar contract id asset --asset native --network "$NETWORK")

echo "Deploying vault..."
VAULT_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/vault.wasm \
  --source deployer \
  --network "$NETWORK")
stellar contract invoke --id "$VAULT_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --token "$NATIVE_TOKEN_ID"

cat > "$OUT_FILE" <<EOF
{
  "network": "$NETWORK",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployer": "$DEPLOYER_ADDR",
  "native_token": "$NATIVE_TOKEN_ID",
  "contracts": {
    "vault": "$VAULT_ID"
  },
  "notes": {
    "adapters_and_strategy_router": "Not deployed — still stubs, see README Current Status."
  }
}
EOF

echo ""
echo "Deployed to $NETWORK — recorded in $OUT_FILE"
cat "$OUT_FILE"
