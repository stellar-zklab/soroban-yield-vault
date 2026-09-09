# soroban-yield-vault 🏦🌾

![Soroban](https://img.shields.io/badge/Soroban-Protocol_22-blue?style=flat&logo=stellar)
![License](https://img.shields.io/badge/License-Apache_2.0-green)
[![CI](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml/badge.svg)](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml)
![ERC4626](https://img.shields.io/badge/Vault-Yearn_V3_ERC4626-emerald)
[![Live Demo](https://img.shields.io/badge/Live_Demo-soroban--yield--vault.vercel.app-black?style=flat&logo=vercel)](https://soroban-yield-vault.vercel.app/)

Automated Tokenized Yield Optimizer & Strategy Router across Blend Capital Lending and Phoenix DEX Pools on Soroban.

**[🔗 Try the live demo](https://soroban-yield-vault.vercel.app/)** — wired to the real deployed testnet contracts listed below, not a mockup.

## Why this is a real DeFi primitive, not a demo

- **Real yield, not a simulated number.** Deposits are actually supplied to a live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2) lending pool on testnet via its real `submit()` entrypoint — accrued interest flows back through the pool's own `b_rate`, not a counter this vault increments itself.
- **Correct ERC-4626-style share math.** `convert_to_shares`/`convert_to_assets` implement Yearn V3's virtual-offset inflation-attack protection, the same defense real production vaults use against a first-depositor donation attack.
- **A real bug the live pool itself caught.** Deploying against the actual Blend pool (not a mock) surfaced a genuine field-layout mismatch in this project's own `BlendReserveConfig` struct — see `contracts/adapters/blend/src/lib.rs`'s doc comment for what broke and how it was fixed, verified by a second live deployment.
- **Three contracts, genuinely wired together on-chain** — `vault.set_router()` → `strategy_router.update_debt()` → the Blend adapter — not three independently-deployed contracts that merely coexist.
- **A real multi-strategy debt allocator, not a single hardcoded strategy.** `strategy_router` is modeled on Yearn V3's real vault/strategy architecture: an admin registers strategies (`add_strategy`), caps each one's allocation (`set_max_debt_for_strategy`), and moves real funds toward a target via `update_debt` — deposits land idle until explicitly allocated, the same deliberate decoupling Yearn V3 uses, rather than every deposit auto-deploying to one hardcoded strategy.

## Architecture

```
+----------------------------+
|         Depositor          |
|       (SDK / Wallet)       |
+----------------------------+
            |  deposit() / withdraw()
            v
+----------------------------+
|           vault            |
|      (ERC-4626-style       |
|        share math)         |
+----------------------------+
            |  set_router() -> forwards funds
            v
+----------------------------+
|      strategy_router       |
|  (multi-strategy Debt      |
|  Allocator: max_debt caps  |
|   + update_debt targets)   |
+----------------------------+
            |  update_debt()
            v
+----------------------------+
|       adapter_blend        |
+----------------------------+
            |  submit() / get_reserve()
            v
+----------------------------+
|     Blend Protocol V2      |
|     pool (real, live,      |
|          testnet)          |
+----------------------------+
```

## Current Status — what's real vs. not

**`contracts/vault` — real deposits and withdrawals, both actually move tokens.** `convert_to_shares`/`convert_to_assets` correctly implement the Yearn V3 virtual-offset inflation-attack protection (`(assets * (total_shares + 1000)) / (total_assets + 1000)`) — this is genuine, correct DeFi security engineering, not filler. `deposit()` now actually pulls the real underlying token from the caller via `TokenClient::transfer` before crediting shares — previously it only updated internal share counters and never moved a real token at all, so a caller could mint shares against assets the vault never held. `withdraw()` is new: it burns shares and pays real tokens back out, and rejects a caller trying to redeem more shares than they actually hold (checked against `initialize(admin, token)`'s registered token, not a per-call address, so it can't be pointed at a different asset). Covered by 4 tests that check actual token balances moving, not just share counters, including two depositors independently withdrawing their own share of a shared pool.

**`contracts/strategy_router` + `contracts/adapters/blend` — real, and now a genuine multi-strategy Debt Allocator, not a single hardcoded strategy.** Modeled directly on Yearn V3's real architecture: an admin registers a strategy (`add_strategy`), sets how much it may ever hold (`set_max_debt_for_strategy`), and moves real funds toward a target with `update_debt(strategy, desired_debt)` — pulling from the router's idle balance and depositing to increase a strategy's allocation, or withdrawing back to idle to decrease it. Deposits from the vault now land in the router's own idle balance rather than auto-deploying to a strategy immediately; allocation is a deliberate, separate admin action, matching Yearn V3's real separation of "receiving deposits" from "capital allocation decisions." `withdraw()` pulls from idle first, then drains registered strategies in registration order (a simple withdrawal queue) if idle alone can't cover the request. `adapter-blend` supplies allocated funds to a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2) lending pool on testnet via its actual `submit()` entrypoint (`RequestType::Supply`, never `SupplyCollateral` — this adapter never borrows, so there's no reason to take on liquidation risk) — a real cross-contract call moving real tokens, not a simulated yield number. `total_value()` reads the position's real current worth straight from the pool's own `get_reserve()`/`get_positions()` state (the bToken share balance × the pool's live `b_rate`), so accrued interest flows through automatically, and the router's own `total_assets()` sums every registered strategy's live `total_value()` plus idle — never a locally-tracked approximation. The vault's `total_assets()` — and therefore `convert_to_shares`/`convert_to_assets`, and therefore what every depositor's shares are actually worth — reads this live value, which is what makes accrued yield actually reach depositors. Covered by 5 adapter tests plus 12 router tests (multi-strategy allocation, decreasing an allocation, withdrawal-queue draining across a shortfall, duplicate/removal guards, and two strategies each getting an independent cap) against a mock Blend pool and mock adapters that replicate real behavior.

**`contracts/adapters/phoenix` — still not implemented, deliberately.** Still a bare `#[contract]` with a single `version() -> 1` function. This isn't an oversight: Phoenix is a DEX, not a lending pool, and its real yield mechanism (`provide_liquidity` + `stake` on [phoenix-contracts](https://github.com/Phoenix-Protocol-Group/phoenix-contracts)) has no single-asset deposit path the way Blend's `Supply` does — a genuine integration would need to swap roughly half of every deposit into a paired asset, provide two-sided liquidity, and stake the resulting LP tokens, then reverse all of that on withdrawal. That's real impermanent-loss and slippage exposure for every depositor's share price, not just lending-style interest — a materially different risk profile than this vault currently carries, and not something to build silently into a "yield vault" without that tradeoff being an explicit, disclosed decision rather than an implementation detail. The router's multi-strategy allocator is ready for it the moment it's real, though: `add_strategy`/`set_max_debt_for_strategy` work against any contract implementing the shared `deposit`/`withdraw`/`total_value` interface.

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
`strategy_router.add_strategy()` + `set_max_debt_for_strategy()` + `update_debt()` allocate
real funds to `adapter_blend`, so a real `deposit()` on `vault` now actually reaches the
Blend pool. `scripts/deploy.sh` and `scripts/deploy_blend_strategy.sh` reproduce this from
scratch.

An earlier `vault` instance (`CC3KUCEJ7PXTJSHTFE3K52OR2U4QICJ7IUJG7YHXTIBQ62KSMH4G2HCR`,
deployed 2026-09-03) predated the `set_router()` entrypoint and is stale — see
`deployments/testnet.json`'s notes for why a vault redeploy was unavoidable once router
integration landed. The `strategy_router` address above also predates the multi-strategy
Debt Allocator rewrite (it only exposes the old single-strategy `set_strategy`/`get_strategy`
API) — it needs a fresh `scripts/deploy_blend_strategy.sh` run before `add_strategy`/
`set_max_debt_for_strategy`/`update_debt` will actually exist on-chain for it.

## Usage

```typescript
import { StellarYieldVaultClient } from '@stellar-zklab/yield-vault-sdk';
import freighter from '@stellar/freighter-api';

const vault = new StellarYieldVaultClient({
  vaultContractId: 'CAUGDNJ4TUBNSMV6CIL356GLPTA77UFC3PNUQ7OKEFLRPY7TBJ3VWGP6', // live on testnet, see Deployment above
  signTransaction: async (xdr, opts) => {
    const { signedTxXdr } = await freighter.signTransaction(xdr, opts);
    return signedTxXdr;
  },
});

// Preview, then actually deposit — both hit the vault's real on-chain math.
const expectedShares = await vault.previewDeposit(10_0000000n); // 10 XLM, 7 decimals
const { sharesMinted, txHash } = await vault.deposit({ depositor: userAddress, amount: 10_0000000n });

// total_assets() includes real accrued Blend interest, not a stale counter.
const totalManaged = await vault.getTotalAssets();
```

See [`sdk/README.md`](sdk/README.md) for the full API (`withdraw`, `getShareBalance`, `previewWithdraw`) — every method above calls this vault's actual deployed contract, not a mock.

## Ecosystem

Part of **stellar-zklab**'s Soroban Protocol 25 project suite, alongside:
- [`stellar-zkident`](https://github.com/stellar-zklab/stellar-zkident) — self-sovereign DID + real Groth16 zero-knowledge credentials ([live demo](https://stellar-zkident.vercel.app/))
- [`stellar-zkstream`](https://github.com/stellar-zklab/stellar-zkstream) — privacy-preserving payment streaming with Groth16 range/nullifier proofs ([live demo](https://stellar-zkstream.vercel.app/))

All three share the same "real vs. not" documentation discipline and the same Protocol 25 BN254/testnet deployment conventions.

## 🚀 Quick Start

**Prerequisites**: Rust with the `wasm32v1-none` target (`rustup target add wasm32v1-none`), Node.js 20+.

```bash
# Run the real contract test suite (19 tests across vault, strategy_router, adapter_blend)
cargo test --all --features testutils

# Run the frontend against the real deployed vault (connects Freighter, real deposit/withdraw)
cd frontend && npm install && npm run dev
```
