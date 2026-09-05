#![cfg(test)]
use super::*;
use soroban_sdk::Env;

#[test]
fn test_router_version() {
    let env = Env::default();
    let cid = env.register(StrategyRouterContract, ());
    let client = StrategyRouterContractClient::new(&env, &cid);
    assert_eq!(client.version(), 1);
}
