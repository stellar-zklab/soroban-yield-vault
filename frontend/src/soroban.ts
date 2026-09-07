// Real integration against the actual deployed Stellar testnet contracts — no mocking
// here. See ../deployments/testnet.json for where these addresses come from and how to
// verify them independently on stellar.expert.
import { Client as ContractClient } from '@stellar/stellar-sdk/contract';
import { StellarWalletsKit } from '@creit.tech/stellar-wallets-kit/sdk';
import { FreighterModule } from '@creit.tech/stellar-wallets-kit/modules/freighter';
import { xBullModule } from '@creit.tech/stellar-wallets-kit/modules/xbull';
import { Networks } from '@creit.tech/stellar-wallets-kit/types';

export const NETWORK_PASSPHRASE = 'Test SDF Network ; September 2015';
export const RPC_URL = 'https://soroban-testnet.stellar.org';

export const VAULT_CONTRACT_ID = 'CAUGDNJ4TUBNSMV6CIL356GLPTA77UFC3PNUQ7OKEFLRPY7TBJ3VWGP6';
export const NATIVE_TOKEN_ID = 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC';

// The native XLM Stellar Asset Contract, like all Stellar assets, uses 7 decimal places
// (stroops). The vault's own i128 amounts are in the same base unit as the underlying
// token, so UI amounts entered in XLM need this conversion both ways.
export const STROOPS_PER_XLM = 10_000_000n;

// Scoped to just Freighter + xBull rather than the kit's full module list (which also
// pulls in Ledger/Trezor hardware-wallet and WalletConnect support) — this is a testnet
// demo, not a production wallet, so only the two lightest, most commonly available
// options are wired in. init() is a one-time, module-level call since StellarWalletsKit's
// methods are static.
let walletKitReady = false;
function ensureWalletKit(): void {
  if (walletKitReady) return;
  StellarWalletsKit.init({
    modules: [new FreighterModule(), new xBullModule()],
    network: Networks.TESTNET,
  });
  walletKitReady = true;
}

/** Opens the kit's real wallet-picker modal (Freighter or xBull), and returns the real
 * connected address. The modal itself handles "wallet not installed" — there's no
 * separate not-detected error to catch here the way the old Freighter-only code needed. */
export async function connectWallet(): Promise<string> {
  ensureWalletKit();
  const { address } = await StellarWalletsKit.authModal();
  return address;
}

async function getClient(contractId: string, publicKey?: string) {
  ensureWalletKit();
  return ContractClient.from({
    contractId,
    networkPassphrase: NETWORK_PASSPHRASE,
    rpcUrl: RPC_URL,
    publicKey,
    signTransaction: StellarWalletsKit.signTransaction,
  });
}

/** Real, live deposit call — pulls real native XLM from the connected wallet into the
 * deployed vault contract and mints real vault shares, using the vault's actual Yearn V3
 * virtual-offset share formula on-chain (not a local approximation). Requires a connected
 * wallet's signature; the same signed auth entry covers both the vault's own
 * `deposit` call and the nested native-token `transfer` it makes internally, since Soroban's
 * auth tree lets one signature authorize the whole sub-invocation tree the SDK simulates. */
export async function depositRealXlm(callerPublicKey: string, amountXlm: number): Promise<bigint> {
  const client = await getClient(VAULT_CONTRACT_ID, callerPublicKey);
  const assets = BigInt(Math.round(amountXlm * Number(STROOPS_PER_XLM)));
  const tx = await (client as any).deposit(
    { caller: callerPublicKey, assets },
    { timeoutInSeconds: 1800 }
  );
  const sent = await tx.signAndSend();
  return sent.result as bigint;
}

/** Real, live withdraw call — burns real vault shares and pays out the corresponding real
 * native XLM back to the connected wallet, computed by the vault's own on-chain math. */
export async function withdrawRealShares(callerPublicKey: string, shares: bigint): Promise<bigint> {
  const client = await getClient(VAULT_CONTRACT_ID, callerPublicKey);
  const tx = await (client as any).withdraw(
    { caller: callerPublicKey, shares },
    { timeoutInSeconds: 1800 }
  );
  const sent = await tx.signAndSend();
  return sent.result as bigint;
}

/** Read-only: the connected wallet's real share balance, read directly from the vault
 * contract's own persistent storage. No wallet signature needed to read. */
export async function getRealShareBalance(user: string): Promise<bigint> {
  const client = await getClient(VAULT_CONTRACT_ID);
  const tx = await (client as any).balance_of({ user });
  return tx.result as bigint;
}

/** Read-only: converts a real share amount into its real underlying-asset value using the
 * vault's own on-chain virtual-offset math (simulated against real on-chain totals, not
 * reimplemented locally). */
export async function convertRealSharesToAssets(shares: bigint): Promise<bigint> {
  const client = await getClient(VAULT_CONTRACT_ID);
  const tx = await (client as any).convert_to_assets({ shares });
  return tx.result as bigint;
}

/** Read-only: previews how many shares a given real asset amount would mint right now,
 * simulated against the vault's real on-chain totals. */
export async function convertRealAssetsToShares(assets: bigint): Promise<bigint> {
  const client = await getClient(VAULT_CONTRACT_ID);
  const tx = await (client as any).convert_to_shares({ assets });
  return tx.result as bigint;
}
