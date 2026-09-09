#![cfg(test)]
use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, token::{StellarAssetClient, TokenClient}, Env};

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
/// that can call add_strategy/update_debt; `vault` stands in for the real vault contract's
/// address, the only caller allowed to invoke deposit/withdraw.
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

fn add_mock_strategy(env: &Env, router_id: &Address, admin: &Address, asset: &Address) -> Address {
    let client = StrategyRouterContractClient::new(env, router_id);
    let adapter_id = env.register(MockAdapter, ());
    MockAdapterClient::new(env, &adapter_id).init(asset);
    client.add_strategy(admin, &adapter_id);
    adapter_id
}

#[test]
fn test_deposit_always_leaves_funds_idle_until_explicitly_allocated() {
    // Deposits no longer auto-deploy to a strategy — see the module doc comment for why.
    // This holds whether or not a strategy is even registered.
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&router_id), 500_0000000i128, "deposit must not move funds on its own anymore");
    // total_assets reflects idle + every strategy's live total_value(); with nothing
    // allocated yet, that's just the idle balance.
    assert_eq!(client.total_assets(), 500_0000000i128);
}

#[test]
fn test_update_debt_allocates_real_funds_to_a_registered_strategy() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);

    client.set_max_debt_for_strategy(&admin, &adapter_id, &500_0000000i128);
    let new_debt = client.update_debt(&admin, &adapter_id, &500_0000000i128);
    assert_eq!(new_debt, 500_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&adapter_id), 500_0000000i128, "update_debt must actually move the tokens");
    assert_eq!(token.balance(&router_id), 0);
    assert_eq!(client.get_debt(&adapter_id), 500_0000000i128);
    assert_eq!(client.total_assets(), 500_0000000i128);
}

#[test]
#[should_panic(expected = "desired_debt exceeds this strategy's max_debt")]
fn test_update_debt_rejects_exceeding_the_strategys_max_debt() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);

    client.set_max_debt_for_strategy(&admin, &adapter_id, &100_0000000i128);
    client.update_debt(&admin, &adapter_id, &500_0000000i128);
}

#[test]
fn test_update_debt_can_decrease_allocation_pulling_funds_back_to_idle() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);
    client.set_max_debt_for_strategy(&admin, &adapter_id, &500_0000000i128);
    client.update_debt(&admin, &adapter_id, &500_0000000i128);

    let new_debt = client.update_debt(&admin, &adapter_id, &200_0000000i128);
    assert_eq!(new_debt, 200_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&router_id), 300_0000000i128, "the 300 pulled back must land in the router's idle balance");
    assert_eq!(token.balance(&adapter_id), 200_0000000i128);
    assert_eq!(client.get_debt(&adapter_id), 200_0000000i128);
}

#[test]
fn test_withdraw_pulls_shortfall_from_a_strategy_when_idle_alone_is_insufficient() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);
    client.set_max_debt_for_strategy(&admin, &adapter_id, &500_0000000i128);
    client.update_debt(&admin, &adapter_id, &500_0000000i128);
    // All 500 is now deployed; idle is 0.

    let vault_recipient = Address::generate(&env);
    let received = client.withdraw(&vault, &200_0000000i128, &vault_recipient);
    assert_eq!(received, 200_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&vault_recipient), 200_0000000i128);
    assert_eq!(client.get_debt(&adapter_id), 300_0000000i128, "withdrawing the shortfall from the strategy must reduce its recorded debt");
    assert_eq!(client.total_assets(), 300_0000000i128);
}

#[test]
#[should_panic(expected = "insufficient total funds across idle balance and all registered strategies")]
fn test_withdraw_panics_when_nothing_can_cover_the_requested_amount() {
    let env = Env::default();
    let (_asset, _admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let to = Address::generate(&env);
    client.withdraw(&vault, &1_0000000i128, &to);
}

#[test]
#[should_panic]
fn test_deposit_rejects_a_caller_who_is_not_the_configured_controller() {
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

#[test]
#[should_panic(expected = "strategy already added")]
fn test_add_strategy_rejects_a_duplicate() {
    let env = Env::default();
    let (asset, admin, _vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);
    client.add_strategy(&admin, &adapter_id);
}

#[test]
#[should_panic(expected = "strategy still has debt allocated")]
fn test_remove_strategy_rejects_removal_while_debt_is_still_allocated() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);
    client.set_max_debt_for_strategy(&admin, &adapter_id, &500_0000000i128);
    client.update_debt(&admin, &adapter_id, &500_0000000i128);

    client.remove_strategy(&admin, &adapter_id);
}

#[test]
fn test_remove_strategy_succeeds_once_debt_is_fully_pulled_back() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_id = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &500_0000000i128);
    client.deposit(&vault, &500_0000000i128);
    client.set_max_debt_for_strategy(&admin, &adapter_id, &500_0000000i128);
    client.update_debt(&admin, &adapter_id, &500_0000000i128);
    client.update_debt(&admin, &adapter_id, &0i128);

    client.remove_strategy(&admin, &adapter_id);
    assert_eq!(client.get_strategies().len(), 0);
}

#[test]
fn test_multiple_strategies_each_get_their_own_independent_allocation() {
    let env = Env::default();
    let (asset, admin, vault, router_id) = setup(&env);
    let client = StrategyRouterContractClient::new(&env, &router_id);
    let adapter_a = add_mock_strategy(&env, &router_id, &admin, &asset);
    let adapter_b = add_mock_strategy(&env, &router_id, &admin, &asset);

    StellarAssetClient::new(&env, &asset).mint(&router_id, &1_000_0000000i128);
    client.deposit(&vault, &1_000_0000000i128);

    client.set_max_debt_for_strategy(&admin, &adapter_a, &600_0000000i128);
    client.set_max_debt_for_strategy(&admin, &adapter_b, &400_0000000i128);
    client.update_debt(&admin, &adapter_a, &600_0000000i128);
    client.update_debt(&admin, &adapter_b, &400_0000000i128);

    let token = soroban_sdk::token::TokenClient::new(&env, &asset);
    assert_eq!(token.balance(&adapter_a), 600_0000000i128);
    assert_eq!(token.balance(&adapter_b), 400_0000000i128);
    assert_eq!(client.total_assets(), 1_000_0000000i128);
    assert_eq!(client.get_strategies().len(), 2);
}
