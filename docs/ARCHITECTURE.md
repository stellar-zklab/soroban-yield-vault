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
| strategy_router | Dynamic allocation engine across strategies |
| adapter_blend | Strategy adapter for Blend Capital lending pools |
| adapter_phoenix | Strategy adapter for Phoenix DEX yield farming |
