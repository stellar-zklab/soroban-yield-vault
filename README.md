# soroban-yield-vault 🏦🌾

> **Automated ERC-4626 Yield Optimizer & Strategy Allocator on Soroban**  
> *Cross-Protocol Yield Farming for Blend Capital Lending Pools & Phoenix DEX Liquidity*

[![CI](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml/badge.svg)](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Stellar Drips Wave](https://img.shields.io/badge/Stellar-Drips%20Wave-blueviolet)](https://drips.network)
[![Soroban Version](https://img.shields.io/badge/Soroban-v22.0.0-orange)](https://developers.stellar.org)

---

## Executive Summary

`soroban-yield-vault` is an automated yield aggregator and asset management protocol built natively on **Soroban**, Stellar's smart contract platform. 

Modeled after the **ERC-4626 Tokenized Vault Standard**, it allows liquidity providers to deposit Stellar assets (e.g. XLM, USDC) in exchange for yield-bearing vault shares (`vTokens`). The vault continuously routes capital to the highest-yielding strategies across the Stellar DeFi ecosystem — including **Blend Capital lending pools** and **Phoenix DEX automated market makers (AMMs)** — automatically compounding rewards to maximize real APY.

---

## Key Features & Protocol Innovations

- 🏦 **ERC-4626 Tokenized Vault Standard**: Full implementation of standardized vault operations (`deposit`, `withdraw`, `total_assets`, `get_share_balance`).
- 🔀 **Dynamic Strategy Router**: Programmatic capital rebalancer that dynamically adjusts asset allocations across lending and DEX strategy adapters based on real-time APY.
- 🌾 **Auto-Compounding Yield Engine**: Harvests strategy rewards and reinvests them back into the vault, compounding interest without requiring manual user intervention.
- 🔌 **Modular Strategy Adapters**: Plug-and-play architecture for protocol adapters:
  - `adapter-blend`: Supplies asset liquidity to Blend Capital pools.
  - `adapter-phoenix`: Supplies LP liquidity to Phoenix DEX pools.
- 🛡️ **Flash Deposit & Sandwich Attack Protection**: Ledger sequence tracking prevents flash-loan sandwich attacks by locking intra-block deposit/withdrawal arbitrage.

---

## Protocol Architecture & Capital Flow

```
                               ┌───────────────────────────────────┐
                               │            LIQUIDITY PROVIDER     │
                               └─────────────────┬─────────────────┘
                                                 │ deposit(Asset)
                                                 ▼
                               ┌───────────────────────────────────┐
                               │           VaultContract           │
                               │           (ERC-4626 Shares)       │
                               └─────────────────┬─────────────────┘
                                                 │ allocate()
                                                 ▼
                               ┌───────────────────────────────────┐
                               │      StrategyRouterContract       │
                               └────────┬─────────────────┬────────┘
                                        │                 │
                   ┌────────────────────┘                 └────────────────────┐
                   ▼                                                           ▼
    ┌───────────────────────────────┐                           ┌───────────────────────────────┐
    │     BlendAdapterContract      │                           │    PhoenixAdapterContract     │
    └──────────────┬────────────────┘                           └──────────────┬────────────────┘
                   │                                                           │
                   ▼                                                           ▼
    ┌───────────────────────────────┐                           ┌───────────────────────────────┐
    │     Blend Capital Pool        │                           │       Phoenix DEX Pool        │
    └───────────────────────────────┘                           └───────────────────────────────┘
```

---

## Cryptographic & Mathematical Specification

### 1. ERC-4626 Share Pricing Math
When a user deposits an asset amount $A$, the number of vault shares $S$ minted is calculated as:

$$S = \begin{cases} 
A & \text{if } S_{\text{total}} = 0 \lor A_{\text{total}} = 0 \\
\frac{A \cdot S_{\text{total}}}{A_{\text{total}}} & \text{otherwise}
\end{cases}$$

Where:
- $S_{\text{total}}$ is the total supply of vault shares (`TotalShares`).
- $A_{\text{total}}$ is the total underlying asset balance controlled by the vault (`total_assets`).

### 2. Share Withdrawal Formula
When redeeming shares $S$ for underlying assets $A_{\text{return}}$:

$$A_{\text{return}} = \frac{S \cdot A_{\text{total}}}{S_{\text{total}}}$$

---

## Smart Contract API Reference

### 1. `VaultContract` (`contracts/vault`)

#### `initialize(env: Env, admin: Address, asset: Address, router: Address)`
Initializes the vault with underlying token asset and strategy router address.

#### `deposit(env: Env, caller: Address, amount: i128) -> i128`
Transfers `amount` of underlying asset to escrow and mints proportional vault shares to `caller`.
- **Returns**: Number of vault shares minted (`i128`).

#### `withdraw(env: Env, caller: Address, shares: i128) -> i128`
Burns `shares` from `caller` and transfers proportional underlying asset balance.
- **Returns**: Amount of underlying asset returned (`i128`).

#### `total_assets(env: Env) -> i128`
Returns total underlying asset balance managed by the contract.

#### `get_share_balance(env: Env, user: Address) -> i128`
Returns vault share balance for a given user address.

---

### 2. `StrategyRouterContract` (`contracts/strategy_router`)
Routes vault liquidity across registered strategy adapters and rebalances capital allocation.

---

### 3. Strategy Adapters (`contracts/adapters/blend` & `contracts/adapters/phoenix`)
Implements standardized interface for external protocol deposits, withdrawals, and yield harvesting.

---

## Directory Structure

```
soroban-yield-vault/
├── contracts/
│   ├── vault/                # ERC-4626 tokenized vault contract
│   ├── strategy_router/      # Capital allocator & rebalancing router
│   └── adapters/
│       ├── blend/            # Blend Capital lending protocol adapter
│       └── phoenix/          # Phoenix DEX LP yield adapter
├── sdk/                      # TypeScript SDK
├── frontend/                 # React yield optimizer dashboard
├── docs/                     # Architecture, strategies, deployment guides
└── scripts/
    └── deploy.sh             # Testnet deployment script
```

---

## Developer Quick Start

### Build & Test Contracts

```bash
git clone https://github.com/stellar-zklab/soroban-yield-vault.git
cd soroban-yield-vault

# Run test suite across all 4 contracts
cargo test --all --features testutils

# Compile release WASM binaries
cargo build --release --target wasm32v1-none
```

### Testnet Deployment

```bash
cp .env.example .env
# Set STELLAR_ACCOUNT in .env
bash scripts/deploy.sh
```

---

## 🌊 Contributing — Stellar Drips Wave

`soroban-yield-vault` participates in the **[Stellar Drips Wave](https://drips.network)** program.

| Category | Points | Tasks |
|---|---|---|
| 🔴 **High Complexity** | 200 pts | Share calculation math, Strategy Router, protocol adapters |
| 🟡 **Medium Complexity** | 150 pts | Emergency pause, TypeScript SDK, React dashboard |
| 🟢 **Trivial Complexity** | 100 pts | Documentation, deployment scripts, APY view functions |

Browse open issues on [GitHub Issues](https://github.com/stellar-zklab/soroban-yield-vault/issues).

---

## License

Licensed under **Apache License 2.0**. See [LICENSE](LICENSE).
