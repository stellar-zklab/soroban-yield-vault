import React, { useEffect, useState } from 'react';
import './index.css';
import {
  connectWallet,
  depositRealXlm,
  withdrawRealShares,
  getRealShareBalance,
  convertRealAssetsToShares,
  getStrategyAllocations,
  getRouterTotalAssets,
  StrategyAllocation,
  VAULT_CONTRACT_ID,
  STRATEGY_ROUTER_CONTRACT_ID,
  NATIVE_TOKEN_ID,
  STROOPS_PER_XLM,
} from './soroban';

const formatXlm = (stroops: bigint): string => (Number(stroops) / Number(STROOPS_PER_XLM)).toLocaleString(undefined, { maximumFractionDigits: 2 });

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

  const [strategies, setStrategies] = useState<StrategyAllocation[] | null>(null);
  const [routerTotalAssets, setRouterTotalAssets] = useState<bigint | null>(null);
  const [strategiesLoading, setStrategiesLoading] = useState(false);
  const [strategiesError, setStrategiesError] = useState<string | null>(null);

  const [logs, setLogs] = useState<string[]>([
    `[REAL] This app talks to the real deployed vault contract ${VAULT_CONTRACT_ID} on Stellar testnet — deposits and withdrawals are real signed transactions moving real testnet XLM (token ${NATIVE_TOKEN_ID}).`,
    '[NOTE] This vault is wired to a real strategy_router + adapter-blend supplying to a live Blend Protocol V2 pool on testnet — deposits earn real accrued interest. Only adapter-phoenix remains a deliberate stub (see README).',
  ]);

  const log = (msg: string) => setLogs((prev) => [...prev, msg]);

  const refreshBalance = async (addr: string) => {
    const bal = await getRealShareBalance(addr);
    setShareBalance(bal);
  };

  // Real contract reads, no wallet connection needed — the strategy router's allocation
  // state is public data. Loaded on mount so the table is visible before anyone connects.
  const loadStrategies = async () => {
    setStrategiesLoading(true);
    setStrategiesError(null);
    try {
      const [allocations, total] = await Promise.all([getStrategyAllocations(), getRouterTotalAssets()]);
      setStrategies(allocations);
      setRouterTotalAssets(total);
    } catch (err) {
      setStrategiesError((err as Error).message);
    } finally {
      setStrategiesLoading(false);
    }
  };

  useEffect(() => {
    loadStrategies();
  }, []);

  const handleConnect = async () => {
    setConnecting(true);
    try {
      const addr = await connectWallet();
      setAddress(addr);
      log(`[REAL] Wallet connected: ${addr}`);
      await refreshBalance(addr);
    } catch (err) {
      log(`[ERROR] Wallet connection failed: ${(err as Error).message}`);
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
    log(`[REAL] Signing a real deposit of ${val} XLM into the vault. Your wallet will ask you to review and sign.`);
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
    log(`[REAL] Signing a real withdrawal of ${shares.toString()} vault shares. Your wallet will ask you to review and sign.`);
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
        ✓ REAL vault contract on testnet — deposits earn real yield via a live Blend Protocol V2 pool. Only the Phoenix adapter is still a deliberate stub, see{' '}
        <a
          href="https://github.com/stellar-zklab/soroban-yield-vault/blob/main/README.md"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: '#fff', textDecoration: 'underline' }}
        >
          README
        </a>.
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

        <section style={{ background: '#0c221a', padding: '1.75rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
            <h2 style={{ fontSize: '1rem', fontWeight: 600, margin: 0, color: '#f8fafc' }}>Strategy Allocation</h2>
            <button
              onClick={loadStrategies}
              disabled={strategiesLoading}
              style={{ background: 'none', border: 'none', color: '#34d399', cursor: strategiesLoading ? 'default' : 'pointer', fontSize: '0.75rem', fontWeight: 600, padding: 0 }}
            >
              {strategiesLoading ? 'Refreshing...' : '↻ Refresh'}
            </button>
          </div>
          <p style={{ margin: 0, fontSize: '0.78rem', color: '#94a3b8' }}>
            Real, live reads from <code style={{ color: '#6ee7b7' }}>strategy_router</code> ({STRATEGY_ROUTER_CONTRACT_ID.slice(0, 6)}...{STRATEGY_ROUTER_CONTRACT_ID.slice(-4)}) —
            every registered strategy's real max-debt cap and real current allocation, not a cached snapshot.
          </p>

          {strategiesError && (
            <div style={{ background: '#2a0f0f', border: '1px solid #7f1d1d', color: '#fca5a5', padding: '0.75rem 1rem', borderRadius: '6px', fontSize: '0.8rem' }}>
              Failed to load strategy allocation: {strategiesError}
            </div>
          )}

          {!strategiesError && strategies && strategies.length === 0 && (
            <div style={{ color: '#94a3b8', fontSize: '0.85rem' }}>No strategies registered on the router yet.</div>
          )}

          {!strategiesError && strategies && strategies.length > 0 && (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.85rem' }}>
                <thead>
                  <tr style={{ textAlign: 'left', color: '#94a3b8', fontSize: '0.75rem', textTransform: 'uppercase', letterSpacing: '0.03em' }}>
                    <th style={{ padding: '0 0 0.6rem 0', fontWeight: 600 }}>Strategy</th>
                    <th style={{ padding: '0 0 0.6rem 0', fontWeight: 600 }}>Allocation</th>
                    <th style={{ padding: '0 0 0.6rem 0', fontWeight: 600 }}>Amount</th>
                    <th style={{ padding: '0 0 0.6rem 0', fontWeight: 600 }}>Cap</th>
                    <th style={{ padding: '0 0 0.6rem 0', fontWeight: 600 }}>Headroom</th>
                  </tr>
                </thead>
                <tbody>
                  {strategies.map((s) => {
                    const total = routerTotalAssets ?? 0n;
                    const pct = total > 0n ? (Number(s.debt) / Number(total)) * 100 : 0;
                    const headroom = s.maxDebt - s.debt;
                    return (
                      <tr key={s.address} style={{ borderTop: '1px solid #163e30' }}>
                        <td style={{ padding: '0.7rem 0', color: '#f8fafc', fontWeight: 500 }}>{s.label}</td>
                        <td style={{ padding: '0.7rem 0' }}>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <div style={{ width: '60px', height: '6px', background: '#163e30', borderRadius: '3px', overflow: 'hidden' }}>
                              <div style={{ width: `${Math.min(pct, 100)}%`, height: '100%', background: '#10b981' }} />
                            </div>
                            <span style={{ color: '#94a3b8' }}>{total > 0n ? `${pct.toFixed(1)}%` : '—'}</span>
                          </div>
                        </td>
                        <td style={{ padding: '0.7rem 0', color: '#f8fafc' }}>{formatXlm(s.debt)} XLM</td>
                        <td style={{ padding: '0.7rem 0', color: '#94a3b8' }}>{formatXlm(s.maxDebt)} XLM</td>
                        <td style={{ padding: '0.7rem 0', color: '#94a3b8' }}>{formatXlm(headroom)} XLM free</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {!strategiesError && strategies && strategies.length > 0 && strategies.every((s) => s.debt === 0n) && (
            <div style={{ fontSize: '0.78rem', color: '#fbbf24', background: 'rgba(251, 191, 36, 0.08)', border: '1px solid rgba(251, 191, 36, 0.25)', borderRadius: '6px', padding: '0.6rem 0.85rem' }}>
              No funds allocated to any strategy yet — deposits currently sit idle until an admin allocates them. This is a real zero, not a loading placeholder.
            </div>
          )}
        </section>

      </div>
    </div>
  );
};
export default App;
