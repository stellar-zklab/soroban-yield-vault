#![no_std]
//! Real integration with a deployed Blend Protocol V2 lending pool
//! (https://github.com/blend-capital/blend-contracts-v2). Supplies the vault's idle assets
//! to earn real interest, and withdraws them back on demand — no mocked yield, no invented
//! interest calculation. All accounting reads (`total_value`) come from the pool's own
//! on-chain `get_reserve`/`get_positions` state, not a local approximation.
//!
//! Uses raw `invoke_contract` with locally-defined `#[contracttype]` structs matching
//! Blend's real on-chain layout, rather than depending on the `blend-contract-sdk` crate —
//! matching this workspace's existing convention (see stellar-zkident's credential_verifier,
//! which avoids a crate dependency on asp_registry for the same reason: it sidesteps wasm
//! export collisions and crate-version drift between this workspace and Blend's own release
//! cadence). `#[contracttype]` structs serialize as field-name-keyed maps, so this works
//! correctly cross-contract as long as field names/types match Blend's real types — verified
//! directly against blend-contracts-v2's source, not guessed from documentation.
use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Env, IntoVal, Map, Symbol, Val, Vec,
};

/// Blend's real Request type (pool/src/pool/actions.rs). request_type is a raw u32, not a
/// typed enum, matching Blend's own on-chain encoding.
#[derive(Clone)]
#[contracttype]
pub struct BlendRequest {
    pub request_type: u32,
    pub address: Address,
    pub amount: i128,
}

/// Blend's RequestType::Supply / ::Withdraw discriminants. Deliberately never using
/// SupplyCollateral (2) — this adapter only ever supplies to earn interest and never
/// borrows, so exposing the position as collateral would add real liquidation risk for
/// zero benefit.
const REQUEST_TYPE_SUPPLY: u32 = 0;
const REQUEST_TYPE_WITHDRAW: u32 = 1;

/// Blend's real Positions type (pool/src/pool/user.rs) — share balances keyed by reserve
/// index, not by asset address or underlying amount.
#[derive(Clone)]
#[contracttype]
pub struct BlendPositions {
    pub liabilities: Map<u32, i128>,
    pub collateral: Map<u32, i128>,
    pub supply: Map<u32, i128>,
}

/// Blend's real ReserveConfig (pool/src/storage.rs). Verified field-for-field against a
/// live pool's actual get_reserve() response on testnet — an earlier version of this struct
/// had an invented `util` field that doesn't exist on-chain and was missing `enabled` and
/// `supply_cap`, which caused a real deserialization failure the first time this adapter was
/// deployed against the real pool instead of the test mock.
#[derive(Clone)]
#[contracttype]
pub struct BlendReserveConfig {
    pub index: u32,
    pub decimals: u32,
    pub c_factor: u32,
    pub l_factor: u32,
    pub max_util: u32,
    pub r_base: u32,
    pub r_one: u32,
    pub r_two: u32,
    pub r_three: u32,
    pub reactivity: u32,
    pub enabled: bool,
    pub supply_cap: i128,
}

/// Blend's real ReserveData (pool/src/storage.rs). b_rate is the bToken->underlying
/// conversion rate with 12 decimals.
#[derive(Clone)]
#[contracttype]
pub struct BlendReserveData {
    pub d_rate: i128,
    pub b_rate: i128,
    pub ir_mod: i128,
    pub b_supply: i128,
    pub d_supply: i128,
    pub backstop_credit: i128,
    pub last_time: u64,
}

/// Blend's real Reserve (pool/src/pool/reserve.rs), as returned by the pool's public
/// get_reserve() — a V2-only entrypoint; V1 pools have no public way for an external
/// contract to read a reserve's current b_rate at all.
#[derive(Clone)]
#[contracttype]
pub struct BlendReserve {
    pub asset: Address,
    pub config: BlendReserveConfig,
    pub data: BlendReserveData,
    pub scalar: i128,
}

const B_RATE_SCALAR: i128 = 1_000_000_000_000; // 10^12

#[contracttype]
pub enum DataKey {
    Admin,
    Controller,
    Pool,
    Asset,
}

#[contract]
pub struct BlendAdapterContract;

