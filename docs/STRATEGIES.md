# Strategy Adapter Guide

## Overview
Adapters normalize interactions with external Stellar DeFi protocols under a unified
interface so `strategy_router` can deploy vault funds to whichever one is currently active
without knowing anything protocol-specific. `contracts/adapters/blend` is the first real
implementation of this; `contracts/adapters/phoenix` is still an unimplemented placeholder.

## Adapter Interface — as actually implemented

This is `strategy_router`'s own `StrategyAdapter` trait (see
`contracts/strategy_router/src/lib.rs`), not an aspirational spec — every adapter needs to
expose exactly this:

```rust
pub trait StrategyAdapter {
    fn deposit(env: Env, caller: Address, amount: i128);
    fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128;
    fn total_value(env: Env) -> i128;
}
```

- **`deposit`**: the caller (the router) must have already transferred `amount` of the
  configured asset into the adapter's own balance before calling — adapters move funds via
  plain token transfers, not allowances, matching how Blend's own `submit()` works.
- **`withdraw`**: sends the withdrawn amount directly to `to`, skipping an extra hop back
  through the router. Returns the actual amount received, which may be less than requested
  if the position's real value is smaller than `amount`.
- **`total_value`**: read-only. Must reflect the position's real current worth *live* —
  including any interest/yield accrued since the last deposit or withdrawal — not a
  snapshot taken at deposit time. This is what makes yield actually reach vault
  depositors: the vault's own `total_assets()` calls this on every share-price
  calculation.

Every adapter also needs an `initialize(admin: Address, controller: Address, ...)` — two
separate addresses, not one. `admin` is a human deploy-time identity (needed so a real
keypair can authorize the call from the CLI); `controller` is the strategy_router's own
contract address, which `deposit`/`withdraw` are gated on. These have to be different
addresses: a contract can only self-authorize `require_auth()` when it's the one directly
invoking, which a human deployer calling `initialize()` from the CLI never is.

## `adapter-blend` — the real implementation

Wraps a deployed [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2)
lending pool. Uses raw `invoke_contract` with locally-defined `#[contracttype]` structs
matching Blend's real on-chain types (`Request`, `Positions`, `Reserve`, ...), verified
directly against blend-contracts-v2's source — not the `blend-contract-sdk` crate, which
would tie this workspace's build to Blend's own release/version cadence for no benefit
here, matching the pattern this workspace already uses for stellar-zkident's
credential_verifier.

Only ever supplies via `RequestType::Supply`, never `SupplyCollateral` — this adapter
never borrows, so exposing the position as collateral would add real liquidation risk for
zero benefit. `total_value()` reads the pool's real `get_reserve()` (for the current
`b_rate`) and `get_positions()` (for this adapter's own share balance), and replicates
Blend's own `to_asset_from_b_token` conversion math — that conversion is a private impl
method on Blend's side, not a callable entrypoint, so this adapter does the same
fixed-point division itself from data the pool already provides.

## What a future `adapter-phoenix` (or any other adapter) needs

Whatever set of contracts it wraps, it needs to implement the same three-function
interface above, with the same `total_value()` guarantee: a live, protocol-native
computation of current worth, not a cached number from whenever funds were last moved.

## Why `adapter-phoenix` specifically isn't a Blend-style port

Checked directly against [phoenix-contracts](https://github.com/Phoenix-Protocol-Group/phoenix-contracts):
Phoenix's `pool` contract's `provide_liquidity()` takes both `desired_a` and `desired_b` —
there's no single-asset `Supply`-equivalent the way Blend has. Its real yield mechanism is
liquidity provision (earning trading fees) plus staking the resulting LP shares in a
separate `stake` contract for additional reward emissions (`stake` bonds `lp_token`
specifically, not any raw asset). A real adapter would need to: swap roughly half of every
deposit into a paired asset, call `provide_liquidity(..., auto_stake: true)`, and on
withdrawal reverse that (`withdraw_liquidity` with `auto_unstake`, then swap the non-native
portion back). `total_value()` would need `query_share()` to convert staked LP shares back
into both underlying assets, plus a price reference to express the paired asset's amount
in terms of the vault's own asset — itself a real oracle-security question (a naive
`simulate_swap()`-based spot price is manipulable).

None of this is infeasible — it's a real, buildable integration — but it changes what
"depositing in this vault" exposes a user to: real impermanent-loss and slippage risk on
top of (or instead of) lending-style interest. That's a product decision, not an
implementation detail, which is why this wasn't built silently alongside the Blend
adapter. See the main README's Current Status for where that decision currently stands.
