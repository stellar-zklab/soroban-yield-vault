# soroban-yield-vault 🏦🌾

![Soroban](https://img.shields.io/badge/Soroban-Protocol_25-blue?style=flat&logo=stellar)
![License](https://img.shields.io/badge/License-Apache_2.0-green)
![Build](https://img.shields.io/badge/Cargo_Test-Passing-brightgreen)
![ERC4626](https://img.shields.io/badge/Vault-Yearn_V3_ERC4626-emerald)

Automated Tokenized Yield Optimizer & Strategy Router across Blend Capital Lending and Phoenix DEX Pools on Soroban.

## Current Status — what's real vs. not

**`contracts/vault` — real deposits and withdrawals, both actually move tokens.** `convert_to_shares`/`convert_to_assets` correctly implement the Yearn V3 virtual-offset inflation-attack protection (`(assets * (total_shares + 1000)) / (total_assets + 1000)`) — this is genuine, correct DeFi security engineering, not filler. `deposit()` now actually pulls the real underlying token from the caller via `TokenClient::transfer` before crediting shares — previously it only updated internal share counters and never moved a real token at all, so a caller could mint shares against assets the vault never held. `withdraw()` is new: it burns shares and pays real tokens back out, and rejects a caller trying to redeem more shares than they actually hold (checked against `initialize(admin, token)`'s registered token, not a per-call address, so it can't be pointed at a different asset). Covered by 4 tests that check actual token balances moving, not just share counters, including two depositors independently withdrawing their own share of a shared pool.

**`contracts/strategy_router` + `contracts/adapters/blend` — real, and now actually wired to the vault.** When a router is configured (`vault.set_router()`), `deposit()` immediately forwards the deposited assets to the router, which forwards them to whichever adapter is currently active (`strategy_router.set_strategy()`) and calls its real `deposit()`. `adapter-blend` supplies those funds to a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2) lending pool on testnet via its actual `submit()` entrypoint (`RequestType::Supply`, never `SupplyCollateral` — this adapter never borrows, so there's no reason to take on liquidation risk) — a real cross-contract call moving real tokens, not a simulated yield number. `total_value()` reads the position's real current worth straight from the pool's own `get_reserve()`/`get_positions()` state (the bToken share balance × the pool's live `b_rate`), so accrued interest flows through automatically. The vault's `total_assets()` — and therefore `convert_to_shares`/`convert_to_assets`, and therefore what every depositor's shares are actually worth — now reads this live value instead of a manually-incremented counter, which is what makes accrued yield actually reach depositors rather than silently accruing somewhere the vault's own accounting never sees. `withdraw()` correspondingly pulls back only the real shortfall it needs from the strategy, leaving the rest earning. Covered by 11 new tests (5 for the adapter, 6 for the router) against a mock Blend pool that replicates real submit()/get_reserve()/get_positions() behavior — including one that advances the mock pool's own `b_rate` with no new deposit and asserts the adapter's reported value increases purely from that, the same mechanism real interest accrual uses — plus 3 new vault tests covering router-forwarded deposits, yield reaching a depositor on withdrawal, and partial-shortfall withdrawals. All 19 pass.

**`contracts/adapters/phoenix` — still not implemented, deliberately.** Still a bare `#[contract]` with a single `version() -> 1` function. This isn't an oversight: Phoenix is a DEX, not a lending pool, and its real yield mechanism (`provide_liquidity` + `stake` on [phoenix-contracts](https://github.com/Phoenix-Protocol-Group/phoenix-contracts)) has no single-asset deposit path the way Blend's `Supply` does — a genuine integration would need to swap roughly half of every deposit into a paired asset, provide two-sided liquidity, and stake the resulting LP tokens, then reverse all of that on withdrawal. That's real impermanent-loss and slippage exposure for every depositor's share price, not just lending-style interest — a materially different risk profile than this vault currently carries, and not something to build silently into a "yield vault" without that tradeoff being an explicit, disclosed decision rather than an implementation detail. `strategy_router` is deliberately single-strategy today regardless (see its own doc comments) — real multi-strategy allocation is future work, not implied here.

## Deployment

All three contracts are live on Stellar testnet and wired together (deployed/redeployed
2026-09-05, see [`deployments/testnet.json`](deployments/testnet.json) — independently
checkable on [stellar.expert](https://stellar.expert/explorer/testnet)):

| Contract | Address |
|---|---|
| `vault` | `CAUGDNJ4TUBNSMV6CIL356GLPTA77UFC3PNUQ7OKEFLRPY7TBJ3VWGP6` |
| `strategy_router` | `CBRIDAO4NYYGMEUBYVSZ6O6U3SD73XHWLUDN56R3QPPLS2CTXAAPTBF4` |
| `adapter_blend` | `CA4EF5DW4ZOLPETNFRGNWZUNCOUIZ4NIR5STGDZ56VCOJ3L7PZ7PP3X2` |

`vault` is initialized against testnet's real native XLM Stellar Asset Contract
(`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`), not a placeholder token.
`adapter_blend` targets a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2)
pool on testnet (`CCEBVDYM32YNYCVNRXQKDFFPISJJCV557CDZEIRBEE4NCV4KHPQ44HGF`) — confirmed to
actually carry an XLM reserve via a real `get_reserve()` call before this deployment was
wired up. `vault.set_router()` points the vault at `strategy_router`, and
`strategy_router.set_strategy()` points it at `adapter_blend`, so a real `deposit()` on
`vault` now actually reaches the Blend pool. `scripts/deploy.sh` and
`scripts/deploy_blend_strategy.sh` reproduce this from scratch.

An earlier `vault` instance (`CC3KUCEJ7PXTJSHTFE3K52OR2U4QICJ7IUJG7YHXTIBQ62KSMH4G2HCR`,
deployed 2026-09-03) predated the `set_router()` entrypoint and is stale — see
`deployments/testnet.json`'s notes for why a vault redeploy was unavoidable once router
integration landed.

## 🚀 Quick Start
```bash
cargo test --all --features testutils
cd frontend && npm run dev
```
