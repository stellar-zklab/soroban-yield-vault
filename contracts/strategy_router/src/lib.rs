#![no_std]
//! Multi-strategy debt allocator, modeled on Yearn V3's real vault/strategy architecture:
//! the router can hold funds across several strategy adapters at once, each with an
//! admin-set `max_debt` cap, and an explicit `update_debt` call moves real funds toward a
//! `desired_debt` target for one strategy at a time. Allocation is a deliberate admin (or
//! future keeper/allocator) decision, not an automatic side effect of every deposit — this
//! deliberately mirrors Yearn V3's separation of "receiving deposits" from "capital
//! allocation," rather than this contract's previous single-strategy, auto-deploy-on-
//! deposit design. See Yearn's own docs: a manager calls `update_debt(strategy,
//! desired_debt)` bounded by a per-strategy `max_debt`, and a strategy's debt can never be
//! lowered while it holds unrealized losses (this contract doesn't yet track unrealized
//! loss the way Yearn's V3 vaults do — that's real future work, not implied here).
use soroban_sdk::{contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, Vec};

#[contractclient(name = "Client")]
#[allow(dead_code)] // only used to generate `Client`; the trait itself is never called directly
pub trait StrategyAdapter {
    fn deposit(env: Env, caller: Address, amount: i128);
    fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128;
    fn total_value(env: Env) -> i128;
}

#[contracttype]
pub enum DataKey {
    Admin,
    Controller,
    Asset,
    /// Registered strategies, in registration order — this order also doubles as the
    /// withdrawal queue (see `withdraw`), matching Yearn's own "default queue" concept.
    Strategies,
    MaxDebt(Address),
    /// Bookkeeping only: how much this router has told a strategy to hold via
    /// `update_debt`. NOT the source of truth for a strategy's real current value — that's
    /// always `Client::total_value()`, read live in `total_assets`. A strategy's real value
    /// can (and should) exceed its `CurrentDebt` once it starts earning yield.
    CurrentDebt(Address),
}

#[contract]
pub struct StrategyRouterContract;

