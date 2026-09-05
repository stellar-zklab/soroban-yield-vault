#![no_std]
//! Placeholder only. There is no real strategy allocation logic here yet — no comparison
//! of Blend vs Phoenix yields, no rebalancing, no routing of vault deposits anywhere.
//! `version()` exists solely so the workspace has something to compile and deploy while
//! this is being built. The vault's `deposit()` currently doesn't call into this contract
//! at all — deposited funds just sit in the vault contract's own balance.
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct StrategyRouterContract;

#[contractimpl]
impl StrategyRouterContract {
    pub fn version(_env: Env) -> u32 { 1 }
}

mod test;
