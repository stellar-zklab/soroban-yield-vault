#![cfg(test)]
use super::*;
use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, token::StellarAssetClient,
};

/// A mock Soroswap Pair implementing just enough of the REAL interface
/// (balance/total_supply/token_0/token_1/get_reserves, verified live against testnet's real
/// deployed pair before writing this adapter) for this adapter's own reads to be exercised
/// for real. `seed_pool`/`apply_swap`/`mint_lp`/`burn_lp` are test-only mutators the mock
/// router below calls — they don't need to match Soroswap's real low-level Pair interface,
/// since this adapter never calls Pair directly for anything but the four real reads above.
#[contract]
struct MockPair;

#[derive(Clone)]
#[contracttype]
enum PairKey {
    Token0,
    Token1,
    Reserve0,
    Reserve1,
    TotalSupply,
    Lp(Address),
}

#[contractimpl]
impl MockPair {
    pub fn init_pair(env: Env, token_0: Address, token_1: Address) {
        env.storage().instance().set(&PairKey::Token0, &token_0);
        env.storage().instance().set(&PairKey::Token1, &token_1);
        env.storage().instance().set(&PairKey::Reserve0, &0i128);
        env.storage().instance().set(&PairKey::Reserve1, &0i128);
        env.storage().instance().set(&PairKey::TotalSupply, &0i128);
    }

    /// Test-only genesis seeding — bypasses the real first-deposit LP-minting formula
    /// entirely, standing in for "a real pool that already has liquidity before this
    /// adapter ever touches it," which is genuinely the case on the real testnet pair this
    /// adapter targets.
    pub fn seed_pool(env: Env, reserve_0: i128, reserve_1: i128, to: Address, initial_lp: i128) {
        env.storage().instance().set(&PairKey::Reserve0, &reserve_0);
        env.storage().instance().set(&PairKey::Reserve1, &reserve_1);
        env.storage().instance().set(&PairKey::TotalSupply, &initial_lp);
        env.storage().persistent().set(&PairKey::Lp(to), &initial_lp);
    }

    pub fn apply_swap(env: Env, reserve_0_delta: i128, reserve_1_delta: i128) {
        let r0: i128 = env.storage().instance().get(&PairKey::Reserve0).unwrap();
        let r1: i128 = env.storage().instance().get(&PairKey::Reserve1).unwrap();
        env.storage().instance().set(&PairKey::Reserve0, &(r0 + reserve_0_delta));
        env.storage().instance().set(&PairKey::Reserve1, &(r1 + reserve_1_delta));
    }

    pub fn mint_lp(env: Env, to: Address, amount: i128, reserve_0_delta: i128, reserve_1_delta: i128) {
        let bal: i128 = env.storage().persistent().get(&PairKey::Lp(to.clone())).unwrap_or(0);
        env.storage().persistent().set(&PairKey::Lp(to), &(bal + amount));
        let ts: i128 = env.storage().instance().get(&PairKey::TotalSupply).unwrap();
        env.storage().instance().set(&PairKey::TotalSupply, &(ts + amount));
        Self::apply_swap(env, reserve_0_delta, reserve_1_delta);
    }

    pub fn burn_lp(env: Env, from: Address, amount: i128, reserve_0_delta: i128, reserve_1_delta: i128) {
        let bal: i128 = env.storage().persistent().get(&PairKey::Lp(from.clone())).unwrap_or(0);
        assert!(bal >= amount, "mock: insufficient LP balance");
        env.storage().persistent().set(&PairKey::Lp(from), &(bal - amount));
        let ts: i128 = env.storage().instance().get(&PairKey::TotalSupply).unwrap();
        env.storage().instance().set(&PairKey::TotalSupply, &(ts - amount));
        Self::apply_swap(env, reserve_0_delta, reserve_1_delta);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage().persistent().get(&PairKey::Lp(id)).unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(&PairKey::TotalSupply).unwrap()
    }

    pub fn token_0(env: Env) -> Address {
        env.storage().instance().get(&PairKey::Token0).unwrap()
    }

    pub fn token_1(env: Env) -> Address {
        env.storage().instance().get(&PairKey::Token1).unwrap()
    }

    pub fn get_reserves(env: Env) -> (i128, i128) {
        (
            env.storage().instance().get(&PairKey::Reserve0).unwrap(),
            env.storage().instance().get(&PairKey::Reserve1).unwrap(),
        )
    }
}

/// A mock Soroswap Router implementing the REAL interface this adapter actually calls
/// (router_get_amounts_out/swap_exact_tokens_for_tokens/add_liquidity/remove_liquidity, all
/// signatures verified live against testnet before writing this adapter) with a genuine
/// constant-product-with-fee formula, not a rubber stamp. Moves real tokens exactly the way
/// the real router does: a plain `transfer` FROM the `to` parameter's own balance (never an
/// allowance), requiring `to.require_auth()` — confirmed against the real router's own
/// published source before writing this.
#[contract]
struct MockRouter;

