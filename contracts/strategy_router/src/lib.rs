#![no_std]
//! Routes the vault's idle assets to a single configured strategy adapter (e.g.
//! adapter-blend) and reports that strategy's real current value back to the vault.
//!
//! This is deliberately single-strategy, not a real allocator across multiple strategies
//! comparing yields — that's real future work, not implied here. What it does do for real:
//! move real tokens to whichever adapter is currently set, and forward real deposit/
//! withdraw calls to it. The vault is this router's only caller (see `admin` below); this
//! isn't meant to be called directly by end users.
use soroban_sdk::{contract, contractimpl, contracttype, token::TokenClient, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
    Controller,
    Asset,
    Strategy,
}

#[contract]
pub struct StrategyRouterContract;

#[contractimpl]
impl StrategyRouterContract {
    /// `admin` is a human-controlled deploy-time identity, used for configuration changes
    /// like set_strategy — required so a real keypair can actually authorize this call
    /// from the CLI. `controller` is the vault's own contract address; it's a DIFFERENT
    /// address deliberately, because a contract address can only self-authorize when IT is
    /// the one directly invoking, which a human deployer calling initialize() never is.
    /// deposit()/withdraw() are gated on `controller`, not `admin` — this router takes
    /// fund-moving instructions from exactly one vault contract, never from a human keypair.
    pub fn initialize(env: Env, admin: Address, controller: Address, asset: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Controller, &controller);
        env.storage().instance().set(&DataKey::Asset, &asset);
    }

    /// (Admin only) Sets the active strategy adapter. Replacing an existing strategy does
    /// NOT migrate its funds automatically — this router has no way to know a new
    /// adapter's deposit/withdraw interface matches, so migrating is left as an explicit,
    /// separate admin action (withdraw from the old adapter, then set_strategy, then
    /// deposit into the new one) rather than attempted implicitly here.
    pub fn set_strategy(env: Env, admin: Address, strategy: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Strategy, &strategy);
    }

    pub fn get_strategy(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Strategy)
    }

    /// Forwards `amount` of the router's own real token balance (the vault must have
    /// already transferred it here) to the active strategy adapter and instructs it to
    /// deposit. No-ops if no strategy is set yet, leaving funds sitting in the router
    /// rather than reverting — lets the vault accept deposits before a strategy exists.
    pub fn deposit(env: Env, caller: Address, amount: i128) {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let strategy: Option<Address> = env.storage().instance().get(&DataKey::Strategy);
        let strategy = match strategy {
            Some(s) => s,
            None => return,
        };
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();

        TokenClient::new(&env, &asset).transfer(&this, &strategy, &amount);
        let client = adapter::Client::new(&env, &strategy);
        client.deposit(&this, &amount);
    }

    /// Instructs the active strategy adapter to withdraw `amount` and send it directly to
    /// `to`. Panics if no strategy is set — unlike deposit's no-op, a withdraw request
    /// with nowhere to pull funds from is a real error, not a valid pass-through state.
    pub fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128 {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let strategy: Address = env
            .storage()
            .instance()
            .get(&DataKey::Strategy)
            .expect("no strategy configured to withdraw from");
        let this = env.current_contract_address();

        let client = adapter::Client::new(&env, &strategy);
        client.withdraw(&this, &amount, &to)
    }

    /// Read-only: the active strategy's real current value, or 0 if none is configured.
    pub fn total_assets(env: Env) -> i128 {
        let strategy: Option<Address> = env.storage().instance().get(&DataKey::Strategy);
        match strategy {
            Some(s) => adapter::Client::new(&env, &s).total_value(),
            None => 0,
        }
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert_eq!(*caller, admin, "caller is not this router's admin");
    }

    fn require_controller(env: &Env, caller: &Address) {
        let controller: Address = env.storage().instance().get(&DataKey::Controller).unwrap();
        assert_eq!(*caller, controller, "caller is not this router's controller");
    }
}

/// Minimal client for any strategy adapter (adapter-blend, and future adapters) sharing
/// this same deposit/withdraw/total_value interface. Not tied to Blend specifically — a
/// future Phoenix or other adapter just needs to expose the same three entrypoints.
mod adapter {
    use soroban_sdk::{contractclient, Address, Env};

    #[contractclient(name = "Client")]
    #[allow(dead_code)] // only used to generate `Client`; the trait itself is never called directly
    pub trait StrategyAdapter {
        fn deposit(env: Env, caller: Address, amount: i128);
        fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128;
        fn total_value(env: Env) -> i128;
    }
}

#[cfg(test)]
mod test;
