#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey { Admin, Asset, StrategyRouter, TotalShares, Balance(Address) }

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

    pub fn deposit(env: Env, caller: Address, amount: i128) -> i128 {
        caller.require_auth();
        assert!(amount > 0, "amount must be positive");
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_assets = Self::total_assets(env.clone());

        let shares = if total_shares == 0 || total_assets == 0 { amount } else { amount * total_shares / total_assets };

        soroban_sdk::token::TokenClient::new(&env, &asset)
            .transfer(&caller, &env.current_contract_address(), &amount);

        let user_shares: i128 = env.storage().persistent().get(&DataKey::Balance(caller.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(caller.clone()), &(user_shares + shares));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares + shares));

        env.events().publish((symbol_short!("vault"), symbol_short!("deposit")), (caller, amount, shares));
        shares
    }

    pub fn withdraw(env: Env, caller: Address, shares: i128) -> i128 {
        caller.require_auth();
        let user_shares: i128 = env.storage().persistent().get(&DataKey::Balance(caller.clone())).unwrap_or(0);
        assert!(user_shares >= shares, "insufficient shares");

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap();
        let total_assets = Self::total_assets(env.clone());
        let amount = shares * total_assets / total_shares;

        env.storage().persistent().set(&DataKey::Balance(caller.clone()), &(user_shares - shares));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - shares));

        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        soroban_sdk::token::TokenClient::new(&env, &asset)
            .transfer(&env.current_contract_address(), &caller, &amount);

        env.events().publish((symbol_short!("vault"), symbol_short!("withdraw")), (caller, amount, shares));
        amount
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
