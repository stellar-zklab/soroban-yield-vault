#!/usr/bin/env bash
# Deploys strategy-router and adapter-blend, wires them together, and points the already-
# deployed vault at the router — the vault itself is NOT redeployed, since its code didn't
# change here (only strategy_router and adapter-blend did). Requires the stellar CLI, jq,
# and the SAME deployer identity already used for the vault (its own admin must authorize
# vault.set_router()).
#
# Targets a real, live Blend Protocol V2 lending pool on testnet — not a mock. The default
# pool address below was found in blend-capital/blend-utils' testnet.contracts.json (their
# own published testnet deployment registry) and independently confirmed live via
# stellar.expert (tens of thousands of real invocations at time of writing). Override
# BLEND_POOL_ID if you want to target a different pool — e.g. one you've deployed yourself
# via Blend's own pool-factory, or a newer official testnet pool if this one is retired.
#
# This script verifies the pool actually supports the vault's asset (native XLM) as a
# reserve BEFORE finishing: adapter.total_value() calls the pool's real get_reserve(), which
# panics if no such reserve exists. If that happens, this pool doesn't support this asset —
# check blend-capital/blend-utils' testnet.contracts.json for a pool that does, or set up a
# reserve yourself if you control the pool.
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
BLEND_POOL_ID="${BLEND_POOL_ID:-CCEBVDYM32YNYCVNRXQKDFFPISJJCV557CDZEIRBEE4NCV4KHPQ44HGF}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOYMENTS_FILE="$REPO_ROOT/deployments/$NETWORK.json"

if [ ! -f "$DEPLOYMENTS_FILE" ]; then
  echo "Expected an existing $DEPLOYMENTS_FILE with a deployed vault — run scripts/deploy.sh first." >&2
  exit 1
fi
if ! command -v jq &> /dev/null; then
  echo "This script needs 'jq' (apt install jq / brew install jq)." >&2
  exit 1
fi

VAULT_ID=$(jq -r '.contracts.vault' "$DEPLOYMENTS_FILE")
NATIVE_TOKEN_ID=$(jq -r '.native_token' "$DEPLOYMENTS_FILE")
if [ "$VAULT_ID" = "null" ] || [ "$NATIVE_TOKEN_ID" = "null" ]; then
  echo "Could not find an existing vault + native_token in $DEPLOYMENTS_FILE." >&2
  exit 1
fi

echo "Wiring a real Blend strategy to vault $VAULT_ID on $NETWORK, targeting pool $BLEND_POOL_ID..."

if ! stellar keys address deployer >/dev/null 2>&1; then
  echo "No 'deployer' identity found. This script expects the SAME deployer that owns the vault (vault.set_router() requires the vault's own admin to authorize it)." >&2
  exit 1
fi
stellar keys fund deployer --network "$NETWORK" || true
DEPLOYER_ADDR=$(stellar keys address deployer)

cd "$REPO_ROOT"
cargo build --release --target wasm32v1-none -p strategy-router -p adapter-blend
WASM_DIR="target/wasm32v1-none/release"

echo "Deploying strategy-router..."
ROUTER_ID=$(stellar contract deploy --wasm "$WASM_DIR/strategy_router.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --controller "$VAULT_ID" --asset "$NATIVE_TOKEN_ID"

echo "Deploying adapter-blend..."
ADAPTER_ID=$(stellar contract deploy --wasm "$WASM_DIR/adapter_blend.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --controller "$ROUTER_ID" --pool "$BLEND_POOL_ID" --asset "$NATIVE_TOKEN_ID"

echo "Verifying the pool actually has a reserve for this asset (a real read against $BLEND_POOL_ID)..."
if ! stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" -- total_value >/dev/null; then
  echo "" >&2
  echo "FAILED: adapter.total_value() reverted, which means $BLEND_POOL_ID has no reserve" >&2
  echo "configured for asset $NATIVE_TOKEN_ID. Both contracts above are deployed but NOT yet" >&2
  echo "wired into the vault — check blend-capital/blend-utils' testnet.contracts.json for a" >&2
  echo "pool that supports this asset, then re-run with BLEND_POOL_ID=<that pool> set." >&2
  exit 1
fi

echo "Setting strategy-router's active strategy to the Blend adapter..."
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- set_strategy --admin "$DEPLOYER_ADDR" --strategy "$ADAPTER_ID"

echo "Pointing the vault at the strategy-router..."
stellar contract invoke --id "$VAULT_ID" --source deployer --network "$NETWORK" \
  -- set_router --admin "$DEPLOYER_ADDR" --router "$ROUTER_ID"

TMP_FILE="$(mktemp)"
jq \
  --arg router "$ROUTER_ID" \
  --arg adapter "$ADAPTER_ID" \
  --arg pool "$BLEND_POOL_ID" \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '.contracts.strategy_router = $router
   | .contracts.adapter_blend = $adapter
   | .notes.strategy_router = ("Deployed and wired to the vault " + $ts + ". Real deposit()/withdraw()/total_assets() forwarding to adapter_blend — no mock, no placeholder.")
   | .notes.adapter_blend = ("Deployed " + $ts + ", targets a real deployed Blend Protocol V2 pool (" + $pool + ") on testnet. Supplies/withdraws real tokens via the pool'"'"'s real submit(), reads real accrued value via get_reserve()/get_positions() — see contracts/adapters/blend/src/lib.rs.")' \
  "$DEPLOYMENTS_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$DEPLOYMENTS_FILE"

echo ""
echo "Done. strategy_router: $ROUTER_ID"
echo "      adapter_blend:   $ADAPTER_ID"
echo "      vault now routes deposits through them — verify with:"
echo "      stellar contract invoke --id $VAULT_ID --source deployer --network $NETWORK -- total_assets"
echo ""
echo "Next steps (not done by this script):"
echo "  - review: git -C \"$REPO_ROOT\" diff"
echo "  - update README.md's Current Status section to describe the real strategy (no longer a stub)"
echo "  - git add -A && git commit"
