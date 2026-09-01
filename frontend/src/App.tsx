import React, { useState } from 'react';
import './index.css';

// DEMO UI — there is no live backend/deployed vault behind this yet. No wallet is
// actually connected and no deposit is actually submitted. The share-conversion math
// below (Yearn V3 virtual-offset formula) IS the same real formula the vault contract
// uses, so the calculator is a genuine preview of that math — but nothing here talks to
// a real, deployed contract, and there is no strategy routing (Blend/Phoenix) at all yet.

export const App: React.FC = () => {
  const [walletConnected, setWalletConnected] = useState(false);
  const [balance, setBalance] = useState(0);
  const [vaultShares, setVaultShares] = useState(0);
  const [depositAmount, setDepositAmount] = useState('');
  const [loading, setLoading] = useState(false);

  const [logs, setLogs] = useState<string[]>([
    '[DEMO] No wallet connected, no vault contract deployed — everything below is a UI mockup.',
  ]);

  const handleDeposit = (e: React.FormEvent) => {
    e.preventDefault();
    const val = parseFloat(depositAmount);
    if (isNaN(val) || val <= 0) return;

    setLoading(true);
    setLogs(prev => [...prev, `[DEMO] Walking through the "deposit" UI for ${val} XLM — nothing is signed or submitted.`]);

    setTimeout(() => {
      const mintedShares = (val * (vaultShares + 1000)) / (balance + 1000);
      setBalance(prev => prev + val);
      setVaultShares(prev => prev + mintedShares);
      setLoading(false);
      setLogs(prev => [
        ...prev,
        `[DEMO] Computed ${mintedShares.toFixed(2)} vXLM using the real share formula, shown locally only. No Soroban transaction happened — there is no deployed vault or strategy router this UI talks to yet.`,
      ]);
      setDepositAmount('');
    }, 600);
  };

  const numDeposit = parseFloat(depositAmount);
  const estimatedShares = !isNaN(numDeposit) && numDeposit > 0
    ? ((numDeposit * (vaultShares + 1000)) / (balance + 1000)).toFixed(2)
    : '0.00';

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#06120e', color: '#e2e8f0' }}>
      <div style={{ background: 'linear-gradient(135deg, #b45309, #92400e)', color: '#fff', padding: '0.65rem 1.5rem', fontSize: '0.85rem', fontWeight: 600, textAlign: 'center' }}>
        ⚠ DEMO MODE — no wallet connected, no deployed vault, no strategy routing exists yet. See README for current status.
      </div>
      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem 1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>

        {/* Navigation Bar */}
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: '#0c221a', padding: '1rem 1.5rem', borderRadius: '10px', border: '1px solid #163e30' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <h1 style={{ fontSize: '1.25rem', fontWeight: 700, margin: 0, color: '#10b981' }}>soroban-yield-vault</h1>
            <span style={{ fontSize: '0.75rem', background: 'rgba(180, 83, 9, 0.2)', color: '#fbbf24', padding: '0.2rem 0.5rem', borderRadius: '4px', border: '1px solid rgba(180, 83, 9, 0.4)', fontWeight: 600 }}>
              No Backend Deployed
            </span>
          </div>

          <button
            onClick={() => setWalletConnected(!walletConnected)}
            style={{ padding: '0.5rem 1rem', background: '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: 600, fontSize: '0.85rem' }}
          >
            {walletConnected ? 'Wallet (demo toggle only)' : 'Connect Wallet (not implemented)'}
          </button>
        </header>

        {/* 2-Column Split Layout */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>

          {/* Left Column: Action Card */}
          <section style={{ background: '#0c221a', padding: '1.75rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <h2 style={{ fontSize: '1rem', fontWeight: 600, margin: 0, color: '#f8fafc' }}>Deposit Demo (Not a Real Vault)</h2>

            <form onSubmit={handleDeposit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#94a3b8', marginBottom: '0.4rem' }}>Deposit Amount (XLM)</label>
                <input
                  type="number"
                  placeholder="1000"
                  value={depositAmount}
                  onChange={(e) => setDepositAmount(e.target.value)}
                  style={{ width: '100%', padding: '0.75rem 1rem', background: '#06120e', border: '1px solid #1f503e', color: '#f8fafc', borderRadius: '6px', fontSize: '0.9rem', outline: 'none', boxSizing: 'border-box' }}
                />
              </div>

              {/* Dynamic ERC-4626 Calculator Box — uses the real vault share formula, computed locally */}
              <div style={{ background: '#06120e', padding: '0.85rem 1rem', borderRadius: '6px', border: '1px solid #1f503e', fontSize: '0.8rem' }}>
                <div style={{ color: '#10b981', fontWeight: 600 }}>Share Calculator (real formula, local preview only):</div>
                <div style={{ color: '#94a3b8', marginTop: '0.2rem' }}>
                  Would mint: <strong style={{ color: '#f8fafc' }}>{estimatedShares} vXLM</strong> — using the vault contract's actual virtual-offset math, run here in your browser, not on-chain.
                </div>
              </div>

              <button
                type="submit"
                disabled={loading}
                style={{ padding: '0.85rem', background: loading ? '#1f503e' : '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: loading ? 'wait' : 'pointer', fontWeight: 600, fontSize: '0.9rem' }}
              >
                {loading ? 'Running demo...' : 'Run Demo Deposit (Not a Real Transaction)'}
              </button>
            </form>
          </section>

          {/* Right Column: Log */}
          <section style={{ background: '#06120e', padding: '1.5rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column' }}>
            <h2 style={{ fontSize: '0.95rem', fontWeight: 600, color: '#94a3b8', margin: '0 0 1rem 0' }}>
              Demo Walkthrough Log
            </h2>

            <div style={{ background: '#020907', padding: '1.25rem', borderRadius: '8px', border: '1px solid #0c221a', fontFamily: 'Fira Code, monospace', fontSize: '0.8rem', color: '#10b981', flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
              {logs.map((log, idx) => (
                <div key={idx} style={{ color: '#fbbf24' }}>
                  {log}
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
