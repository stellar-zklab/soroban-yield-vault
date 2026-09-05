#![cfg(test)]
use super::*;
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, token::StellarAssetClient, Env, Map,
};

/// A mock Blend V2 pool implementing just enough of the real interface (submit,
/// get_reserve, get_positions) to exercise this adapter's real logic against real token
/// transfers and real b_rate-based share math — not a rubber-stamp that always succeeds
/// regardless of what's called, and not a claim that this replaces integration testing
/// against the real deployed pool (see docs/DEPLOYMENT_GUIDE.md for that).
#[contract]
struct MockBlendPool;

#[derive(Clone)]
#[contracttype]
enum MockKey {
    Asset,
    Index,
    BRate,
    Shares(Address),
}

#[contractimpl]
impl MockBlendPool {
    pub fn init(env: Env, asset: Address, index: u32, b_rate: i128) {
        env.storage().instance().set(&MockKey::Asset, &asset);
        env.storage().instance().set(&MockKey::Index, &index);
        env.storage().instance().set(&MockKey::BRate, &b_rate);
    }

    pub fn set_b_rate(env: Env, b_rate: i128) {
        env.storage().instance().set(&MockKey::BRate, &b_rate);
    }

    pub fn submit(
        env: Env,
        from: Address,
        spender: Address,
        to: Address,
        requests: Vec<BlendRequest>,
    ) -> BlendPositions {
        // Matches real Blend's own submit() (pool/src/contract.rs): spender always
        // authorizes, from authorizes too only if it differs from spender. Without this,
        // there is no root-tied authorization anywhere in the chain for the caller's
        // address, and the nested token transfer's own self-authorization has nothing to
        // anchor to under mock_all_auths()'s root-invocation check.
        spender.require_auth();
        if from != spender {
            from.require_auth();
        }
        let asset: Address = env.storage().instance().get(&MockKey::Asset).unwrap();
        let index: u32 = env.storage().instance().get(&MockKey::Index).unwrap();
        let b_rate: i128 = env.storage().instance().get(&MockKey::BRate).unwrap();
        let mut shares: i128 = env
            .storage()
            .persistent()
            .get(&MockKey::Shares(from.clone()))
            .unwrap_or(0);
        let token = soroban_sdk::token::TokenClient::new(&env, &asset);

        for req in requests.iter() {
            if req.request_type == 0 {
                // Supply: pulls real tokens from spender's real balance, mints shares at
                // the current b_rate — exactly the fixed-point relationship Blend uses.
                token.transfer(&spender, &env.current_contract_address(), &req.amount);
                shares += (req.amount * B_RATE_SCALAR) / b_rate;
            } else if req.request_type == 1 {
                let shares_needed = (req.amount * B_RATE_SCALAR) / b_rate;
                let shares_to_burn = shares.min(shares_needed);
                shares -= shares_to_burn;
                let underlying_out = (shares_to_burn * b_rate) / B_RATE_SCALAR;
                token.transfer(&env.current_contract_address(), &to, &underlying_out);
            }
        }
        env.storage().persistent().set(&MockKey::Shares(from), &shares);

        let mut supply = Map::new(&env);
        supply.set(index, shares);
        BlendPositions { liabilities: Map::new(&env), collateral: Map::new(&env), supply }
    }

    pub fn get_reserve(env: Env, asset: Address) -> BlendReserve {
        let index: u32 = env.storage().instance().get(&MockKey::Index).unwrap();
        let b_rate: i128 = env.storage().instance().get(&MockKey::BRate).unwrap();
        BlendReserve {
            asset,
            config: BlendReserveConfig {
                index, decimals: 7, c_factor: 0, l_factor: 0, util: 0, max_util: 0,
                r_base: 0, r_one: 0, r_two: 0, r_three: 0, reactivity: 0,
            },
            data: BlendReserveData {
                d_rate: B_RATE_SCALAR, b_rate, ir_mod: 0, b_supply: 0, d_supply: 0,
                backstop_credit: 0, last_time: 0,
            },
            scalar: 10_000_000,
        }
    }

    pub fn get_positions(env: Env, address: Address) -> BlendPositions {
        let index: u32 = env.storage().instance().get(&MockKey::Index).unwrap();
        let shares: i128 = env.storage().persistent().get(&MockKey::Shares(address)).unwrap_or(0);
        let mut supply = Map::new(&env);
        supply.set(index, shares);
        BlendPositions { liabilities: Map::new(&env), collateral: Map::new(&env), supply }
    }
}