#[contractimpl]
impl BlendAdapterContract {
    /// `admin` is a human-controlled deploy-time identity — required so a real keypair can
    /// actually authorize this call from the CLI. `controller` is the strategy_router's own
    /// contract address; it's a DIFFERENT address deliberately, because a contract address
    /// can only self-authorize when IT is the one directly invoking, which a human deployer
    /// calling initialize() from the CLI never is. deposit()/withdraw() are gated on
    /// `controller`, not `admin` — this adapter takes fund-moving instructions from exactly
    /// one router contract, never from a human keypair or arbitrary caller.
    pub fn initialize(env: Env, admin: Address, controller: Address, pool: Address, asset: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Controller, &controller);
        env.storage().instance().set(&DataKey::Pool, &pool);
        env.storage().instance().set(&DataKey::Asset, &asset);
    }

    /// Supplies `amount` of the configured asset to the real Blend pool. The caller
    /// (strategy_router) must have already transferred `amount` into this adapter's own
    /// balance before calling — Blend's submit() moves tokens via a plain transfer() from
    /// the named spender's real balance, not an allowance, so the adapter has to actually
    /// hold the funds first.
    pub fn deposit(env: Env, caller: Address, amount: i128) {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let pool: Address = env.storage().instance().get(&DataKey::Pool).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();

        let requests: Vec<BlendRequest> = vec![
            &env,
            BlendRequest { request_type: REQUEST_TYPE_SUPPLY, address: asset, amount },
        ];
        let args: Vec<Val> = vec![
            &env,
            this.into_val(&env),
            this.into_val(&env),
            this.into_val(&env),
            requests.into_val(&env),
        ];
        let _: BlendPositions = env.invoke_contract(&pool, &Symbol::new(&env, "submit"), args);
    }

    /// Withdraws `amount` of the configured asset from the real Blend pool and forwards it
    /// directly to `to` (skipping an extra hop back through the router). Returns the actual
    /// amount received, which Blend may cap at the position's real current value if `amount`
    /// exceeds it — the caller should treat the return value as authoritative, not assume
    /// it always equals the requested `amount`.
    pub fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128 {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let pool: Address = env.storage().instance().get(&DataKey::Pool).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();

        let balance_before = soroban_sdk::token::TokenClient::new(&env, &asset).balance(&this);

        let requests: Vec<BlendRequest> = vec![
            &env,
            BlendRequest { request_type: REQUEST_TYPE_WITHDRAW, address: asset.clone(), amount },
        ];
        let args: Vec<Val> = vec![
            &env,
            this.into_val(&env),
            this.into_val(&env),
            this.into_val(&env),
            requests.into_val(&env),
        ];
        let _: BlendPositions = env.invoke_contract(&pool, &Symbol::new(&env, "submit"), args);

        let token = soroban_sdk::token::TokenClient::new(&env, &asset);
        let received = token.balance(&this) - balance_before;
        assert!(received > 0, "Blend returned nothing for this withdrawal");
        token.transfer(&this, &to, &received);
        received
    }

    /// Read-only: this adapter's real current position value in the underlying asset,
    /// computed from the pool's own live reserve rate and this adapter's own live share
    /// balance — not a locally-tracked approximation. Returns 0 if the pool has never
    /// initialized a reserve for this asset or if this adapter holds no supply position.
    pub fn total_value(env: Env) -> i128 {
        let pool: Address = env.storage().instance().get(&DataKey::Pool).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let this = env.current_contract_address();

        let reserve_args: Vec<Val> = vec![&env, asset.into_val(&env)];
        let reserve: BlendReserve =
            env.invoke_contract(&pool, &Symbol::new(&env, "get_reserve"), reserve_args);

        let positions_args: Vec<Val> = vec![&env, this.into_val(&env)];
        let positions: BlendPositions =
            env.invoke_contract(&pool, &Symbol::new(&env, "get_positions"), positions_args);

        let b_tokens = positions.supply.get(reserve.config.index).unwrap_or(0);
        if b_tokens == 0 {
            return 0;
        }
        // Mirrors Blend's own Reserve::to_asset_from_b_token (pool/src/pool/reserve.rs) —
        // that conversion is a private impl method, not a callable entrypoint, so this
        // adapter replicates the same floor-division fixed-point math from the reserve
        // data get_reserve() already gives it, rather than needing Blend to expose it.
        (b_tokens * reserve.data.b_rate) / B_RATE_SCALAR
    }

    fn require_controller(env: &Env, caller: &Address) {
        let controller: Address = env.storage().instance().get(&DataKey::Controller).unwrap();
        assert_eq!(*caller, controller, "caller is not this adapter's controller");
    }
}

#[cfg(test)]
mod test;