#[derive(Clone)]
#[contracttype]
enum RouterKey {
    Pair,
}

const FEE_NUM: i128 = 997;
const FEE_DEN: i128 = 1000;

#[contractimpl]
impl MockRouter {
    pub fn init_router(env: Env, pair: Address) {
        env.storage().instance().set(&RouterKey::Pair, &pair);
    }

    fn pair(env: &Env) -> Address {
        env.storage().instance().get(&RouterKey::Pair).unwrap()
    }

    fn reserves_for(env: &Env, token_in: &Address) -> (i128, i128, bool) {
        let pair = Self::pair(env);
        let token_0: Address = env.invoke_contract(&pair, &Symbol::new(env, "token_0"), Vec::new(env));
        let (r0, r1): (i128, i128) = env.invoke_contract(&pair, &Symbol::new(env, "get_reserves"), Vec::new(env));
        if &token_0 == token_in {
            (r0, r1, true)
        } else {
            (r1, r0, false)
        }
    }

    fn amount_out(amount_in: i128, reserve_in: i128, reserve_out: i128) -> i128 {
        let amount_in_with_fee = amount_in * FEE_NUM;
        (amount_in_with_fee * reserve_out) / (reserve_in * FEE_DEN + amount_in_with_fee)
    }

    pub fn router_get_amounts_out(env: Env, amount_in: i128, path: Vec<Address>) -> Vec<i128> {
        let token_in = path.get(0).unwrap();
        let (reserve_in, reserve_out, _) = Self::reserves_for(&env, &token_in);
        let out = Self::amount_out(amount_in, reserve_in, reserve_out);
        vec![&env, amount_in, out]
    }

    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        amount_in: i128,
        amount_out_min: i128,
        path: Vec<Address>,
        to: Address,
        _deadline: u64,
    ) -> Vec<i128> {
        to.require_auth();
        let pair = Self::pair(&env);
        let token_in = path.get(0).unwrap();
        let token_out = path.get(1).unwrap();
        let (reserve_in, reserve_out, in_is_token0) = Self::reserves_for(&env, &token_in);
        let out = Self::amount_out(amount_in, reserve_in, reserve_out);
        assert!(out >= amount_out_min, "mock: slippage exceeded");

        soroban_sdk::token::TokenClient::new(&env, &token_in).transfer(&to, &pair, &amount_in);
        soroban_sdk::token::TokenClient::new(&env, &token_out).transfer(&pair, &to, &out);

        let (d0, d1) = if in_is_token0 { (amount_in, -out) } else { (-out, amount_in) };
        env.invoke_contract::<()>(&pair, &Symbol::new(&env, "apply_swap"), vec![&env, d0.into_val(&env), d1.into_val(&env)]);

        vec![&env, amount_in, out]
    }

    pub fn add_liquidity(
        env: Env,
        token_a: Address,
        token_b: Address,
        amount_a_desired: i128,
        amount_b_desired: i128,
        amount_a_min: i128,
        amount_b_min: i128,
        to: Address,
        _deadline: u64,
    ) -> (i128, i128, i128) {
        to.require_auth();
        let pair = Self::pair(&env);
        let token_0: Address = env.invoke_contract(&pair, &Symbol::new(&env, "token_0"), Vec::new(&env));
        let (r0, r1): (i128, i128) = env.invoke_contract(&pair, &Symbol::new(&env, "get_reserves"), Vec::new(&env));
        let (reserve_a, reserve_b) = if token_0 == token_a { (r0, r1) } else { (r1, r0) };
        let total_supply: i128 = env.invoke_contract(&pair, &Symbol::new(&env, "total_supply"), Vec::new(&env));
        assert!(total_supply > 0, "mock: pool must be seeded before incremental add_liquidity");

        // Real Uniswap V2 optimal-amount selection: use the full desired amount of one side,
        // the current-ratio-implied amount of the other, whichever fits within both desired
        // amounts.
        let optimal_b = (amount_a_desired * reserve_b) / reserve_a;
        let (amount_a, amount_b) = if optimal_b <= amount_b_desired {
            assert!(optimal_b >= amount_b_min, "mock: amount_b below min");
            (amount_a_desired, optimal_b)
        } else {
            let optimal_a = (amount_b_desired * reserve_a) / reserve_b;
            assert!(optimal_a >= amount_a_min, "mock: amount_a below min");
            (optimal_a, amount_b_desired)
        };

        soroban_sdk::token::TokenClient::new(&env, &token_a).transfer(&to, &pair, &amount_a);
        soroban_sdk::token::TokenClient::new(&env, &token_b).transfer(&to, &pair, &amount_b);

        let minted = ((amount_a * total_supply) / reserve_a).min((amount_b * total_supply) / reserve_b);
        let (d0, d1) = if token_0 == token_a { (amount_a, amount_b) } else { (amount_b, amount_a) };
        env.invoke_contract::<()>(
            &pair,
            &Symbol::new(&env, "mint_lp"),
            vec![&env, to.into_val(&env), minted.into_val(&env), d0.into_val(&env), d1.into_val(&env)],
        );

        (amount_a, amount_b, minted)
    }

    pub fn remove_liquidity(
        env: Env,
        token_a: Address,
        token_b: Address,
        liquidity: i128,
        amount_a_min: i128,
        amount_b_min: i128,
        to: Address,
        _deadline: u64,
    ) -> (i128, i128) {
        to.require_auth();
        let pair = Self::pair(&env);
        let token_0: Address = env.invoke_contract(&pair, &Symbol::new(&env, "token_0"), Vec::new(&env));
        let (r0, r1): (i128, i128) = env.invoke_contract(&pair, &Symbol::new(&env, "get_reserves"), Vec::new(&env));
        let total_supply: i128 = env.invoke_contract(&pair, &Symbol::new(&env, "total_supply"), Vec::new(&env));
        let (reserve_a, reserve_b) = if token_0 == token_a { (r0, r1) } else { (r1, r0) };

        let amount_a = (reserve_a * liquidity) / total_supply;
        let amount_b = (reserve_b * liquidity) / total_supply;
        assert!(amount_a >= amount_a_min && amount_b >= amount_b_min, "mock: below min");

        let (d0, d1) = if token_0 == token_a { (-amount_a, -amount_b) } else { (-amount_b, -amount_a) };
        env.invoke_contract::<()>(
            &pair,
            &Symbol::new(&env, "burn_lp"),
            vec![&env, to.into_val(&env), liquidity.into_val(&env), d0.into_val(&env), d1.into_val(&env)],
        );

        soroban_sdk::token::TokenClient::new(&env, &token_a).transfer(&pair, &to, &amount_a);
        soroban_sdk::token::TokenClient::new(&env, &token_b).transfer(&pair, &to, &amount_b);

        (amount_a, amount_b)
    }
}

