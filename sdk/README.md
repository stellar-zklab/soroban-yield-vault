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
  vaultContractId: 'CC3KUCEJ7PXTJSHTFE3K52OR2U4QICJ7IUJG7YHXTIBQ62KSMH4G2HCR',
  underlyingTokenId: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC', // native XLM SAC
  signTransaction: async (xdr, opts) => {
    const { signedTxXdr } = await freighter.signTransaction(xdr, opts);
    return signedTxXdr;
  },
});

const { sharesMinted, txHash } = await vault.deposit({ depositor: userAddress, amount: 10_0000000n });
const totalAssets = await vault.getTotalAssets();
```

The vault contract itself has no public `total_assets()` getter (only an internal
counter), so `getTotalAssets()` reads the vault's real balance of `underlyingTokenId`
directly from that SAC token instead — correct today because deposits move real tokens
into the vault and the yield-strategy adapters currently hold nothing, but it will need
to change if/when the router actually starts routing funds to adapters.

## Not implemented

The vault's strategy adapters (Blend, Phoenix) and `strategy_router` are inert
placeholders on-chain (see the main repo README) — this SDK has nothing to wrap there
yet, since there's no real routing behavior to call.
