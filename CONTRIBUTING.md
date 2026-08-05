# Contributing Guidelines

Thank you for your interest in contributing! We welcome pull requests, bug reports, feature proposals, and documentation improvements from the developer community.

## 🛠️ Local Development Quickstart

```bash
# 1. Clone the repository
git clone https://github.com/stellar-zklab/soroban-yield-vault.git
cd soroban-yield-vault

# 2. Run smart contract unit tests
cargo test --all --features testutils

# 3. Build release WASM binaries
cargo build --release --target wasm32v1-none
```

## 🌿 Git Branch & Commit Conventions

Please use conventional commit prefixes for clean commit histories:
- `feat:` New features or contract functionality
- `fix:` Bug fixes or contract logic patches
- `docs:` Documentation, inline comments, or README updates
- `test:` Unit tests or integration test suites
- `ci:` GitHub Actions CI workflow updates

Example: `feat(stream): add rate limiting per sender address`

## 📋 Pull Request Process

1. Fork the repository and create your branch from `main`.
2. Ensure all smart contract unit tests pass (`cargo test --all --features testutils`).
3. Verify that code formatting adheres to standard Rust formatting (`cargo fmt -- --check`).
4. Submit your Pull Request with a descriptive summary of your changes.