#[contractimpl]
impl StrategyRouterContract {
    /// `admin` is a human-controlled deploy-time identity, used for configuration changes
    /// like `add_strategy`/`update_debt` — required so a real keypair can actually
    /// authorize this call from the CLI. `controller` is the vault's own contract address;
    /// it's a DIFFERENT address deliberately, because a contract address can only
    /// self-authorize when IT is the one directly invoking, which a human deployer calling
    /// initialize() from the CLI never is. deposit()/withdraw() are gated on `controller`,
    /// not `admin` — this router takes fund-moving instructions from exactly one vault
    /// contract, never from a human keypair.
    pub fn initialize(env: Env, admin: Address, controller: Address, asset: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Controller, &controller);
        env.storage().instance().set(&DataKey::Asset, &asset);
        env.storage()
            .instance()
            .set(&DataKey::Strategies, &Vec::<Address>::new(&env));
    }

    /// (Admin only) Registers a new strategy adapter with zero `max_debt` — it won't
    /// receive any real allocation until `set_max_debt_for_strategy` explicitly raises its
    /// cap, matching Yearn V3's real `add_strategy` → `update_max_debt_for_strategy`
    /// sequence rather than granting capacity implicitly on registration.
    pub fn add_strategy(env: Env, admin: Address, strategy: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        assert!(!Self::is_registered(&env, &strategy), "strategy already added");

        let mut strategies = Self::get_strategies(env.clone());
        strategies.push_back(strategy.clone());
        env.storage().instance().set(&DataKey::Strategies, &strategies);
        env.storage()
            .instance()
            .set(&DataKey::MaxDebt(strategy.clone()), &0i128);
        env.storage()
            .instance()
            .set(&DataKey::CurrentDebt(strategy.clone()), &0i128);

        env.events().publish((symbol_short!("strat_add"), strategy), ());
    }

    /// (Admin only) Removes a strategy from the active list and withdrawal queue. Refuses
    /// to remove one that still has debt allocated — call `update_debt(strategy, 0)` first
    /// to pull its funds back to idle, so removal can never strand allocated capital with
    /// no router-side record of it.
    pub fn remove_strategy(env: Env, admin: Address, strategy: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        assert!(
            Self::get_debt(env.clone(), strategy.clone()) == 0,
            "strategy still has debt allocated; call update_debt(strategy, 0) first"
        );

        let strategies = Self::get_strategies(env.clone());
        let mut updated = Vec::new(&env);
        for s in strategies.iter() {
            if s != strategy {
                updated.push_back(s);
            }
        }
        env.storage().instance().set(&DataKey::Strategies, &updated);
        env.storage().instance().remove(&DataKey::MaxDebt(strategy.clone()));
        env.storage().instance().remove(&DataKey::CurrentDebt(strategy.clone()));

        env.events().publish((symbol_short!("strat_rm"), strategy), ());
    }

    /// (Admin only) Sets the maximum this strategy may ever be allocated via `update_debt`.
    pub fn set_max_debt_for_strategy(env: Env, admin: Address, strategy: Address, max_debt: i128) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        assert!(Self::is_registered(&env, &strategy), "strategy not registered");
        assert!(max_debt >= 0, "max_debt must not be negative");

        env.storage().instance().set(&DataKey::MaxDebt(strategy.clone()), &max_debt);
        env.events()
            .publish((symbol_short!("maxdebt"), strategy), max_debt);
    }

    /// (Admin only) The actual Debt Allocator action: moves real funds so `strategy` ends
    /// up holding exactly `desired_debt` (bounded by its `max_debt`) — pulling from the
    /// router's idle balance and depositing to increase it, or withdrawing back to idle to
    /// decrease it. Returns the strategy's new current debt.
    pub fn update_debt(env: Env, admin: Address, strategy: Address, desired_debt: i128) -> i128 {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        assert!(Self::is_registered(&env, &strategy), "strategy not registered");
        assert!(desired_debt >= 0, "desired_debt must not be negative");

        let max_debt = Self::get_max_debt(env.clone(), strategy.clone());
        assert!(desired_debt <= max_debt, "desired_debt exceeds this strategy's max_debt");

        let current_debt = Self::get_debt(env.clone(), strategy.clone());
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();
        let token = soroban_sdk::token::TokenClient::new(&env, &asset);

        if desired_debt > current_debt {
            let increase = desired_debt - current_debt;
            let idle = token.balance(&this);
            assert!(idle >= increase, "insufficient idle balance to cover this allocation");
            token.transfer(&this, &strategy, &increase);
            Client::new(&env, &strategy).deposit(&this, &increase);
        } else if desired_debt < current_debt {
            let decrease = current_debt - desired_debt;
            Client::new(&env, &strategy).withdraw(&this, &decrease, &this);
        }

        env.storage()
            .instance()
            .set(&DataKey::CurrentDebt(strategy.clone()), &desired_debt);
        env.events()
            .publish((symbol_short!("debt_upd"), strategy), desired_debt);

        desired_debt
    }

    pub fn get_strategies(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Strategies)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_max_debt(env: Env, strategy: Address) -> i128 {
        env.storage().instance().get(&DataKey::MaxDebt(strategy)).unwrap_or(0)
    }

    pub fn get_debt(env: Env, strategy: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::CurrentDebt(strategy))
            .unwrap_or(0)
    }

    /// Deposits land in the router's own idle balance. Allocation to a strategy is now a
    /// deliberate, separate `update_debt` call rather than an automatic side effect of
    /// every deposit — see the module doc comment for why. No-ops if no strategy is even
    /// registered yet, leaving funds idle rather than reverting, so the vault can accept
    /// deposits before any real strategy exists.
    pub fn deposit(env: Env, caller: Address, amount: i128) {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");
        // Funds are already transferred to this contract by the caller (the vault) before
        // this call, matching this contract's existing convention — nothing further to do
        // here beyond the auth/amount checks; the funds simply sit idle until an admin's
        // `update_debt` call allocates them to a specific strategy.
    }

    /// Withdraws `amount` for the controller, pulling from idle first and then draining
    /// registered strategies in registration order (a simple withdrawal queue, matching
    /// Yearn's "default queue" concept) if idle alone isn't enough. Panics if the combined
    /// idle balance and every strategy's allocated debt still can't cover `amount` — a
    /// withdraw request that can't be satisfied is a real error, not a partial-success case.
    pub fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128 {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();
        let token = soroban_sdk::token::TokenClient::new(&env, &asset);
        let idle = token.balance(&this);

        if idle < amount {
            let mut shortfall = amount - idle;
            for strategy in Self::get_strategies(env.clone()).iter() {
                if shortfall == 0 {
                    break;
                }
                let current_debt = Self::get_debt(env.clone(), strategy.clone());
                if current_debt == 0 {
                    continue;
                }
                let pull = shortfall.min(current_debt);
                let received = Client::new(&env, &strategy).withdraw(&this, &pull, &this);
                env.storage()
                    .instance()
                    .set(&DataKey::CurrentDebt(strategy.clone()), &(current_debt - pull));
                shortfall -= received.min(shortfall);
            }
            assert!(
                shortfall == 0,
                "insufficient total funds across idle balance and all registered strategies"
            );
        }

        token.transfer(&this, &to, &amount);
        amount
    }

    /// Read-only: idle balance plus every registered strategy's own real, live
    /// `total_value()` — never a locally-tracked approximation, and never just the sum of
    /// `CurrentDebt` bookkeeping (which reflects allocated principal, not accrued yield).
    pub fn total_assets(env: Env) -> i128 {
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let idle = soroban_sdk::token::TokenClient::new(&env, &asset).balance(&env.current_contract_address());
        let mut deployed: i128 = 0;
        for strategy in Self::get_strategies(env.clone()).iter() {
            deployed += Client::new(&env, &strategy).total_value();
        }
        idle + deployed
    }

    fn is_registered(env: &Env, strategy: &Address) -> bool {
        Self::get_strategies(env.clone()).iter().any(|s| &s == strategy)
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

#[cfg(test)]
mod test;
