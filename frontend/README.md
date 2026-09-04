# soroban-yield-vault Frontend

React UI application for depositing assets and managing yield vault shares on Stellar.

## Current status — what's real vs. not

**Wired to the real deployed testnet `vault` contract, not mocked.** `src/soroban.ts` uses
`@stellar/stellar-sdk`'s `contract.Client` to talk directly to the real, deployed vault
(see `../deployments/testnet.json`):

- **Real deposit** — needs a connected Freighter wallet. Pulls real native XLM from the
  connected wallet, transferred by the vault contract itself, and mints real vault shares
  using the vault's actual Yearn V3 virtual-offset share formula, computed on-chain (not a
  local approximation).
- **Real withdraw** — burns real vault shares and pays out the corresponding real XLM,
  computed by the same on-chain math.
- **Real share balance & conversion previews** — read directly from the vault contract's
  own storage via simulated (unsigned) calls; no wallet needed to read.

**What's still an honest stub:** `contracts/adapters/blend`, `contracts/adapters/phoenix`,
and `contracts/strategy_router` are bare `version() -> 1` stubs with no real protocol
integration yet (see the root README's Current Status section). Deposited funds sit in the
vault contract itself and earn no real yield — there is no strategy routing. This UI does
not claim otherwise; the banner at the top says so.

## Prerequisites to actually use it

- The [Freighter](https://freighter.app) browser extension, set to **Testnet**.
- A funded testnet account (Freighter can request testnet XLM from friendbot itself).

## Running it

```bash
npm install
npm run dev
```
