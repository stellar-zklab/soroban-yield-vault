# soroban-yield-vault 🏦🌾

![Soroban](https://img.shields.io/badge/Soroban-Protocol_25-blue?style=flat&logo=stellar)
![License](https://img.shields.io/badge/License-Apache_2.0-green)
![Build](https://img.shields.io/badge/Cargo_Test-Passing-brightgreen)
![ERC4626](https://img.shields.io/badge/Vault-Yearn_V3_ERC4626-emerald)

Automated Tokenized Yield Optimizer & Strategy Router across Blend Capital Lending and Phoenix DEX Pools on Soroban.

## Current Status — what's real vs. not

**`contracts/vault` — real deposits and withdrawals, both actually move tokens.** `convert_to_shares`/`convert_to_assets` correctly implement the Yearn V3 virtual-offset inflation-attack protection (`(assets * (total_shares + 1000)) / (total_assets + 1000)`) — this is genuine, correct DeFi security engineering, not filler. `deposit()` now actually pulls the real underlying token from the caller via `TokenClient::transfer` before crediting shares — previously it only updated internal share counters and never moved a real token at all, so a caller could mint shares against assets the vault never held. `withdraw()` is new: it burns shares and pays real tokens back out, and rejects a caller trying to redeem more shares than they actually hold (checked against `initialize(admin, token)`'s registered token, not a per-call address, so it can't be pointed at a different asset). Covered by 4 tests that check actual token balances moving, not just share counters, including two depositors independently withdrawing their own share of a shared pool.

**The actual "yield optimizer" — not implemented.** `contracts/adapters/blend`, `contracts/adapters/phoenix`, and `contracts/strategy_router` are each a bare `#[contract]` with a single `version() -> 1` function and nothing else — no Blend lending integration, no Phoenix DEX integration, no rebalancing logic. The vault's `deposit()` still doesn't call into the strategy router; deposited assets sit in the vault contract's own token balance, held but not put to work. The headline feature of this repo — routing deposits into real yield strategies — doesn't exist yet, and needs real Blend/Phoenix protocol integration work this repo hasn't attempted.

## Deployment

`vault` is live on Stellar testnet (deployed 2026-09-03, see
[`deployments/testnet.json`](deployments/testnet.json) — independently checkable on
[stellar.expert](https://stellar.expert/explorer/testnet)):

| Contract | Address |
|---|---|
| `vault` | `CC3KUCEJ7PXTJSHTFE3K52OR2U4QICJ7IUJG7YHXTIBQ62KSMH4G2HCR` |

It's initialized against testnet's real native XLM Stellar Asset Contract
(`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`), not a placeholder token. The
stub adapters and strategy router are deliberately not deployed — see
[`docs/DEPLOYMENT_GUIDE.md`](docs/DEPLOYMENT_GUIDE.md). `scripts/deploy.sh` reproduces this
from scratch.

## 🚀 Quick Start
```bash
cargo test --all --features testutils
cd frontend && npm run dev
```
