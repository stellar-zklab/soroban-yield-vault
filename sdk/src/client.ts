/**
 * StellarYieldVaultClient — TypeScript SDK wrapper for soroban-yield-vault smart contracts.
 *
 * This wraps the same `@stellar/stellar-sdk/contract` Client the deployed frontend uses
 * (see ../../frontend/src/soroban.ts) — real simulate/sign/submit calls against a real
 * Soroban RPC endpoint, not fixture data. Signing is injected via `signTransaction` rather
 * than hard-wired to Freighter, so this SDK works with any wallet adapter that can produce
 * a signed transaction XDR (Freighter, a server-side signer, xBull, etc.).
 */
import { Client as ContractClient } from '@stellar/stellar-sdk/contract';

export type SignTransaction = (
  xdr: string,
  opts?: { network?: string; networkPassphrase?: string; accountToSign?: string }
) => Promise<string>;

export interface StellarYieldVaultConfig {
  vaultContractId: string;
  rpcUrl?: string;
  networkPassphrase?: string;
  signTransaction: SignTransaction;
}

export interface VaultDepositParams {
  depositor: string;
  amount: bigint;
}

export interface VaultWithdrawParams {
  withdraw_address: string;
  shares: bigint;
}

export class StellarYieldVaultClient {
  private vaultContractId: string;
  private rpcUrl: string;
  private networkPassphrase: string;
  private signTransaction: SignTransaction;

  constructor(config: StellarYieldVaultConfig) {
    this.vaultContractId = config.vaultContractId;
    this.rpcUrl = config.rpcUrl ?? 'https://soroban-testnet.stellar.org';
    this.networkPassphrase = config.networkPassphrase ?? 'Test SDF Network ; September 2015';
    this.signTransaction = config.signTransaction;
  }

  private async getClient(publicKey?: string, contractId: string = this.vaultContractId) {
    return ContractClient.from({
      contractId,
      networkPassphrase: this.networkPassphrase,
      rpcUrl: this.rpcUrl,
      publicKey,
      signTransaction: this.signTransaction,
    });
  }

  /** Real, live deposit call — moves the caller's real tokens into the vault and mints
   * real shares using the vault's actual on-chain Yearn V3 virtual-offset formula.
   * Requires `params.depositor` to sign. */
  async deposit(params: VaultDepositParams): Promise<{ sharesMinted: bigint; txHash: string }> {
    const client = await this.getClient(params.depositor);
    const tx = await (client as any).deposit(
      { caller: params.depositor, assets: params.amount },
      { timeoutInSeconds: 1800 }
    );
    const sent = await tx.signAndSend();
    return { sharesMinted: sent.result as bigint, txHash: sent.sendTransactionResponse?.hash ?? sent.getTransactionResponse?.txHash ?? '' };
  }

  /** Real, live withdraw call — burns real shares and pays out the corresponding real
   * underlying asset amount, computed by the vault's own on-chain math. */
  async withdraw(params: VaultWithdrawParams): Promise<{ amountReturned: bigint; txHash: string }> {
    const client = await this.getClient(params.withdraw_address);
    const tx = await (client as any).withdraw(
      { caller: params.withdraw_address, shares: params.shares },
      { timeoutInSeconds: 1800 }
    );
    const sent = await tx.signAndSend();
    return { amountReturned: sent.result as bigint, txHash: sent.sendTransactionResponse?.hash ?? sent.getTransactionResponse?.txHash ?? '' };
  }

  /** Read-only: the vault's real total managed assets, straight from the vault contract's
   * own `total_assets()` — whatever it's holding idle plus whatever its strategy router
   * reports as the real current value of deployed funds (including accrued yield). Correct
   * whether or not a strategy is configured, since the vault computes this live rather than
   * this SDK approximating it from the underlying token's balance. */
  async getTotalAssets(): Promise<bigint> {
    const client = await this.getClient();
    const tx = await (client as any).total_assets();
    return tx.result as bigint;
  }

  /** Read-only: a given user's real share balance. */
  async getShareBalance(user: string): Promise<bigint> {
    const client = await this.getClient();
    const tx = await (client as any).balance_of({ user });
    return tx.result as bigint;
  }

  /** Read-only: previews the real share amount a deposit of `assets` would mint right now,
   * simulated against the vault's real on-chain totals. */
  async previewDeposit(assets: bigint): Promise<bigint> {
    const client = await this.getClient();
    const tx = await (client as any).convert_to_shares({ assets });
    return tx.result as bigint;
  }

  /** Read-only: previews the real asset amount a withdrawal of `shares` would return right
   * now, simulated against the vault's real on-chain totals. */
  async previewWithdraw(shares: bigint): Promise<bigint> {
    const client = await this.getClient();
    const tx = await (client as any).convert_to_assets({ shares });
    return tx.result as bigint;
  }
}
