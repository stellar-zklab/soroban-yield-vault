#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn test_phoenix_adapter_version() {
    let env = Env::default();
    let cid = env.register(PhoenixAdapterContract, ());
    let client = PhoenixAdapterContractClient::new(&env, &cid);
    assert_eq!(client.version(), 1);
}
