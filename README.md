# soroban-yield-vault 🏦🌾

> **Automated ERC-4626 Yield Optimizer & Strategy Allocator on Soroban**  
> *Cross-Protocol Yield Farming for Blend Capital Lending Pools & Phoenix DEX Liquidity*

[![CI](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml/badge.svg)](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
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

## Cryptographic & Mathematical Specification

### 1. ERC-4626 Share Pricing Math
When a user deposits an asset amount $A$, the number of vault shares $S$ minted is calculated as:

$$S = \begin{cases} 
A & \text{if } S_{\text{total}} = 0 \lor A_{\text{total}} = 0 \\
\frac{A \cdot S_{\text{total}}}{A_{\text{total}}} & \text{otherwise}
\end{cases}$$

---

## Smart Contract API Reference

### 1. `VaultContract` (`contracts/vault`)

#### `initialize(env: Env, admin: Address, asset: Address, router: Address)`
Initializes the vault with underlying token asset and strategy router address.

#### `deposit(env: Env, caller: Address, amount: i128) -> i128`
Transfers `amount` of underlying asset to escrow and mints proportional vault shares to `caller`.

#### `withdraw(env: Env, caller: Address, shares: i128) -> i128`
Burns `shares` from `caller` and transfers proportional underlying asset balance.

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

---

## 🤝 Contributing & Community Roadmap

`soroban-yield-vault` is an open-source yield primitive for Stellar. We welcome contributions from developers, quantitative strategists, and protocol integration teams!

### How to Contribute
1. **Explore Issues**: Check out open tasks tagged [`good-first-issue`](https://github.com/stellar-zklab/soroban-yield-vault/issues?q=is%3Aissue+is%3Aopen+label%3A%22good-first-issue%22) or [`help-wanted`](https://github.com/stellar-zklab/soroban-yield-vault/issues).
2. **Fork & Branch**: Create a feature branch (`git checkout -b feat/your-feature`).
3. **Test Your Changes**: Ensure all unit tests pass (`cargo test --all --features testutils`).
4. **Submit a Pull Request**: Open a PR with a clear summary of your changes.

---

## License

Licensed under **Apache License 2.0**. See [LICENSE](LICENSE).
