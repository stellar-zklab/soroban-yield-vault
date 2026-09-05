# Contributing to soroban-yield-vault 🏦🌾

Welcome to **`soroban-yield-vault`**! We are building the primary **automated ERC-4626 Yield Optimizer and Strategy Allocator** for the Stellar (Soroban) DeFi ecosystem.

We welcome contributions from Rust contract developers, quantitative strategists, DEX integration engineers, and frontend builders.

---

## 🚀 About the Protocol & Ecosystem Impact

`soroban-yield-vault` optimizes liquidity provision across Stellar:
- Implements the **ERC-4626 Tokenized Vault Standard** for Soroban assets.
- Dynamically allocates capital across **Blend Capital lending pools** and **Phoenix DEX AMMs**.
- Auto-compounds strategy rewards back into underlying shares.
- Protects depositors against flash deposit sandwich attacks.

---

## 🗺️ Technical Architecture & Contribution Roadmap

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DEVELOPMENT ROADMAP PHASES                          │
│                                                                         │
│  Phase 1: ERC-4626 Vault & Strategy Adapters (Scaffolded & Verified)   │
│    ├── Vault contract (deposit/withdraw/share math)                     │
│    ├── StrategyRouter contract                                          │
│    └── Protocol adapters (adapter-blend, adapter-phoenix)               │
│                                                                         │
│  Phase 2: Yield Analytics & Keeper Bots (Active Contribution)          │
│    ├── TypeScript SDK (@stellar-zklab/yield-vault-sdk)                 │
│    ├── Automated Keeper bot for harvest() execution                     │
│    └── Reflector Oracle price feed integration                          │
│                                                                         │
│  Phase 3: Vault Dashboard & Multi-Asset Baskets (Upcoming)             │
│    ├── React Yield Dashboard UI                                         │
│    ├── Multi-asset basket vault contract (50/50 XLM-USDC pools)         │
│    └── Historical APY analytics indexer                                 │
│                                                                         │
│  Phase 4: Decentralized Governance & Security (Future)                 │
│    ├── Strategy weight allocation governance voting                     │
│    ├── Invariant test suite for share pricing math                      │
│    └── Flash-loan sandwich attack prevention verification               │
└─────────────────────────────────────────────────────────────────────────┘
```

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
3. **Verify Locally**: Ensure `cargo test --all --features testutils` passes.
4. **Submit PR**: Open a Pull Request referencing the issue number (e.g. `Closes #5`).

Thank you for advancing DeFi yield infrastructure on Stellar! 🏦
