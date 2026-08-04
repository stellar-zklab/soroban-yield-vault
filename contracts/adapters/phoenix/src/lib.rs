#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct PhoenixAdapterContract;

#[contractimpl]
impl PhoenixAdapterContract {
    pub fn version(_env: Env) -> u32 { 1 }
}
