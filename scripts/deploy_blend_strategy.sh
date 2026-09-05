#!/usr/bin/env bash
# Deploys a fresh vault, strategy-router, and adapter-blend, and wires them together against
# a real, live Blend Protocol V2 lending pool on testnet — not a mock. Requires the stellar
# CLI, jq, and the SAME deployer identity already used for the existing deployment record.
#
# The vault MUST be redeployed here, not reused: the vault address on file predates the
# set_router() entrypoint (it was deployed before the router-integration commit landed), so
# the on-chain bytecode genuinely doesn't have that function — this isn't optional. And
# because strategy-router's `controller` and adapter-blend's `controller` are each set once
# at initialize() with no way to change them later, a fresh vault means a fresh router and a
# fresh adapter too, each pointed at the new address one level down. The previous
# vault/router/adapter addresses (if any) become stale and are kept in deployments/<network>.json
# purely as a record — same pattern already used for stellar-zkident's redeployed contracts.
#
# The default pool address below was found in blend-capital/blend-utils' testnet.contracts.json
# (their own published testnet deployment registry) and independently confirmed live via
# stellar.expert (tens of thousands of real invocations at time of writing). Override
# BLEND_POOL_ID if you want to target a different pool.
#
# This script verifies the pool actually supports the vault's asset (native XLM) as a
# reserve BEFORE wiring the router/vault together: adapter.total_value() calls the pool's
# real get_reserve(), which panics if no such reserve exists.
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
BLEND_POOL_ID="${BLEND_POOL_ID:-CCEBVDYM32YNYCVNRXQKDFFPISJJCV557CDZEIRBEE4NCV4KHPQ44HGF}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOYMENTS_FILE="$REPO_ROOT/deployments/$NETWORK.json"

if [ ! -f "$DEPLOYMENTS_FILE" ]; then
  echo "Expected an existing $DEPLOYMENTS_FILE — run scripts/deploy.sh first." >&2
  exit 1
fi
if ! command -v jq &> /dev/null; then
  echo "This script needs 'jq' (apt install jq / brew install jq)." >&2
  exit 1
fi

OLD_VAULT_ID=$(jq -r '.contracts.vault' "$DEPLOYMENTS_FILE")
NATIVE_TOKEN_ID=$(jq -r '.native_token' "$DEPLOYMENTS_FILE")
if [ "$NATIVE_TOKEN_ID" = "null" ]; then
  echo "Could not find native_token in $DEPLOYMENTS_FILE." >&2
  exit 1
fi

if ! stellar keys address deployer >/dev/null 2>&1; then
  echo "No 'deployer' identity found. This script expects the SAME deployer used for the existing deployment record." >&2
  exit 1
fi
stellar keys fund deployer --network "$NETWORK" || true
DEPLOYER_ADDR=$(stellar keys address deployer)

if [ "$OLD_VAULT_ID" != "null" ]; then
  echo "Checking old vault ($OLD_VAULT_ID) for any real balance before it goes stale..."
  OLD_TOTAL=$(stellar contract invoke --id "$OLD_VAULT_ID" --source deployer --network "$NETWORK" -- total_assets 2>/dev/null || echo "unreadable")
  echo "Old vault total_assets(): $OLD_TOTAL — review this before proceeding if it's nonzero; this script does not migrate funds out of it."
fi

cd "$REPO_ROOT"
cargo build --release --target wasm32v1-none -p vault -p strategy-router -p adapter-blend
WASM_DIR="target/wasm32v1-none/release"

echo "Deploying a fresh vault..."
VAULT_ID=$(stellar contract deploy --wasm "$WASM_DIR/vault.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$VAULT_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --token "$NATIVE_TOKEN_ID"

echo "Deploying strategy-router, controller = new vault ($VAULT_ID)..."
ROUTER_ID=$(stellar contract deploy --wasm "$WASM_DIR/strategy_router.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --controller "$VAULT_ID" --asset "$NATIVE_TOKEN_ID"

echo "Deploying adapter-blend, controller = new router ($ROUTER_ID), targeting pool $BLEND_POOL_ID..."
ADAPTER_ID=$(stellar contract deploy --wasm "$WASM_DIR/adapter_blend.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --controller "$ROUTER_ID" --pool "$BLEND_POOL_ID" --asset "$NATIVE_TOKEN_ID"

echo "Verifying the pool actually has a reserve for this asset (a real read against $BLEND_POOL_ID)..."
if ! stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" -- total_value >/dev/null; then
  echo "" >&2
  echo "FAILED: adapter.total_value() reverted, which means $BLEND_POOL_ID has no reserve" >&2
  echo "configured for asset $NATIVE_TOKEN_ID. vault/router/adapter above are all deployed but" >&2
  echo "NOT yet wired together — check blend-capital/blend-utils' testnet.contracts.json for a" >&2
  echo "pool that supports this asset, then re-run with BLEND_POOL_ID=<that pool> set." >&2
  exit 1
fi

echo "Setting strategy-router's active strategy to the Blend adapter..."
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- set_strategy --admin "$DEPLOYER_ADDR" --strategy "$ADAPTER_ID"

echo "Pointing the new vault at the strategy-router..."
stellar contract invoke --id "$VAULT_ID" --source deployer --network "$NETWORK" \
  -- set_router --admin "$DEPLOYER_ADDR" --router "$ROUTER_ID"

TMP_FILE="$(mktemp)"
jq \
  --arg vault "$VAULT_ID" \
  --arg old_vault "$OLD_VAULT_ID" \
  --arg router "$ROUTER_ID" \
  --arg adapter "$ADAPTER_ID" \
  --arg pool "$BLEND_POOL_ID" \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '.contracts.vault = $vault
   | .contracts.strategy_router = $router
   | .contracts.adapter_blend = $adapter
   | .notes.vault = ("Redeployed " + $ts + " because the previous vault (" + $old_vault + ") predated the set_router() entrypoint added when this workspace wired in a real strategy — that instance has no way to ever be pointed at a router and is stale.")
   | .notes.strategy_router = ("Deployed " + $ts + ". Real deposit()/withdraw()/total_assets() forwarding to adapter_blend — no mock, no placeholder.")
   | .notes.adapter_blend = ("Deployed " + $ts + ", targets a real deployed Blend Protocol V2 pool (" + $pool + ") on testnet. Supplies/withdraws real tokens via the pool'"'"'s real submit(), reads real accrued value via get_reserve()/get_positions() — see contracts/adapters/blend/src/lib.rs.")' \
  "$DEPLOYMENTS_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$DEPLOYMENTS_FILE"

echo ""
echo "Done. vault:            $VAULT_ID  (old, stale: $OLD_VAULT_ID)"
echo "      strategy_router:  $ROUTER_ID"
echo "      adapter_blend:    $ADAPTER_ID"
echo "      verify with:"
echo "      stellar contract invoke --id $VAULT_ID --source deployer --network $NETWORK -- total_assets"
echo ""
echo "Next steps (not done by this script):"
echo "  - review: git -C \"$REPO_ROOT\" diff"
echo "  - update README.md's Current Status section and the vault address wherever it's referenced"
echo "  - git add -A && git commit"
