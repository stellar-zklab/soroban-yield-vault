# @stellar-zklab/yield-vault-sdk

TypeScript SDK for interacting with `soroban-yield-vault` smart contracts on Stellar.

## Current Status — what's real

`StellarYieldVaultClient` wraps `@stellar/stellar-sdk/contract`'s `Client` and makes real
simulate/sign/submit calls against a real Soroban RPC endpoint — the same integration
`../frontend/src/soroban.ts` uses, factored out into a reusable package. It is **not**
mocked: `deposit()` and `withdraw()` build a real transaction, run it through your
supplied `signTransaction`, and submit it; the preview/read methods simulate against real
on-chain vault state.

Signing is dependency-injected rather than hard-wired to Freighter, so this SDK works with
any wallet adapter that can produce a signed transaction XDR string.

```ts
import { StellarYieldVaultClient } from '@stellar-zklab/yield-vault-sdk';
import freighter from '@stellar/freighter-api';

const vault = new StellarYieldVaultClient({
  vaultContractId: 'CAUGDNJ4TUBNSMV6CIL356GLPTA77UFC3PNUQ7OKEFLRPY7TBJ3VWGP6',
  signTransaction: async (xdr, opts) => {
    const { signedTxXdr } = await freighter.signTransaction(xdr, opts);
    return signedTxXdr;
  },
});

const { sharesMinted, txHash } = await vault.deposit({ depositor: userAddress, amount: 10_0000000n });
const totalAssets = await vault.getTotalAssets();
```

`getTotalAssets()` calls the vault contract's own real `total_assets()` entrypoint
directly — idle balance plus whatever the configured strategy router reports as the real
current value of deployed funds, including accrued yield. This stays correct whether or
not a router/strategy is configured, since the vault computes it live rather than this SDK
approximating it from a token balance.

## Not implemented

`adapter-phoenix` is still an inert placeholder on-chain, deliberately (see the main repo
README's Current Status section for why) — this SDK has nothing to wrap there. Blend is
real: see the main repo README for the deployed `strategy_router`/`adapter_blend`
addresses this vault is actually wired to.
