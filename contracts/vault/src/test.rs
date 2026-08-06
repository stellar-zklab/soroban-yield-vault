#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env};

#[test]
fn test_deposit_and_withdraw_erc4626() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let asset = env.register_stellar_asset_contract_v2(token_admin).address();
    let router = Address::generate(&env);
    let user = Address::generate(&env);

    StellarAssetClient::new(&env, &asset).mint(&user, &10_000i128);

    let cid = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &cid);
    client.initialize(&admin, &asset, &router);

    let shares = client.deposit(&user, &1_000i128);
    assert!(shares > 0);
    assert_eq!(client.get_share_balance(&user), shares);

    let converted_assets = client.convert_to_assets(&shares);
    assert!(converted_assets > 0);

    let withdrawn = client.withdraw(&user, &shares);
    assert!(withdrawn > 0);
}
