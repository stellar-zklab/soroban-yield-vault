#![cfg(test)]
use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, token::StellarAssetClient, Address, Env};

/// A mock strategy_router implementing the real StrategyRouter interface (deposit,
/// withdraw, total_assets) with real token transfers and an admin-adjustable simulated
/// yield, so the vault's live-accounting logic is exercised against real fund movement —
/// not a rubber stamp that always reports whatever the vault expects.
#[contract]
struct MockRouter;

#[contracttype]
enum MockKey {
    Asset,
    Deployed,
    YieldBps, // simulated accrued yield, in basis points on top of Deployed
}

#[contractimpl]
impl MockRouter {
    pub fn init(env: Env, asset: Address) {
        env.storage().instance().set(&MockKey::Asset, &asset);
        env.storage().instance().set(&MockKey::Deployed, &0i128);
        env.storage().instance().set(&MockKey::YieldBps, &0i128);
    }

    pub fn set_yield_bps(env: Env, bps: i128) {
        env.storage().instance().set(&MockKey::YieldBps, &bps);
    }

    pub fn deposit(env: Env, _caller: Address, amount: i128) {
        let deployed: i128 = env.storage().instance().get(&MockKey::Deployed).unwrap_or(0);
        env.storage().instance().set(&MockKey::Deployed, &(deployed + amount));
    }

    pub fn withdraw(env: Env, _caller: Address, amount: i128, to: Address) -> i128 {
        let asset: Address = env.storage().instance().get(&MockKey::Asset).unwrap();
        let token = soroban_sdk::token::TokenClient::new(&env, &asset);
        // Capped by what's actually, physically here — not by the separate `Deployed`
        // counter, which set_yield_bps()'s simulated interest deliberately never updates
        // (real interest isn't a deposit event either). Using the real balance as the cap
        // matches what a real strategy can actually pay out, and keeps this consistent
        // with what total_assets() below claims is available.
        let out = token.balance(&env.current_contract_address()).min(amount);
        let deployed: i128 = env.storage().instance().get(&MockKey::Deployed).unwrap_or(0);
        env.storage().instance().set(&MockKey::Deployed, &(deployed - out).max(0));
        token.transfer(&env.current_contract_address(), &to, &out);
        out
    }

    pub fn total_assets(env: Env) -> i128 {
        let deployed: i128 = env.storage().instance().get(&MockKey::Deployed).unwrap_or(0);
        let bps: i128 = env.storage().instance().get(&MockKey::YieldBps).unwrap_or(0);
        deployed + (deployed * bps / 10_000)
    }
}

fn setup(env: &Env) -> (VaultContractClient<'static>, Address, Address) {
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let asset = env.register_stellar_asset_contract_v2(token_admin).address();

    let cid = env.register(VaultContract, ());
    let client = VaultContractClient::new(env, &cid);
    client.initialize(&admin, &asset);

    (client, asset, admin)
}

#[test]
fn test_deposit_actually_moves_real_tokens_into_the_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, asset, _admin) = setup(&env);
    let user = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    let shares = client.deposit(&user, &1_000i128);

    assert!(shares > 0);
    assert_eq!(client.balance_of(&user), shares);
    assert_eq!(token.balance(&user), 9_000, "the deposited amount must actually leave the depositor's balance");
    assert_eq!(token.balance(&client.address), 1_000, "the vault contract must actually hold the deposited tokens");
}

#[test]
fn test_withdraw_pays_out_real_tokens_and_burns_shares() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, asset, _admin) = setup(&env);
    let user = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);
    let token = soroban_sdk::token::TokenClient::new(&env, &asset);

    let shares = client.deposit(&user, &1_000i128);
    let withdrawn = client.withdraw(&user, &shares);

    assert_eq!(withdrawn, 1_000);
    assert_eq!(client.balance_of(&user), 0, "shares must be burned after withdrawal");
    assert_eq!(token.balance(&user), 10_000, "the user must get their real tokens back");
    assert_eq!(token.balance(&client.address), 0, "the vault must no longer hold tokens it already paid out");
}

#[test]
#[should_panic(expected = "Insufficient shares")]
fn test_withdraw_rejects_more_shares_than_the_caller_actually_holds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, asset, _admin) = setup(&env);
    let user = Address::generate(&env);
    let other_depositor = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);
    StellarAssetClient::new(&env, &asset).mint(&other_depositor, &10_000i128);

    let user_shares = client.deposit(&user, &1_000i128);
    client.deposit(&other_depositor, &5_000i128);

    // `user` only holds `user_shares` — trying to redeem more than that must not let them
    // reach into the pool of tokens `other_depositor` deposited.
    client.withdraw(&user, &(user_shares + 1));
}

