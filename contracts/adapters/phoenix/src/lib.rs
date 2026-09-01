#![no_std]
//! Placeholder only. There is no real Phoenix DEX integration here yet — no swap,
//! liquidity provisioning, or yield-reporting logic, no CPI/cross-contract call into a
//! Phoenix pool. `version()` exists solely so the workspace has something to compile and
//! deploy while this is being built.
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct PhoenixAdapterContract;

#[contractimpl]
impl PhoenixAdapterContract {
    pub fn version(_env: Env) -> u32 { 1 }
}

mod test;
