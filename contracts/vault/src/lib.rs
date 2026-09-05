#![no_std]
use soroban_sdk::{contract, contractclient, contractimpl, contracttype, symbol_short, token::TokenClient, Address, Env};

pub const VIRTUAL_OFFSET: i128 = 1000;

#[contractclient(name = "RouterClient")]
pub trait StrategyRouter {
    fn deposit(env: Env, caller: Address, amount: i128);
    fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128;
    fn total_assets(env: Env) -> i128;
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    TotalShares,
    UserShare(Address),
    Router,
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

    /// (Admin only) Points the vault at a strategy_router so future deposits actually get
    /// deployed to earn yield instead of sitting idle. Existing idle funds already in the
    /// vault are NOT automatically swept to the router by this call — that's a deliberate,
    /// separate action (see docs), since silently moving already-deposited funds the
    /// moment an admin changes this setting would be a surprising side effect for a call
    /// that only looks like a configuration change.
    pub fn set_router(env: Env, admin: Address, router: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert_eq!(admin, stored_admin, "caller is not the admin");
        env.storage().instance().set(&DataKey::Router, &router);
    }

    /// The vault's real total managed assets: whatever it's still holding idle, plus
    /// whatever its strategy router reports as the real current value of deployed funds
    /// (including accrued yield) — computed live on every call, not read from a stored
    /// counter that would silently go stale the moment a strategy starts earning interest
    /// independently of vault-level deposit/withdraw calls.
    pub fn total_assets(env: Env) -> i128 {
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("not initialized");
        let idle = TokenClient::new(&env, &token).balance(&env.current_contract_address());
        let deployed = match env.storage().instance().get::<DataKey, Address>(&DataKey::Router) {
            Some(router) => RouterClient::new(&env, &router).total_assets(),
            None => 0,
        };
        idle + deployed
    }

    pub fn convert_to_shares(env: Env, assets: i128) -> i128 {
        let total_assets = Self::total_assets(env.clone());
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        if total_assets == 0 || total_shares == 0 {
            return assets;
        }

        // Yearn V3 Virtual Offset Inflation Protection Math: (A * (S_total + 1000)) / (A_total + 1000)
        (assets * (total_shares + VIRTUAL_OFFSET)) / (total_assets + VIRTUAL_OFFSET)
    }

    pub fn convert_to_assets(env: Env, shares: i128) -> i128 {
        let total_assets = Self::total_assets(env.clone());
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);

        if total_shares == 0 {
            return shares;
        }

        (shares * (total_assets + VIRTUAL_OFFSET)) / (total_shares + VIRTUAL_OFFSET)
    }

    /// Pulls `assets` of the vault's real token from `caller` before crediting shares — a
    /// deposit that only updated internal accounting without ever moving the underlying
    /// token would let a caller mint shares against assets the vault never actually holds.
    /// If a strategy router is configured, the deposited assets are immediately forwarded
    /// to it so they actually start earning yield rather than sitting idle — this must
    /// happen after computing `shares` from the pre-deposit share price, and after the
    /// vault has received the tokens itself, so total_assets() reflects a consistent state
    /// at each step rather than double-counting or momentarily under-counting funds that
    /// are in transit between the depositor, the vault, and the router.
    pub fn deposit(env: Env, caller: Address, assets: i128) -> i128 {
        caller.require_auth();
        if assets <= 0 {
            panic!("Deposit amount must be positive");
        }

        let shares = Self::convert_to_shares(env.clone(), assets);

        let token: Address = env.storage().instance().get(&DataKey::Token).expect("not initialized");
        let this = env.current_contract_address();
        TokenClient::new(&env, &token).transfer(&caller, &this, &assets);

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let user_shares: i128 = env.storage().persistent().get(&DataKey::UserShare(caller.clone())).unwrap_or(0);

        env.storage().instance().set(&DataKey::TotalShares, &(total_shares + shares));

        let key = DataKey::UserShare(caller.clone());
        env.storage().persistent().set(&key, &(user_shares + shares));
        env.storage().persistent().extend_ttl(&key, 172800, 5184000);

        if let Some(router) = env.storage().instance().get::<DataKey, Address>(&DataKey::Router) {
            TokenClient::new(&env, &token).transfer(&this, &router, &assets);
            RouterClient::new(&env, &router).deposit(&this, &assets);
        }

        env.events().publish(
            (symbol_short!("deposit"), caller),
            (assets, shares),
        );

        shares
    }

    /// Burns `shares` and pays out the corresponding real assets. Rejects a caller trying
    /// to redeem more shares than they actually hold — without this check, any caller could
    /// withdraw funds that other depositors put in, since shares only exist as one shared
    /// pool of vault-held tokens. If the vault's own idle balance can't cover the payout
    /// (the common case once deposits are actively deployed to a strategy), it pulls
    /// exactly the shortfall back from the router before paying out — never more than
    /// needed, so funds that don't need to move stay earning yield.
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

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - shares));
        env.storage().persistent().set(&key, &(user_shares - shares));
        env.storage().persistent().extend_ttl(&key, 172800, 5184000);

        let token: Address = env.storage().instance().get(&DataKey::Token).expect("not initialized");
        let this = env.current_contract_address();
        let idle = TokenClient::new(&env, &token).balance(&this);
        if idle < assets {
            let router: Address = env
                .storage()
                .instance()
                .get(&DataKey::Router)
                .expect("insufficient idle balance and no router configured to cover the shortfall");
            let shortfall = assets - idle;
            RouterClient::new(&env, &router).withdraw(&this, &shortfall, &this);
        }
        TokenClient::new(&env, &token).transfer(&this, &caller, &assets);

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