#[allow(dead_code)] // admin/paired_asset kept for setup clarity even where a given test doesn't read them back
struct Fixture {
    adapter: SoroswapAdapterContractClient<'static>,
    pair_id: Address,
    asset: Address,
    paired_asset: Address,
    admin: Address,
    controller: Address,
}

fn setup(env: &Env, seed_reserve_asset: i128, seed_reserve_paired: i128) -> Fixture {
    let admin = Address::generate(env);
    let controller = Address::generate(env);
    let token_a_admin = Address::generate(env);
    let token_b_admin = Address::generate(env);

    // Real SEP-41 tokens, sorted the same way Soroban addresses sort in reality — whichever
    // comes first becomes token_0, matching the real pair's own address-sorted convention.
    let asset = env.register_stellar_asset_contract_v2(token_a_admin).address();
    let paired_asset = env.register_stellar_asset_contract_v2(token_b_admin).address();
    let (token_0, token_1) = if asset < paired_asset { (asset.clone(), paired_asset.clone()) } else { (paired_asset.clone(), asset.clone()) };

    let pair_id = env.register(MockPair, ());
    MockPairClient::new(env, &pair_id).init_pair(&token_0, &token_1);

    let router_id = env.register(MockRouter, ());
    MockRouterClient::new(env, &router_id).init_router(&pair_id);

    // Seed genesis liquidity to some OTHER address (a real LP, not this adapter) so the pool
    // has real, non-trivial reserves before this adapter ever touches it — matching the real
    // testnet pair's own state. The pair's bookkeeping reserves are only honest if the pair
    // ALSO actually holds that many real tokens — minting real balances to match is what lets
    // a later real transfer paying out of "reserves" actually succeed instead of finding a
    // zero real balance behind a claimed number.
    let genesis_lp = Address::generate(env);
    let (r0, r1) = if token_0 == asset { (seed_reserve_asset, seed_reserve_paired) } else { (seed_reserve_paired, seed_reserve_asset) };
    StellarAssetClient::new(env, &token_0).mint(&pair_id, &r0);
    StellarAssetClient::new(env, &token_1).mint(&pair_id, &r1);
    MockPairClient::new(env, &pair_id).seed_pool(&r0, &r1, &genesis_lp, &1_000_000i128);

    let adapter_id = env.register(SoroswapAdapterContract, ());
    let adapter = SoroswapAdapterContractClient::new(env, &adapter_id);
    adapter.initialize(&admin, &controller, &router_id, &pair_id, &asset, &paired_asset);

    Fixture { adapter, pair_id, asset, paired_asset, admin, controller }
}

