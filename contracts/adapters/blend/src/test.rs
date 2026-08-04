#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn test_blend_adapter_version() {
    let env = Env::default();
    let cid = env.register(BlendAdapterContract, ());
    let client = BlendAdapterContractClient::new(&env, &cid);
    assert_eq!(client.version(), 1);
}
