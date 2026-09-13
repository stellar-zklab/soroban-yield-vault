#!/usr/bin/env bash
# Deploys adapter-soroswap and registers it as an ADDITIONAL strategy on the existing
# strategy-router — additive, unlike deploy_blend_strategy.sh, since strategy_router already
# supports multiple concurrent strategies and adapter-soroswap needs no changes to
# vault/strategy_router to slot in. Requires the stellar CLI, jq, and the SAME deployer
# identity already used for the existing deployment record.
#
# Targets a real, live Soroswap AMM pool on testnet (https://github.com/soroswap/core) —
# NOT Phoenix (see the open Phoenix adapter issue for why that one's still a placeholder:
# no currently-live testnet deployment could be found for it). Router/pair/USDC addresses
# below were independently verified live against testnet before this adapter was written:
# router responded with its real interface via `stellar contract invoke --help`, the pair
# genuinely exists (factory.get_pair returned it) and has real non-trivial liquidity
# (router_get_amounts_out for 10 XLM quoted ~0.978 real USDC, not zero/an error).
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
SOROSWAP_ROUTER_ID="${SOROSWAP_ROUTER_ID:-CCJUD55AG6W5HAI5LRVNKAE5WDP5XGZBUDS5WNTIVDU7O264UZZE7BRD}"
SOROSWAP_PAIR_ID="${SOROSWAP_PAIR_ID:-CDVAIOYHCD4RUSLQNVFI7RIZBFT2JZMJWM4RTOLQZQXL4QAVXU5RFKDB}"
TESTNET_USDC_ID="${TESTNET_USDC_ID:-CB3TLW74NBIOT3BUWOZ3TUM6RFDF6A4GVIRUQRQZABG5KPOUL4JJOV2F}"
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

ROUTER_ID=$(jq -r '.contracts.strategy_router' "$DEPLOYMENTS_FILE")
NATIVE_TOKEN_ID=$(jq -r '.native_token' "$DEPLOYMENTS_FILE")
if [ "$ROUTER_ID" = "null" ] || [ "$NATIVE_TOKEN_ID" = "null" ]; then
  echo "Could not find contracts.strategy_router / native_token in $DEPLOYMENTS_FILE." >&2
  exit 1
fi

if ! stellar keys address deployer >/dev/null 2>&1; then
  echo "No 'deployer' identity found. This script expects the SAME deployer used for the existing deployment record." >&2
  exit 1
fi
stellar keys fund deployer --network "$NETWORK" || true
DEPLOYER_ADDR=$(stellar keys address deployer)

cd "$REPO_ROOT"
cargo build --release --target wasm32v1-none -p adapter-soroswap
WASM_DIR="target/wasm32v1-none/release"

echo "Deploying adapter-soroswap, controller = existing strategy_router ($ROUTER_ID)..."
ADAPTER_ID=$(stellar contract deploy --wasm "$WASM_DIR/adapter_soroswap.wasm" --source deployer --network "$NETWORK")
stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" \
  -- initialize --admin "$DEPLOYER_ADDR" --controller "$ROUTER_ID" \
     --router "$SOROSWAP_ROUTER_ID" --pair "$SOROSWAP_PAIR_ID" \
     --asset "$NATIVE_TOKEN_ID" --paired_asset "$TESTNET_USDC_ID"

echo "Verifying total_value() against the real pool before registering (should read 0 — no funds deployed yet)..."
stellar contract invoke --id "$ADAPTER_ID" --source deployer --network "$NETWORK" -- total_value

echo "Registering the Soroswap adapter as an additional strategy on the existing router..."
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- add_strategy --admin "$DEPLOYER_ADDR" --strategy "$ADAPTER_ID"
stellar contract invoke --id "$ROUTER_ID" --source deployer --network "$NETWORK" \
  -- set_max_debt_for_strategy --admin "$DEPLOYER_ADDR" --strategy "$ADAPTER_ID" --max_debt "${MAX_DEBT_STROOPS:-50000000000}"

TMP_FILE="$(mktemp)"
jq \
  --arg adapter "$ADAPTER_ID" \
  --arg router "$SOROSWAP_ROUTER_ID" \
  --arg pair "$SOROSWAP_PAIR_ID" \
  --arg usdc "$TESTNET_USDC_ID" \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '.contracts.adapter_soroswap = $adapter
   | .notes.adapter_soroswap = ("Deployed " + $ts + ", targets a real live Soroswap router (" + $router + ") and pair (" + $pair + ") on testnet, LP-ing native XLM against real testnet USDC (" + $usdc + "). Registered as an ADDITIONAL strategy on the existing strategy_router (multi-strategy, alongside adapter_blend) - vault and router were not redeployed. Earns real trading fees as a liquidity provider, a genuinely different yield source from adapter_blend'"'"'s lending supply.")' \
  "$DEPLOYMENTS_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$DEPLOYMENTS_FILE"

echo ""
echo "Done. adapter_soroswap: $ADAPTER_ID"
echo "      registered on strategy_router: $ROUTER_ID"
echo ""
echo "NOTE: registering a strategy doesn't move funds to it. To actually exercise deposit():"
echo "  stellar contract invoke --id $ROUTER_ID --source deployer --network $NETWORK -- update_debt --admin $DEPLOYER_ADDR --strategy $ADAPTER_ID --desired_debt <amount>"
echo "(requires the vault/router to already hold real idle XLM to allocate)"
echo ""
echo "Next steps (not done by this script):"
echo "  - review: git -C \"$REPO_ROOT\" diff"
echo "  - update README.md's Current Status section"
echo "  - git add -A && git commit"
