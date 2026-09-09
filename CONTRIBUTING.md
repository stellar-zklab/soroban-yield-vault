# Contributing to soroban-yield-vault 🏦🌾

Welcome to **`soroban-yield-vault`**! We are building the primary **automated ERC-4626 Yield Optimizer and Strategy Allocator** for the Stellar (Soroban) DeFi ecosystem.

We welcome contributions from Rust contract developers, quantitative strategists, DEX integration engineers, and frontend builders.

---

## 🚀 About the Protocol & Ecosystem Impact

`soroban-yield-vault` is a real, tested ERC-4626-style tokenized vault for Soroban:
- Implements the **ERC-4626 Tokenized Vault Standard** with Yearn V3's virtual-offset
  inflation-attack protection — genuine share-pricing security math, not filler.
- Actually deploys deposits to a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2)
  lending pool on testnet via `strategy_router` + `adapter-blend`, and reads accrued
  interest back live so depositor share prices reflect real yield, not a stale snapshot.
- `adapter-phoenix` is deliberately not built yet — Phoenix is a DEX, not a lending pool,
  and real yield there means swap + two-sided liquidity + staking, with real
  impermanent-loss exposure. See the main README's Current Status for the full reasoning
  behind holding off rather than building that in silently.

---

## 🗺️ Technical Architecture & Contribution Roadmap

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DEVELOPMENT ROADMAP PHASES                          │
│                                                                         │
│  Phase 1: ERC-4626 Vault & Multi-Strategy Debt Allocator (Built & Tested)│
│    ├── Vault contract (deposit/withdraw/share math, real token moves)  │
│    ├── StrategyRouter: multi-strategy Debt Allocator (Yearn V3 model) │
│    │   — add_strategy/set_max_debt_for_strategy/update_debt, real     │
│    │   withdrawal-queue draining across registered strategies         │
│    └── adapter-blend: real Blend V2 integration, live testnet pool    │
│                                                                         │
│  Phase 2: SDK & Second Strategy (Active Contribution)                  │
│    ├── TypeScript SDK (@stellar-zklab/yield-vault-sdk) — real, tested │
│    └── adapter-phoenix: real swap+LP+stake integration, with IL       │
│        exposure disclosed plainly wherever share price is shown       │
│                                                                         │
│  Phase 3: Vault Dashboard & Analytics (Upcoming)                       │
│    ├── React yield dashboard UI                                        │
│    └── Historical APY tracking from real on-chain events              │
│                                                                         │
│  Phase 4: Security Hardening (Future)                                  │
│    ├── Third-party audit — no audit has happened yet                  │
│    └── Invariant/fuzz test suite for share-pricing math under yield   │
└─────────────────────────────────────────────────────────────────────────┘
```

**Also tracked, not yet started: Protocol 28's migration-friendly contract data (CAP-86).** This workspace hit a real deserialization trap earlier from `BlendReserveConfig` not exactly field-matching Blend's real on-chain struct layout — Soroban's current host functions reject a struct that doesn't match exactly, missing or extra fields included. CAP-86's "sparse" host functions are aimed directly at that class of bug, tolerating schema drift instead of trapping on it. Mainnet vote is 2026-09-16; adopting this waits on stable SDK support, but it's a direct fit for a real problem this repo has already hit once.

---

## 🛠️ Developer Environment Quickstart

### Prerequisites
- **Rust Toolchain**: `rustup target add wasm32v1-none`
- **Stellar CLI**: v22.0.0+

### Build & Run Tests

```bash
# Clone the repository
git clone https://github.com/stellar-zklab/soroban-yield-vault.git
cd soroban-yield-vault

# Check for accidentally committed secrets or leftover local artifacts (.env, .claude/, etc.)
bash scripts/check-source-artifacts.sh

# Run unit tests across all 4 contracts
cargo test --all --features testutils

# Compile release WASM binaries
cargo build --release --target wasm32v1-none
```

---

## 🌿 Git Branch & Conventional Commits

| Prefix | Usage | Example |
|---|---|---|
| `feat:` | New feature or contract function | `feat(vault): add TVL deposit cap manager` |
| `fix:` | Bug fix or logic patch | `fix(router): resolve rebalance rounding error` |
| `docs:` | Documentation updates | `docs(adapters): add strategy development guide` |
| `adapter:` | Adapter crate updates | `adapter(blend): update supply rate query` |

---

## 📋 How to Claim an Issue & Submit a PR

1. **Pick an Issue**: Browse open tasks on [GitHub Issues](https://github.com/stellar-zklab/soroban-yield-vault/issues). Look for [`good-first-issue`](https://github.com/stellar-zklab/soroban-yield-vault/issues?q=is%3Aissue+is%3Aopen+label%3A%22good-first-issue%22).
2. **Create a Branch**: `git checkout -b feat/your-feature-name`
3. **Verify Locally**: Run `bash scripts/check-source-artifacts.sh` and ensure `cargo test --all --features testutils` passes — the same checks CI runs.
4. **Submit PR**: Open a Pull Request referencing the issue number (e.g. `Closes #5`).

Thank you for advancing DeFi yield infrastructure on Stellar! 🏦
