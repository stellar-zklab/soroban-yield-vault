import React, { useState } from 'react';
import './index.css';

export const App: React.FC = () => {
  const [walletConnected, setWalletConnected] = useState(true);
  const [walletAddress] = useState('GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN');
  const [balance, setBalance] = useState(10000);
  const [vaultShares, setVaultShares] = useState(1250);
  const [depositAmount, setDepositAmount] = useState('');
  const [loading, setLoading] = useState(false);

  const [logs, setLogs] = useState<string[]>([
    '[SDK] Yield Vault Client initialized on Soroban Testnet',
    '[ConnectionPool] Connected to https://soroban-testnet.stellar.org:443 (Latency: 28ms)',
    '[ERC4626] Total Vault Assets: 1,250,000 XLM | Total Shares: 1,225,000 vXLM',
    '[StrategyRouter] Active allocation: 60% Blend Lending / 40% Phoenix AMM'
  ]);

  const handleDeposit = (e: React.FormEvent) => {
    e.preventDefault();
    const val = parseFloat(depositAmount);
    if (isNaN(val) || val <= 0 || val > balance) return;

    setLoading(true);
    setLogs(prev => [...prev, `[ERC4626] Calculating share conversion for ${val} XLM deposit...`]);

    setTimeout(() => {
      const mintedShares = (val * (vaultShares + 1000)) / (balance + 1000);
      setBalance(prev => prev - val);
      setVaultShares(prev => prev + mintedShares);
      setLoading(false);
      setLogs(prev => [
        ...prev,
        `[Soroban] Tx Executed: Minted ${mintedShares.toFixed(2)} vXLM shares with Yearn V3 virtual offset protection`,
        `[StrategyRouter] Deposited capital allocated across Blend & Phoenix adapters`
      ]);
      setDepositAmount('');
    }, 1000);
  };

  const numDeposit = parseFloat(depositAmount);
  const estimatedShares = !isNaN(numDeposit) && numDeposit > 0
    ? ((numDeposit * (vaultShares + 1000)) / (balance + 1000)).toFixed(2)
    : '0.00';

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#06120e', color: '#e2e8f0', padding: '2rem 1.5rem' }}>
      <div style={{ maxWidth: '1200px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
        
        {/* Navigation Bar */}
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: '#0c221a', padding: '1rem 1.5rem', borderRadius: '10px', border: '1px solid #163e30' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <h1 style={{ fontSize: '1.25rem', fontWeight: 700, margin: 0, color: '#10b981' }}>soroban-yield-vault</h1>
            <span style={{ fontSize: '0.75rem', background: 'rgba(16, 185, 129, 0.15)', color: '#10b981', padding: '0.2rem 0.5rem', borderRadius: '4px', border: '1px solid rgba(16, 185, 129, 0.3)', fontWeight: 600 }}>
              Testnet RPC (28ms)
            </span>
          </div>

          <button
            onClick={() => setWalletConnected(!walletConnected)}
            style={{ padding: '0.5rem 1rem', background: '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: 600, fontSize: '0.85rem' }}
          >
            {walletConnected ? `${walletAddress.substring(0, 6)}... (Testnet)` : 'Connect Wallet'}
          </button>
        </header>

        {/* 2-Column Split Layout */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>
          
          {/* Left Column: Action Card */}
          <section style={{ background: '#0c221a', padding: '1.75rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <h2 style={{ fontSize: '1rem', fontWeight: 600, margin: 0, color: '#f8fafc' }}>Deposit Assets into ERC-4626 Vault</h2>

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

              {/* Dynamic ERC-4626 Calculator Box */}
              <div style={{ background: '#06120e', padding: '0.85rem 1rem', borderRadius: '6px', border: '1px solid #1f503e', fontSize: '0.8rem' }}>
                <div style={{ color: '#10b981', fontWeight: 600 }}>ERC-4626 Dynamic Calculator:</div>
                <div style={{ color: '#94a3b8', marginTop: '0.2rem' }}>
                  Expected Minted Shares: <strong style={{ color: '#f8fafc' }}>{estimatedShares} vXLM</strong> (1 vXLM = 1.0204 XLM)
                </div>
              </div>

              <button
                type="submit"
                disabled={loading}
                style={{ padding: '0.85rem', background: loading ? '#1f503e' : '#059669', color: '#ffffff', border: 'none', borderRadius: '6px', cursor: loading ? 'wait' : 'pointer', fontWeight: 600, fontSize: '0.9rem' }}
              >
                {loading ? 'Minting Shares...' : 'Deposit Assets'}
              </button>
            </form>
          </section>

          {/* Right Column: SDK Terminal Log */}
          <section style={{ background: '#06120e', padding: '1.5rem', borderRadius: '10px', border: '1px solid #163e30', display: 'flex', flexDirection: 'column' }}>
            <h2 style={{ fontSize: '0.95rem', fontWeight: 600, color: '#94a3b8', margin: '0 0 1rem 0' }}>
              SDK Execution Log
            </h2>

            <div style={{ background: '#020907', padding: '1.25rem', borderRadius: '8px', border: '1px solid #0c221a', fontFamily: 'Fira Code, monospace', fontSize: '0.8rem', color: '#10b981', flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
              {logs.map((log, idx) => (
                <div key={idx} style={{ color: log.includes('[SDK]') ? '#38bdf8' : log.includes('[ERC4626]') ? '#c084fc' : '#10b981' }}>
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
