# soroban-yield-vault 🏦🌾

![Soroban](https://img.shields.io/badge/Soroban-Protocol_25-blue?style=flat&logo=stellar)
![License](https://img.shields.io/badge/License-Apache_2.0-green)
![Build](https://img.shields.io/badge/Cargo_Test-Passing-brightgreen)
![ERC4626](https://img.shields.io/badge/Vault-Yearn_V3_ERC4626-emerald)

Automated Tokenized Yield Optimizer & Strategy Router across Blend Capital Lending and Phoenix DEX Pools on Soroban.

## Current Status — what's real vs. not

**`contracts/vault` — the share accounting is real, deposits are one-way right now.** `convert_to_shares`/`convert_to_assets`/`deposit` correctly implement the Yearn V3 virtual-offset inflation-attack protection (`(assets * (total_shares + 1000)) / (total_assets + 1000)`) — this is genuine, correct DeFi security engineering, not filler. But there's **no `withdraw`/`redeem` function at all yet** — deposited funds currently have no way back out.

**The actual "yield optimizer" — not implemented.** `contracts/adapters/blend`, `contracts/adapters/phoenix`, and `contracts/strategy_router` are each a bare `#[contract]` with a single `version() -> 1` function and nothing else — no Blend lending integration, no Phoenix DEX integration, no rebalancing logic. The vault's `deposit()` doesn't call into the strategy router at all; deposited assets just sit in the vault contract's own balance. The headline feature of this repo — routing deposits into real yield strategies — doesn't exist yet.

## 🚀 Quick Start
```bash
cargo test --all --features testutils
cd frontend && npm run dev
```
