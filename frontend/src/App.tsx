import React, { useState } from 'react';
import './index.css';
import {
  connectWallet,
  depositRealXlm,
  withdrawRealShares,
  getRealShareBalance,
  convertRealAssetsToShares,
  VAULT_CONTRACT_ID,
  NATIVE_TOKEN_ID,
  STROOPS_PER_XLM,
  FreighterNotDetectedError,
} from './soroban';

// Real integration — this UI talks to the real deployed `vault` contract on Stellar
// testnet (see ../deployments/testnet.json). Deposits pull real native XLM from the
// connected wallet and mint real vault shares using the vault's own on-chain virtual-
// offset math. This deployed vault is wired to a real strategy_router + adapter-blend
// (see deployments/testnet.json's notes — set_router() was called as part of the
// 2026-09-05 redeploy), which supplies deposits to a real, live Blend Protocol V2
// lending pool on testnet, so deposited assets do earn real accrued interest. What's
// NOT real: contracts/adapters/phoenix remains a deliberate stub — see the README's
// Current Status for why that one's held off rather than built silently.

export const App: React.FC = () => {
  const [address, setAddress] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [shareBalance, setShareBalance] = useState<bigint | null>(null);
  const [depositAmount, setDepositAmount] = useState('');
  const [withdrawShares, setWithdrawShares] = useState('');
  const [previewShares, setPreviewShares] = useState<bigint | null>(null);
  const [loading, setLoading] = useState(false);

  const [logs, setLogs] = useState<string[]>([
    `[REAL] This app talks to the real deployed vault contract ${VAULT_CONTRACT_ID} on Stellar testnet — deposits and withdrawals are real signed transactions moving real testnet XLM (token ${NATIVE_TOKEN_ID}).`,
    '[NOTE] This vault is wired to a real strategy_router + adapter-blend supplying to a live Blend Protocol V2 pool on testnet — deposits earn real accrued interest. Only adapter-phoenix remains a deliberate stub (see README).',
  ]);

  const log = (msg: string) => setLogs((prev) => [...prev, msg]);

  const refreshBalance = async (addr: string) => {
    const bal = await getRealShareBalance(addr);
    setShareBalance(bal);
  };

  const handleConnect = async () => {
    setConnecting(true);
    try {
      const addr = await connectWallet();
      setAddress(addr);
      log(`[REAL] Wallet connected: ${addr}`);
      await refreshBalance(addr);
    } catch (err) {
      if (err instanceof FreighterNotDetectedError) {
        log(`[ERROR] ${err.message}`);
      } else {
        log(`[ERROR] Wallet connection failed: ${(err as Error).message}`);
      }
    } finally {
      setConnecting(false);
    }
  };

  const handleDepositAmountChange = async (val: string) => {
    setDepositAmount(val);
    const num = parseFloat(val);
    if (isNaN(num) || num <= 0) {
      setPreviewShares(null);
      return;
    }
    try {
      const assets = BigInt(Math.round(num * Number(STROOPS_PER_XLM)));
      const shares = await convertRealAssetsToShares(assets);
      setPreviewShares(shares);
    } catch {
      setPreviewShares(null);
    }
  };

  const handleDeposit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!address) return;
    const val = parseFloat(depositAmount);
    if (isNaN(val) || val <= 0) return;

    setLoading(true);
    log(`[REAL] Signing a real deposit of ${val} XLM into the vault. Freighter will ask you to review and sign.`);
    try {
      const shares = await depositRealXlm(address, val);
      log(`[REAL] Transaction confirmed. Minted ${shares.toString()} real vault shares on testnet.`);
      setDepositAmount('');
      setPreviewShares(null);
      await refreshBalance(address);
    } catch (err) {
      log(`[ERROR] Deposit failed: ${(err as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  const handleWithdraw = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!address) return;
    let shares: bigint;
    try {
      shares = BigInt(withdrawShares);
    } catch {
      return;
    }
    if (shares <= 0n) return;

    setLoading(true);
    log(`[REAL] Signing a real withdrawal of ${shares.toString()} vault shares. Freighter will ask you to review and sign.`);
    try {
      const assets = await withdrawRealShares(address, shares);
      log(`[REAL] Transaction confirmed. Redeemed for ${(Number(assets) / Number(STROOPS_PER_XLM)).toFixed(7)} real XLM on testnet.`);
      setWithdrawShares('');
      await refreshBalance(address);
    } catch (err) {
      log(`[ERROR] Withdraw failed: ${(err as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#06120e', color: '#e2e8f0' }}>
      <div style={{ background: 'linear-gradient(135deg, #047857, #065f46)', color: '#fff', padding: '0.65rem 1.5rem', fontSize: '0.85rem', fontWeight: 600, textAlign: 'center' }}>
        ✓ REAL vault contract on testnet — deposits earn real yield via a live Blend Protocol V2 pool. Only the Phoenix adapter is still a deliberate stub, see README.
      </div>
      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem 1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>

        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: '#0c221a', padding: '1rem 1.5rem', borderRadius: '10px', border: '1px solid #163e30' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <h1 style={{ fontSize: '1.25rem', fontWeight: 700, margin: 0, color: '#10b981' }}>soroban-yield-vault</h1>
            <span style={{ fontSize: '0.75rem', background: 'rgba(5, 150, 105, 0.2)', color: '#34d399', padding: '0.2rem 0.5rem', borderRadius: '4px', border: '1px solid rgba(5, 150, 105, 0.4)', fontWeight: 600 }}>
              Real Vault Deployed
            </span>
          </div>

          <button
            onClick={handleConnect}
            disabled={connecting || !!address}
            style={{ padding: '0.5rem 1rem', background: address ? '#1f503e' : '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: address ? 'default' : 'pointer', fontWeight: 600, fontSize: '0.85rem' }}
          >
            {address ? `${address.slice(0, 6)}...${address.slice(-4)}` : connecting ? 'Connecting...' : 'Connect Real Wallet'}
          </button>
        </header>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>

          <section style={{ background: '#0c221a', padding: '1.75rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <h2 style={{ fontSize: '1rem', fontWeight: 600, margin: 0, color: '#f8fafc' }}>Real Deposit / Withdraw</h2>

            {shareBalance !== null && (
              <div style={{ fontSize: '0.85rem', color: '#94a3b8' }}>
                Your real share balance: <strong style={{ color: '#f8fafc' }}>{shareBalance.toString()} vXLM</strong>
              </div>
            )}

            <form onSubmit={handleDeposit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#94a3b8', marginBottom: '0.4rem' }}>Deposit Amount (real XLM)</label>
                <input
                  type="number"
                  placeholder="10"
                  value={depositAmount}
                  onChange={(e) => handleDepositAmountChange(e.target.value)}
                  disabled={!address}
                  style={{ width: '100%', padding: '0.75rem 1rem', background: '#06120e', border: '1px solid #1f503e', color: '#f8fafc', borderRadius: '6px', fontSize: '0.9rem', outline: 'none', boxSizing: 'border-box' }}
                />
              </div>

              {previewShares !== null && (
                <div style={{ background: '#06120e', padding: '0.85rem 1rem', borderRadius: '6px', border: '1px solid #1f503e', fontSize: '0.8rem' }}>
                  <div style={{ color: '#10b981', fontWeight: 600 }}>Real on-chain preview (simulated against real totals):</div>
                  <div style={{ color: '#94a3b8', marginTop: '0.2rem' }}>
                    Would mint: <strong style={{ color: '#f8fafc' }}>{previewShares.toString()} vXLM</strong>
                  </div>
                </div>
              )}

              <button
                type="submit"
                disabled={loading || !address}
                style={{ padding: '0.85rem', background: loading || !address ? '#1f503e' : '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: loading || !address ? 'not-allowed' : 'pointer', fontWeight: 600, fontSize: '0.9rem' }}
              >
                {!address ? 'Connect Wallet First' : loading ? 'Signing...' : 'Deposit (Real Transaction)'}
              </button>
            </form>

            <form onSubmit={handleWithdraw} style={{ display: 'flex', flexDirection: 'column', gap: '1rem', borderTop: '1px solid #163e30', paddingTop: '1.25rem' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#94a3b8', marginBottom: '0.4rem' }}>Withdraw Shares (vXLM)</label>
                <input
                  type="number"
                  placeholder="100"
                  value={withdrawShares}
                  onChange={(e) => setWithdrawShares(e.target.value)}
                  disabled={!address}
                  style={{ width: '100%', padding: '0.75rem 1rem', background: '#06120e', border: '1px solid #1f503e', color: '#f8fafc', borderRadius: '6px', fontSize: '0.9rem', outline: 'none', boxSizing: 'border-box' }}
                />
              </div>
              <button
                type="submit"
                disabled={loading || !address}
                style={{ padding: '0.85rem', background: loading || !address ? '#1f503e' : '#b45309', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: loading || !address ? 'not-allowed' : 'pointer', fontWeight: 600, fontSize: '0.9rem' }}
              >
                {!address ? 'Connect Wallet First' : loading ? 'Signing...' : 'Withdraw (Real Transaction)'}
              </button>
            </form>
          </section>

          <section style={{ background: '#06120e', padding: '1.5rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column' }}>
            <h2 style={{ fontSize: '0.95rem', fontWeight: 600, color: '#94a3b8', margin: '0 0 1rem 0' }}>
              Activity Log
            </h2>

            <div style={{ background: '#020907', padding: '1.25rem', borderRadius: '8px', border: '1px solid #0c221a', fontFamily: 'Fira Code, monospace', fontSize: '0.8rem', color: '#10b981', flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
              {logs.map((l, idx) => (
                <div key={idx} style={{ color: l.startsWith('[ERROR]') ? '#f87171' : l.startsWith('[REAL]') ? '#10b981' : '#fbbf24' }}>
                  {l}
                </div>
              ))}
            </div>
          </section>

        </div>

      </div>
    </div>
  );
};
export default App;