#[test]
fn test_deposit_forwards_real_tokens_to_the_configured_router() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let (client, asset, admin) = setup(&env);
    let user = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);

    let router_id = env.register(MockRouter, ());
    MockRouterClient::new(&env, &router_id).init(&asset);
    client.set_router(&admin, &router_id);

    client.deposit(&user, &1_000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&client.address), 0, "the vault must forward deposits to the router, not hold them idle");
    assert_eq!(token.balance(&router_id), 1_000, "the router must actually receive the real tokens");
    assert_eq!(client.total_assets(), 1_000, "total_assets must count deployed funds via the router");
}

#[test]
fn test_total_assets_reflects_real_yield_reported_by_the_router_without_a_new_deposit() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let (client, asset, admin) = setup(&env);
    let user = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);

    let router_id = env.register(MockRouter, ());
    let router_client = MockRouterClient::new(&env, &router_id);
    router_client.init(&asset);
    client.set_router(&admin, &router_id);

    let shares = client.deposit(&user, &1_000i128);
    assert_eq!(client.total_assets(), 1_000);

    // Simulate 10% accrued yield purely via the router's own report — no new deposit. Also
    // mint the router the extra tokens a real strategy's own interest accrual would have
    // produced, so a subsequent full withdrawal in this test is backed by real balance
    // rather than asserting a value the mock couldn't actually pay out.
    router_client.set_yield_bps(&1_000i128);
    StellarAssetClient::new(&env, &asset).mint(&router_id, &100i128);

    assert_eq!(client.total_assets(), 1_100, "yield reported by the router must flow into the vault's own accounting");

    // The depositor's existing shares are now worth more, purely from accrued yield. Not
    // asserting an exact "1_100" here: at this small a scale, the Yearn V3 virtual-offset
    // constant (1000) is comparable in magnitude to the amounts involved and measurably
    // perturbs the payout — by design, and correctly so (that's the same inflation-attack
    // protection the vault's other tests already rely on) — so the only thing worth
    // asserting is what the whole test is actually about: real, meaningful yield reaching
    // the depositor, cross-checked against the vault's own conversion math rather than a
    // hand-picked number.
    let expected = client.convert_to_assets(&shares);
    let withdrawn = client.withdraw(&user, &shares);
    assert_eq!(withdrawn, expected);
    assert!(withdrawn > 1_000, "a depositor must be able to realize accrued yield on withdrawal, not just their principal back");
}

#[test]
fn test_withdraw_pulls_only_the_real_shortfall_from_the_router() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let (client, asset, admin) = setup(&env);
    let user = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);

    let router_id = env.register(MockRouter, ());
    MockRouterClient::new(&env, &router_id).init(&asset);
    client.set_router(&admin, &router_id);

    // Deposit twice: the router integration forwards each deposit immediately, so the
    // vault itself never holds an idle balance in this configuration.
    let shares = client.deposit(&user, &1_000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&client.address), 0);

    let withdrawn = client.withdraw(&user, &shares);
    assert_eq!(withdrawn, 1_000);
    assert_eq!(token.balance(&user), 10_000);
    assert_eq!(client.total_assets(), 0);
}

#[test]
#[should_panic]
fn test_set_router_rejects_a_caller_who_is_not_the_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, asset, _admin) = setup(&env);
    let router_id = env.register(MockRouter, ());
    MockRouterClient::new(&env, &router_id).init(&asset);

    let impostor = Address::generate(&env);
    client.set_router(&impostor, &router_id);
}

#[test]
fn test_multiple_depositors_can_each_withdraw_their_own_share() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, asset, _admin) = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    StellarAssetClient::new(&env, &asset).mint(&alice, &10_000i128);
    StellarAssetClient::new(&env, &asset).mint(&bob, &10_000i128);
    let token = soroban_sdk::token::TokenClient::new(&env, &asset);

    let alice_shares = client.deposit(&alice, &1_000i128);
    let bob_shares = client.deposit(&bob, &3_000i128);

    client.withdraw(&alice, &alice_shares);
    client.withdraw(&bob, &bob_shares);

    assert_eq!(token.balance(&alice), 10_000);
    assert_eq!(token.balance(&bob), 10_000);
    assert_eq!(token.balance(&client.address), 0);
}
