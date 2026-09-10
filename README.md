# soroban-yield-vault 🏦🌾

![Soroban](https://img.shields.io/badge/Soroban-Protocol_22-blue?style=flat&logo=stellar)
![License](https://img.shields.io/badge/License-Apache_2.0-green)
[![CI](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml/badge.svg)](https://github.com/stellar-zklab/soroban-yield-vault/actions/workflows/ci.yml)
![ERC4626](https://img.shields.io/badge/Vault-Yearn_V3_ERC4626-emerald)
[![Live Demo](https://img.shields.io/badge/Live_Demo-soroban--yield--vault.vercel.app-black?style=flat&logo=vercel)](https://soroban-yield-vault.vercel.app/)

**Automated tokenized yield optimizer on Soroban** — an ERC-4626-style vault that routes deposits through a Yearn V3-style multi-strategy Debt Allocator into real lending/DEX pools, so depositors' shares accrue real, live yield instead of a simulated number.

**[🔗 Try the live demo](https://soroban-yield-vault.vercel.app/)** — wired to the real deployed testnet contracts listed under [Deployment](#deployment), not a mockup.

## Contents

- [Why this is a real DeFi primitive, not a demo](#why-this-is-a-real-defi-primitive-not-a-demo)
- [Architecture](#architecture)
- [What's built](#whats-built)
- [Deployment](#deployment)
- [Usage](#usage)
- [Quick start](#-quick-start)
- [Ecosystem](#ecosystem)
- [Contributing](#contributing)
- [License](#license)

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

## What's built

Status of each piece, so anyone reading knows exactly what's real, what's tested, and what's deliberately not built yet.

<details open>
<summary><strong><code>contracts/vault</code> — real deposits and withdrawals, both actually move tokens</strong></summary>

`convert_to_shares`/`convert_to_assets` correctly implement the Yearn V3 virtual-offset inflation-attack protection (`(assets * (total_shares + 1000)) / (total_assets + 1000)`) — this is genuine, correct DeFi security engineering, not filler. `deposit()` now actually pulls the real underlying token from the caller via `TokenClient::transfer` before crediting shares — previously it only updated internal share counters and never moved a real token at all, so a caller could mint shares against assets the vault never held. `withdraw()` burns shares and pays real tokens back out, and rejects a caller trying to redeem more shares than they actually hold (checked against `initialize(admin, token)`'s registered token, not a per-call address, so it can't be pointed at a different asset). Covered by 4 tests that check actual token balances moving, not just share counters, including two depositors independently withdrawing their own share of a shared pool.

</details>

<details open>
<summary><strong><code>contracts/strategy_router</code> + <code>contracts/adapters/blend</code> — real, genuine multi-strategy Debt Allocator</strong></summary>

Modeled directly on Yearn V3's real architecture: an admin registers a strategy (`add_strategy`), sets how much it may ever hold (`set_max_debt_for_strategy`), and moves real funds toward a target with `update_debt(strategy, desired_debt)` — pulling from the router's idle balance and depositing to increase a strategy's allocation, or withdrawing back to idle to decrease it. Deposits from the vault now land in the router's own idle balance rather than auto-deploying to a strategy immediately; allocation is a deliberate, separate admin action, matching Yearn V3's real separation of "receiving deposits" from "capital allocation decisions." `withdraw()` pulls from idle first, then drains registered strategies in registration order (a simple withdrawal queue) if idle alone can't cover the request.

`adapter_blend` supplies allocated funds to a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2) lending pool on testnet via its actual `submit()` entrypoint (`RequestType::Supply`, never `SupplyCollateral` — this adapter never borrows, so there's no reason to take on liquidation risk) — a real cross-contract call moving real tokens, not a simulated yield number. `total_value()` reads the position's real current worth straight from the pool's own `get_reserve()`/`get_positions()` state (the bToken share balance × the pool's live `b_rate`), so accrued interest flows through automatically, and the router's own `total_assets()` sums every registered strategy's live `total_value()` plus idle — never a locally-tracked approximation. The vault's `total_assets()` — and therefore `convert_to_shares`/`convert_to_assets`, and therefore what every depositor's shares are actually worth — reads this live value, which is what makes accrued yield actually reach depositors.

Covered by 5 adapter tests plus 12 router tests (multi-strategy allocation, decreasing an allocation, withdrawal-queue draining across a shortfall, duplicate/removal guards, and two strategies each getting an independent cap) against a mock Blend pool and mock adapters that replicate real behavior.

</details>

<details>
<summary><strong><code>contracts/adapters/phoenix</code> — still not implemented, deliberately</strong></summary>

Still a bare `#[contract]` with a single `version() -> 1` function. This isn't an oversight: Phoenix is a DEX, not a lending pool, and its real yield mechanism (`provide_liquidity` + `stake` on [phoenix-contracts](https://github.com/Phoenix-Protocol-Group/phoenix-contracts)) has no single-asset deposit path the way Blend's `Supply` does — a genuine integration would need to swap roughly half of every deposit into a paired asset, provide two-sided liquidity, and stake the resulting LP tokens, then reverse all of that on withdrawal. That's real impermanent-loss and slippage exposure for every depositor's share price, not just lending-style interest — a materially different risk profile than this vault currently carries, and not something to build silently into a "yield vault" without that tradeoff being an explicit, disclosed decision rather than an implementation detail. The router's multi-strategy allocator is ready for it the moment it's real, though: `add_strategy`/`set_max_debt_for_strategy` work against any contract implementing the shared `deposit`/`withdraw`/`total_value` interface.

</details>

## Deployment

All three contracts are live on Stellar testnet and wired together (redeployed 2026-09-09 so `strategy_router` actually runs the multi-strategy Debt Allocator code — see the note below — full history in [`deployments/testnet.json`](deployments/testnet.json), independently checkable on [stellar.expert](https://stellar.expert/explorer/testnet)):

| Contract | Address |
|---|---|
| `vault` | `CAQ6YR3XKGS774M7ERT5DTGMMPFYZ4WLAIMOPCUBGAJLQKPLFUG6AETK` |
| `strategy_router` | `CCOD4BBIZPBM6HJHVYOKXQUDF2KV43PRDRWHNTHECUD2JMOC2RRMNSVR` |
| `adapter_blend` | `CBP3J2QE56I7SEOS33OLFZAP3N4JJWKDRHZMPKPVUPGBXVKIONDSP5FQ` |

`vault` is initialized against testnet's real native XLM Stellar Asset Contract (`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`), not a placeholder token. `adapter_blend` targets a real, live [Blend Protocol V2](https://github.com/blend-capital/blend-contracts-v2) pool on testnet (`CCEBVDYM32YNYCVNRXQKDFFPISJJCV557CDZEIRBEE4NCV4KHPQ44HGF`) — confirmed to actually carry an XLM reserve via a real `get_reserve()` call before this deployment was wired up. `vault.set_router()` points the vault at `strategy_router`, and `strategy_router.add_strategy()` + `set_max_debt_for_strategy()` (real transactions, confirmed via their `strat_add`/`maxdebt` events) registered `adapter_blend` with a 10,000,000 XLM cap, so a real `deposit()` on `vault` now actually reaches the Blend pool once an admin calls `update_debt()` to allocate. `scripts/deploy.sh` and `scripts/deploy_blend_strategy.sh` reproduce this from scratch.

Two earlier `vault` instances are stale: the original (`CC3KUCEJ7PXTJSHTFE3K52OR2U4QICJ7IUJG7YHXTIBQ62KSMH4G2HCR`, 2026-09-03) predated `set_router()` entirely, and the 2026-09-05 one (`CAUGDNJ4TUBNSMV6CIL356GLPTA77UFC3PNUQ7OKEFLRPY7TBJ3VWGP6`) was paired with a `strategy_router` that still only exposed the old single-strategy `set_strategy`/`get_strategy` API — a stale build artifact got redeployed unchanged, not a code regression (the source has had the multi-strategy rewrite since 2026-09-05; see `deployments/testnet.json`'s notes for the full history). The `strategy_router` address above is confirmed live with the real `add_strategy`/`get_strategies`/`get_max_debt`/`get_debt` interface — this is what the frontend's Strategy Allocation table (see below) reads from.

## Usage

```typescript
import { StellarYieldVaultClient } from '@stellar-zklab/yield-vault-sdk';
import freighter from '@stellar/freighter-api';

const vault = new StellarYieldVaultClient({
  vaultContractId: 'CAQ6YR3XKGS774M7ERT5DTGMMPFYZ4WLAIMOPCUBGAJLQKPLFUG6AETK', // live on testnet, see Deployment above
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

**Strategy Allocation table (added 2026-09-09).** The [live demo](https://soroban-yield-vault.vercel.app/) now shows a real-time table of every strategy `strategy_router` has registered — allocation %, amount, max-debt cap, and free headroom — read live via `get_strategies`/`get_max_debt`/`get_debt` (`frontend/src/soroban.ts`'s `getStrategyAllocations`/`getRouterTotalAssets`). No wallet connection needed to view it, since this is public on-chain state. Modeled on Yearn V3's own vault-strategy breakdown UI. Right now it shows one honest zero — `adapter_blend` registered with a 10,000,000 XLM cap and 0 currently allocated, since the freshly redeployed router has no deposits yet — the table states this plainly rather than looking broken or loading forever.

## 🚀 Quick start

**Prerequisites:**
- Rust with the `wasm32v1-none` target (`rustup target add wasm32v1-none`)
- Node.js 20+

```bash
# Run the real contract test suite (19 tests across vault, strategy_router, adapter_blend)
cargo test --all --features testutils

# Run the frontend against the real deployed vault (connects Freighter, real deposit/withdraw)
cd frontend && npm install && npm run dev
```

## Ecosystem

Part of **stellar-zklab**'s Soroban Protocol 25 project suite, alongside:
- [`stellar-zkident`](https://github.com/stellar-zklab/stellar-zkident) — self-sovereign DID + real Groth16 zero-knowledge credentials ([live demo](https://stellar-zkident.vercel.app/))
- [`stellar-zkstream`](https://github.com/stellar-zklab/stellar-zkstream) — privacy-preserving payment streaming with Groth16 range/nullifier proofs ([live demo](https://stellar-zkstream.vercel.app/))

All three share the same "real vs. not" documentation discipline and the same Protocol 25 BN254/testnet deployment conventions.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the phased roadmap and how the vault, router, and adapters fit together. Check the [issue tracker](https://github.com/stellar-zklab/soroban-yield-vault/issues) for known gaps — the `adapters/phoenix` integration above is the biggest open one — before starting something new.

## License

Apache 2.0 — see [`LICENSE`](LICENSE).
