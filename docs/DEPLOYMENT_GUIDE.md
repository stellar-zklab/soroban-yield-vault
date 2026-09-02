# soroban-yield-vault Deployment Guide

Deploys `vault` to Stellar testnet, initialized against testnet's real native XLM Stellar
Asset Contract. It passes its real test suite (`cargo test --all --features testutils`,
see the repo README) before this guide is relevant — deployment doesn't substitute for
that.

## Prerequisites
- **Stellar CLI**: `cargo install --locked stellar-cli`
- **Rust Wasm target**: `rustup target add wasm32v1-none`

## Network
- **Network**: `testnet`
- **RPC URL**: `https://soroban-testnet.stellar.org:443`
- **Passphrase**: `"Test SDF Network ; September 2015"`

## Deploy

```bash
bash scripts/deploy.sh
```

This generates and friendbot-funds a `deployer` testnet identity if one doesn't already
exist, resolves testnet's real native XLM Stellar Asset Contract ID, builds and deploys
`vault`, and initializes it with that token. Resulting contract ID lands in
`deployments/testnet.json`.

## What this does NOT deploy

`contracts/adapters/blend`, `contracts/adapters/phoenix`, and `contracts/strategy_router`
are not deployed — each is still a bare `version() -> 1` stub with no real protocol
integration (see the README's Current Status section). Deploying them would put a
contract address on-chain implying a working yield strategy that doesn't exist yet.
