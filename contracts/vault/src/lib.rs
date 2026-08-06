#![no_std]
//! soroban-yield-vault: Automated ERC-4626 Yield Optimizer on Soroban
//! Benchmarked against Yearn Finance V3 ERC-4626 Specification.

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env,
};

/// Yearn V3 Virtual Offset constant (10^3 = 1000) to mitigate ERC-4626 inflation attacks
pub const VIRTUAL_OFFSET: i128 = 1000;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Asset,
    StrategyRouter,
    TotalShares,
    Balance(Address),
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, asset: Address, router: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Asset, &asset);
        env.storage().instance().set(&DataKey::StrategyRouter, &router);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
    }

    /// Deposit underlying asset, mint shares with Yearn V3 virtual offset protection.
    pub fn deposit(env: Env, caller: Address, amount: i128) -> i128 {
        caller.require_auth();
        assert!(amount > 0, "amount must be positive");
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_assets = Self::total_assets(env.clone());

        // Yearn V3 Virtual Share Offset formula: S = (A * (S_total + 1000)) / (A_total + 1000)
        let shares = (amount * (total_shares + VIRTUAL_OFFSET)) / (total_assets + VIRTUAL_OFFSET);
        assert!(shares > 0, "shares minted must be positive");

        soroban_sdk::token::TokenClient::new(&env, &asset)
            .transfer(&caller, &env.current_contract_address(), &amount);

        let user_shares: i128 = env.storage().persistent().get(&DataKey::Balance(caller.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(caller.clone()), &(user_shares + shares));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares + shares));

        env.events().publish((symbol_short!("vault"), symbol_short!("deposit")), (caller, amount, shares));
        shares
    }

    /// Withdraw shares, burn shares, return underlying asset.
    pub fn withdraw(env: Env, caller: Address, shares: i128) -> i128 {
        caller.require_auth();
        let user_shares: i128 = env.storage().persistent().get(&DataKey::Balance(caller.clone())).unwrap_or(0);
        assert!(user_shares >= shares, "insufficient shares");

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap();
        let total_assets = Self::total_assets(env.clone());

        // Yearn V3 Virtual Share Offset formula: A = (S * (A_total + 1000)) / (S_total + 1000)
        let amount = (shares * (total_assets + VIRTUAL_OFFSET)) / (total_shares + VIRTUAL_OFFSET);

        env.storage().persistent().set(&DataKey::Balance(caller.clone()), &(user_shares - shares));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - shares));

        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        soroban_sdk::token::TokenClient::new(&env, &asset)
            .transfer(&env.current_contract_address(), &caller, &amount);

        env.events().publish((symbol_short!("vault"), symbol_short!("withdraw")), (caller, amount, shares));
        amount
    }

    // ─────────────────────────────────────────────
    // ERC-4626 STANDARD CONVERTER & VIEW FUNCTIONS
    // ─────────────────────────────────────────────

    /// Convert assets to expected shares (ERC-4626 view)
    pub fn convert_to_shares(env: Env, assets: i128) -> i128 {
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_assets = Self::total_assets(env);
        (assets * (total_shares + VIRTUAL_OFFSET)) / (total_assets + VIRTUAL_OFFSET)
    }

    /// Convert shares to expected assets (ERC-4626 view)
    pub fn convert_to_assets(env: Env, shares: i128) -> i128 {
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_assets = Self::total_assets(env);
        (shares * (total_assets + VIRTUAL_OFFSET)) / (total_shares + VIRTUAL_OFFSET)
    }

    pub fn total_assets(env: Env) -> i128 {
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        soroban_sdk::token::TokenClient::new(&env, &asset).balance(&env.current_contract_address())
    }

    pub fn get_share_balance(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Balance(user)).unwrap_or(0)
    }
}

mod test;
