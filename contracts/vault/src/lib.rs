#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env};

pub const VIRTUAL_OFFSET: i128 = 1000;

#[contracttype]
pub enum DataKey {
    TotalAssets,
    TotalShares,
    UserShare(Address),
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn convert_to_shares(env: Env, assets: i128) -> i128 {
        let total_assets: i128 = env.storage().instance().get(&DataKey::TotalAssets).unwrap_or(0);
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        if total_assets == 0 || total_shares == 0 {
            return assets;
        }

        // Yearn V3 Virtual Offset Inflation Protection Math: (A * (S_total + 1000)) / (A_total + 1000)
        (assets * (total_shares + VIRTUAL_OFFSET)) / (total_assets + VIRTUAL_OFFSET)
    }

    pub fn convert_to_assets(env: Env, shares: i128) -> i128 {
        let total_assets: i128 = env.storage().instance().get(&DataKey::TotalAssets).unwrap_or(0);
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        if total_shares == 0 {
            return shares;
        }

        (shares * (total_assets + VIRTUAL_OFFSET)) / (total_shares + VIRTUAL_OFFSET)
    }

    pub fn deposit(env: Env, caller: Address, assets: i128) -> i128 {
        caller.require_auth();
        if assets <= 0 {
            panic!("Deposit amount must be positive");
        }

        let shares = Self::convert_to_shares(env.clone(), assets);

        let total_assets: i128 = env.storage().instance().get(&DataKey::TotalAssets).unwrap_or(0);
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let user_shares: i128 = env.storage().persistent().get(&DataKey::UserShare(caller.clone())).unwrap_or(0);

        env.storage().instance().set(&DataKey::TotalAssets, &(total_assets + assets));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares + shares));
        
        let key = DataKey::UserShare(caller.clone());
        env.storage().persistent().set(&key, &(user_shares + shares));
        env.storage().persistent().extend_ttl(&key, 172800, 5184000);

        env.events().publish(
            (symbol_short!("deposit"), caller),
            (assets, shares),
        );

        shares
    }
}
