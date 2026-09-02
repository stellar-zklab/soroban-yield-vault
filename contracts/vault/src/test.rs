#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env};

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
