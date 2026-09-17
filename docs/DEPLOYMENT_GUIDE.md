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

`contracts/adapters/blend` and `contracts/strategy_router` have real, working
implementations (real Blend Protocol V2 cross-contract calls and real multi-strategy
routing, respectively — see `docs/STRATEGIES.md`), but neither is deployed by
`deploy.sh` yet; only `vault` is. `contracts/adapters/phoenix` is the one still a bare
`version() -> 1` stub with no real protocol integration (see the README's Current Status
section). Deploying the real blend/strategy_router pair without also wiring them into
the vault via `set_router` wouldn't do anything useful yet; deploying phoenix would put a
contract address on-chain implying a working yield strategy that doesn't exist.