fn setup(env: &Env, b_rate: i128) -> (Address, Address, Address) {
    // Plain mock_all_auths() only auto-approves require_auth() for the address that's the
    // direct top-level caller arg — it does not extend to a contract self-authorizing
    // deeper in a nested call chain (adapter -> pool -> token here), even though that's
    // exactly how a real deployed contract legitimately self-authorizes on-chain. Blend's
    // own test suite hits this same thing and uses this same non-root-allowing variant.
    env.mock_all_auths_allowing_non_root_auth();
    let token_admin = Address::generate(env);
    let asset = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    let pool = env.register(MockBlendPool, ());
    let pool_client = MockBlendPoolClient::new(env, &pool);
    pool_client.init(&asset, &0u32, &b_rate);
    (asset, pool, token_admin)
}

#[test]
fn test_deposit_actually_transfers_real_tokens_into_the_real_pool() {
    let env = Env::default();
    let (asset, pool, _) = setup(&env, B_RATE_SCALAR);
    let router = Address::generate(&env);
    let admin = Address::generate(&env);

    let adapter_id = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &adapter_id);
    client.initialize(&admin, &router, &pool, &asset);

    StellarAssetClient::new(&env, &asset).mint(&adapter_id, &1_000_0000000i128);
    client.deposit(&router, &1_000_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&adapter_id), 0, "the adapter should hold nothing itself once supplied");
    assert_eq!(token.balance(&pool), 1_000_0000000i128, "the pool must actually receive the real tokens");
}

#[test]
fn test_total_value_reflects_real_yield_accrual_without_any_new_deposit() {
    let env = Env::default();
    let (asset, pool, _) = setup(&env, B_RATE_SCALAR);
    let router = Address::generate(&env);
    let admin = Address::generate(&env);

    let adapter_id = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &adapter_id);
    client.initialize(&admin, &router, &pool, &asset);

    StellarAssetClient::new(&env, &asset).mint(&adapter_id, &1_000_0000000i128);
    client.deposit(&router, &1_000_0000000i128);
    assert_eq!(client.total_value(), 1_000_0000000i128);

    // Simulate 10% interest accrual purely by advancing the pool's own b_rate — the exact
    // mechanism real Blend uses internally. No new deposit happened.
    let pool_client = MockBlendPoolClient::new(&env, &pool);
    pool_client.set_b_rate(&(B_RATE_SCALAR * 110 / 100));

    assert_eq!(
        client.total_value(),
        1_100_0000000i128,
        "total_value must reflect the pool's real accrued interest, not a stale deposit snapshot"
    );
}

#[test]
fn test_withdraw_sends_real_tokens_directly_to_the_named_recipient() {
    let env = Env::default();
    let (asset, pool, _) = setup(&env, B_RATE_SCALAR);
    let router = Address::generate(&env);
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);

    let adapter_id = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &adapter_id);
    client.initialize(&admin, &router, &pool, &asset);

    StellarAssetClient::new(&env, &asset).mint(&adapter_id, &500_0000000i128);
    client.deposit(&router, &500_0000000i128);

    let received = client.withdraw(&router, &200_0000000i128, &vault);
    assert_eq!(received, 200_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&vault), 200_0000000i128, "funds must land at `to`, not the router or adapter");
    assert_eq!(token.balance(&adapter_id), 0);
    assert_eq!(client.total_value(), 300_0000000i128);
}

#[test]
#[should_panic]
fn test_deposit_rejects_a_caller_who_is_not_the_configured_controller() {
    let env = Env::default();
    let (asset, pool, _) = setup(&env, B_RATE_SCALAR);
    let router = Address::generate(&env);
    let admin = Address::generate(&env);
    let impostor = Address::generate(&env);

    let adapter_id = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &adapter_id);
    client.initialize(&admin, &router, &pool, &asset);

    StellarAssetClient::new(&env, &asset).mint(&adapter_id, &100_0000000i128);
    client.deposit(&impostor, &100_0000000i128);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_rejects_a_second_call() {
    let env = Env::default();
    let (asset, pool, _) = setup(&env, B_RATE_SCALAR);
    let router = Address::generate(&env);
    let admin = Address::generate(&env);

    let adapter_id = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &adapter_id);
    client.initialize(&admin, &router, &pool, &asset);
    client.initialize(&admin, &router, &pool, &asset);
}
