#![no_std]
//! Placeholder only. There is no real Blend Capital integration here yet — no deposit,
//! withdraw, or yield-reporting logic, no CPI/cross-contract call into a Blend pool.
//! `version()` exists solely so the workspace has something to compile and deploy while
//! this is being built.
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct BlendAdapterContract;

#[contractimpl]
impl BlendAdapterContract {
    pub fn version(_env: Env) -> u32 { 1 }
}

mod test;
