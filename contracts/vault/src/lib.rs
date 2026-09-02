#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token::TokenClient, Address, Env};

pub const VIRTUAL_OFFSET: i128 = 1000;

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    TotalAssets,
    TotalShares,
    UserShare(Address),
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
    }

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

    /// Pulls `assets` of the vault's real token from `caller` before crediting shares — a
    /// deposit that only updated internal accounting without ever moving the underlying
    /// token would let a caller mint shares against assets the vault never actually holds.
    pub fn deposit(env: Env, caller: Address, assets: i128) -> i128 {
        caller.require_auth();
        if assets <= 0 {
            panic!("Deposit amount must be positive");
        }

        let shares = Self::convert_to_shares(env.clone(), assets);

        let token: Address = env.storage().instance().get(&DataKey::Token).expect("not initialized");
        TokenClient::new(&env, &token).transfer(&caller, &env.current_contract_address(), &assets);

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

    /// Burns `shares` and pays out the corresponding real assets. Rejects a caller trying
    /// to redeem more shares than they actually hold — without this check, any caller could
    /// withdraw funds that other depositors put in, since shares only exist as one shared
    /// pool of vault-held tokens.
    pub fn withdraw(env: Env, caller: Address, shares: i128) -> i128 {
        caller.require_auth();
        if shares <= 0 {
            panic!("Withdraw amount must be positive");
        }

        let key = DataKey::UserShare(caller.clone());
        let user_shares: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if shares > user_shares {
            panic!("Insufficient shares");
        }

        let assets = Self::convert_to_assets(env.clone(), shares);

        let total_assets: i128 = env.storage().instance().get(&DataKey::TotalAssets).unwrap_or(0);
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        env.storage().instance().set(&DataKey::TotalAssets, &(total_assets - assets));
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - shares));
        env.storage().persistent().set(&key, &(user_shares - shares));
        env.storage().persistent().extend_ttl(&key, 172800, 5184000);

        let token: Address = env.storage().instance().get(&DataKey::Token).expect("not initialized");
        TokenClient::new(&env, &token).transfer(&env.current_contract_address(), &caller, &assets);

        env.events().publish(
            (symbol_short!("withdraw"), caller),
            (assets, shares),
        );

        assets
    }

    pub fn balance_of(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&DataKey::UserShare(user)).unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
