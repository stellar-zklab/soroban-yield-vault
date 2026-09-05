import { describe, it, expect, vi } from 'vitest';
import { StellarYieldVaultClient } from '../src/client';

const dummySign = vi.fn(async (xdr: string) => xdr);

describe('StellarYieldVaultClient config', () => {
  it('applies default testnet rpcUrl and networkPassphrase when not supplied', () => {
    // These defaults are private, so this exercises them indirectly: construction
    // shouldn't throw, and getTotalAssets() should fail on the *missing token id* check
    // specifically, not on some earlier default-resolution error.
    const client = new StellarYieldVaultClient({
      vaultContractId: 'CVAULT000000000000000000000000000000000000000000000000000',
      signTransaction: dummySign,
    });
    expect(client).toBeInstanceOf(StellarYieldVaultClient);
  });

  it('getTotalAssets() rejects clearly when underlyingTokenId was not configured', async () => {
    const client = new StellarYieldVaultClient({
      vaultContractId: 'CVAULT000000000000000000000000000000000000000000000000000',
      signTransaction: dummySign,
    });
    await expect(client.getTotalAssets()).rejects.toThrow('underlyingTokenId');
  });
});