#[test]
fn test_deposit_swaps_half_and_provides_real_liquidity() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    // Real testnet-like ratio: roughly 10 XLM per 1 unit of paired asset.
    let fixture = setup(&env, 200_000_000_000, 20_000_000_000);

    StellarAssetClient::new(&env, &fixture.asset).mint(&fixture.controller, &1_000_000_000);
    soroban_sdk::token::TokenClient::new(&env, &fixture.asset).transfer(&fixture.controller, &fixture.adapter.address, &1_000_000_000);

    fixture.adapter.deposit(&fixture.controller, &1_000_000_000);

    let lp_balance = soroban_sdk::token::TokenClient::new(&env, &fixture.pair_id).balance(&fixture.adapter.address);
    assert!(lp_balance > 0, "adapter should hold a real LP position after depositing");

    let value = fixture.adapter.total_value();
    // Real deposit was 1_000_000_000; total_value should be close to that (within a few
    // percent for AMM fee + rounding), not wildly off or zero.
    assert!(value > 900_000_000 && value < 1_050_000_000, "total_value {} should track the real deposit", value);
}

#[test]
fn test_total_value_increases_when_the_pool_earns_real_fees() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let fixture = setup(&env, 200_000_000_000, 20_000_000_000);

    StellarAssetClient::new(&env, &fixture.asset).mint(&fixture.controller, &1_000_000_000);
    soroban_sdk::token::TokenClient::new(&env, &fixture.asset).transfer(&fixture.controller, &fixture.adapter.address, &1_000_000_000);
    fixture.adapter.deposit(&fixture.controller, &1_000_000_000);
    let value_before = fixture.adapter.total_value();

    // Simulate real trading fee accrual the same way a real pool's reserves would grow from
    // other traders' swaps — bump both reserves up proportionally, mirroring genuine fee
    // income landing in the pool without any new LP being minted (real fee-on-reserve
    // growth, not a locally-invented yield number).
    let (r0, r1) = MockPairClient::new(&env, &fixture.pair_id).get_reserves();
    MockPairClient::new(&env, &fixture.pair_id).apply_swap(&(r0 / 20), &(r1 / 20)); // +5% both sides

    let value_after = fixture.adapter.total_value();
    assert!(value_after > value_before, "total_value {} should exceed pre-fee value {}", value_after, value_before);
}

#[test]
fn test_withdraw_recovers_real_funds_from_the_lp_position() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let fixture = setup(&env, 200_000_000_000, 20_000_000_000);

    StellarAssetClient::new(&env, &fixture.asset).mint(&fixture.controller, &1_000_000_000);
    soroban_sdk::token::TokenClient::new(&env, &fixture.asset).transfer(&fixture.controller, &fixture.adapter.address, &1_000_000_000);
    fixture.adapter.deposit(&fixture.controller, &1_000_000_000);

    let recipient = Address::generate(&env);
    let received = fixture.adapter.withdraw(&fixture.controller, &500_000_000, &recipient);

    assert!(received > 450_000_000, "should recover close to the requested amount, got {}", received);
    let recipient_balance = soroban_sdk::token::TokenClient::new(&env, &fixture.asset).balance(&recipient);
    assert_eq!(recipient_balance, received, "recipient must actually receive exactly what withdraw() returned");
}

#[test]
fn test_withdraw_from_idle_balance_alone_never_touches_the_lp_position() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let fixture = setup(&env, 200_000_000_000, 20_000_000_000);

    // Deposit lands entirely as idle balance here (no deposit() call at all) — withdraw
    // should just do a plain transfer, no swap/remove_liquidity needed.
    StellarAssetClient::new(&env, &fixture.asset).mint(&fixture.adapter.address, &100_000_000);

    let recipient = Address::generate(&env);
    let received = fixture.adapter.withdraw(&fixture.controller, &100_000_000, &recipient);

    assert_eq!(received, 100_000_000);
    let lp_balance = soroban_sdk::token::TokenClient::new(&env, &fixture.pair_id).balance(&fixture.adapter.address);
    assert_eq!(lp_balance, 0, "no LP position should have been touched for an idle-covered withdrawal");
}

#[test]
#[should_panic(expected = "caller is not this adapter's controller")]
fn test_deposit_rejects_a_caller_that_is_not_the_controller() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let fixture = setup(&env, 200_000_000_000, 20_000_000_000);
    let stranger = Address::generate(&env);
    StellarAssetClient::new(&env, &fixture.asset).mint(&fixture.adapter.address, &1_000_000);

    fixture.adapter.deposit(&stranger, &1_000_000);
}
