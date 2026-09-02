# Strategy Adapter Guide

## Overview
Adapters normalize interactions with external Stellar DeFi protocols under a unified interface.

## Adapter Interface Standard
```rust
pub trait StrategyAdapter {
    fn deposit(env: Env, amount: i128) -> i128;
    fn withdraw(env: Env, amount: i128) -> i128;
    fn harvest(env: Env) -> i128;
    fn total_underlying(env: Env) -> i128;
}
```
