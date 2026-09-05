#![cfg(test)]
use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, token::StellarAssetClient, Env};

/// A mock strategy adapter implementing the real StrategyAdapter interface with real token
/// transfers and a real running balance, so the router's forwarding logic is exercised
/// against real behavior — not a rubber stamp that always succeeds.
#[contract]
struct MockAdapter;

#[contracttype]
enum MockKey {
    Asset,
    Balance,
}

#[contractimpl]
impl MockAdapter {
    pub fn init(env: Env, asset: Address) {
        env.storage().instance().set(&MockKey::Asset, &asset);
        env.storage().instance().set(&MockKey::Balance, &0i128);
    }

    pub fn deposit(env: Env, _caller: Address, amount: i128) {
        let bal: i128 = env.storage().instance().get(&MockKey::Balance).unwrap_or(0);
        env.storage().instance().set(&MockKey::Balance, &(bal + amount));
    }

    pub fn withdraw(env: Env, _caller: Address, amount: i128, to: Address) -> i128 {
        let asset: Address = env.storage().instance().get(&MockKey::Asset).unwrap();
        let bal: i128 = env.storage().instance().get(&MockKey::Balance).unwrap_or(0);
        let out = bal.min(amount);
        env.storage().instance().set(&MockKey::Balance, &(bal - out));
        TokenClient::new(&env, &asset).transfer(&env.current_contract_address(), &to, &out);
        out
    }

    pub fn total_value(env: Env) -> i128 {
        env.storage().instance().get(&MockKey::Balance).unwrap_or(0)
    }
}

/// Returns (asset, admin, vault, router_id). `admin` is the human deploy-time identity
/// that can call set_strategy; `vault` stands in for the real vault contract's address,
/// the only caller allowed to invoke deposit/withdraw.
fn setup(env: &Env) -> (Address, Address, Address, Address) {
    // Needed because the router forwards tokens via its own address (self-authorizing a
    // nested transfer/call several hops from the test's own top-level caller arg) — plain
    // mock_all_auths() only auto-approves the top-level arg itself. See the matching note
    // in adapter-blend's tests for the full explanation.
    env.mock_all_auths_allowing_non_root_auth();
    let token_admin = Address::generate(env);
    let asset = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    let admin = Address::generate(env);
    let vault = Address::generate(env);
    let router_id = env.register(StrategyRouterContract, ());
    let client = StrategyRouterContractClient::new(env, &router_id);
    client.initialize(&admin, &vault, &asset);
    (asset, admin, vault, router_id)
}

#[test]
fn test_deposit_forwards_real_tokens_to_the_active_strategy() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);

    let adapter_id = env.register(MockAdapter, ());
    MockAdapterClient::new(&env, &adapter_id).init(&asset);
    client.set_strategy(&admin, &adapter_id);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&adapter_id), 500_0000000i128, "tokens must actually move to the adapter");
    assert_eq!(token.balance(&router_id), 0);
    assert_eq!(client.total_assets(), 500_0000000i128);
}

#[test]
fn test_deposit_is_a_noop_when_no_strategy_is_configured_yet() {
    let env = Env::default();
    let (asset, _admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &100_0000000i128);
    client.deposit(&vault, &100_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&router_id), 100_0000000i128, "funds must stay in the router, not vanish");
    assert_eq!(client.total_assets(), 0);
}

#[test]
fn test_withdraw_sends_real_tokens_directly_to_the_named_recipient() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);

    let adapter_id = env.register(MockAdapter, ());
    MockAdapterClient::new(&env, &adapter_id).init(&asset);
    client.set_strategy(&admin, &adapter_id);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);

    let vault_recipient = Address::generate(&env);
    let received = client.withdraw(&vault, &200_0000000i128, &vault_recipient);
    assert_eq!(received, 200_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&vault_recipient), 200_0000000i128);
    assert_eq!(client.total_assets(), 300_0000000i128);
}

#[test]
#[should_panic(expected = "no strategy configured")]
fn test_withdraw_panics_when_no_strategy_is_configured() {
    let env = Env::default();
    let (_asset, _admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let to = Address::generate(&env);
    client.withdraw(&vault, &1_0000000i128, &to);
}

#[test]
#[should_panic]
fn test_deposit_rejects_a_caller_who_is_not_the_configured_admin() {
    let env = Env::default();
    let (asset, _admin, _vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let impostor = Address::generate(&env);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &100_0000000i128);
    client.deposit(&impostor, &100_0000000i128);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_rejects_a_second_call() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    client.initialize(&admin, &vault, &asset);
}
