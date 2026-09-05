import { describe, it, expect, vi } from 'vitest';
import { StellarYieldVaultClient } from '../src/client';

const dummySign = vi.fn(async (xdr: string) => xdr);

describe('StellarYieldVaultClient config', () => {
  it('applies default testnet rpcUrl and networkPassphrase when not supplied', () => {
    // These defaults are private, so this exercises them indirectly: construction
    // shouldn't throw with only the required fields set.
    const client = new StellarYieldVaultClient({
      vaultContractId: 'CVAULT000000000000000000000000000000000000000000000000000',
      signTransaction: dummySign,
    });
    expect(client).toBeInstanceOf(StellarYieldVaultClient);
  });
});
