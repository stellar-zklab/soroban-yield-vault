# Architecture — soroban-yield-vault

## Overview

```
User → deposit() → Vault (ERC-4626) → StrategyRouter → Blend Adapter   → Blend Capital
                                                      → Phoenix Adapter → Phoenix DEX
```

## Smart Contracts

| Contract | Role |
|---|---|
| vault | ERC-4626 deposit / withdraw / share accounting |
| strategy_router | Forwards vault funds to a single configured strategy adapter and reports its real value back — not a multi-strategy allocator (yet); see `docs/STRATEGIES.md` |
| adapter_blend | Real strategy adapter for a live Blend Capital V2 lending pool — implemented and tested |
| adapter_phoenix | Strategy adapter for Phoenix DEX yield farming — not implemented yet |
