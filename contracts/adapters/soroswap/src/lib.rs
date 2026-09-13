#![no_std]
//! Real integration with a deployed Soroswap AMM pool (https://github.com/soroswap/core) —
//! a genuinely different kind of yield source from adapter_blend's lending supply: this
//! adapter earns real trading fees by acting as a liquidity provider, not interest from a
//! borrow/lend market.
//!
//! `deposit()` swaps roughly half the vault's single asset (e.g. native XLM) for
//! `paired_asset` (e.g. testnet USDC) via the real router, then provides both as liquidity —
//! a standard single-asset LP "zap," since the vault only ever hands this adapter one asset
//! but a pool needs both sides. `total_value()` reads the pool's own real, live
//! `get_reserves()`/`total_supply()` and this adapter's own real LP token balance to compute
//! this adapter's genuine share of the pool, converted back into `asset` terms — not a
//! locally-tracked approximation, matching this workspace's existing convention.
//!
//! Real Soroswap semantics verified directly against testnet before writing this (not
//! guessed from docs): the router's `add_liquidity`/`remove_liquidity`/
//! `swap_exact_tokens_for_tokens` all move tokens via a plain `transfer` FROM the `to`
//! parameter's own balance (never an allowance/`transfer_from`), requiring `to.require_auth()`
//! — so this adapter always passes its OWN address as `to`. The pool contract (`Pair`) also
//! has no fixed token ordering — `token_0()`/`token_1()` are sorted by address, confirmed
//! live to put USDC before XLM on this specific pair — so reserve/order-dependent math here
//! always checks `token_0()` rather than assuming a side.
//!
//! **A contract's self-authorization is NOT automatic two hops deep** — confirmed the hard
//! way, by a real failed testnet transaction on this exact adapter before this fix: when this
//! adapter calls the router, and the router then calls `token.transfer(this_adapter, pair,
//! amount)`, that's `adapter -> router -> token`, and Soroban only auto-authorizes a
//! contract's own address for calls it makes DIRECTLY (one hop). For a call two hops deep
//! that needs this adapter's authorization, this adapter must call
//! `env.authorize_as_current_contract(...)` with the exact nested call(s) pre-declared,
//! immediately before making the router call that will trigger them — see
//! `authorize_transfers` below. The local unit tests didn't catch this because
//! `mock_all_auths_allowing_non_root_auth()` bypasses the check entirely; only the real
//! testnet deploy did.
//!
//! Uses raw `invoke_contract` with locally-defined arg/return shapes matching Soroswap's
//! real on-chain interface, rather than depending on a `soroswap-sdk` crate — matching this
//! workspace's existing convention (see adapter_blend's own module doc for the same
//! reasoning: sidesteps wasm export collisions and crate-version drift).
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, contracttype, vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

const SLIPPAGE_TOLERANCE_BPS: i128 = 100; // 1% — applied to the real live quote just fetched,
                                           // not an arbitrary guess.
const DEADLINE_WINDOW_SECONDS: u64 = 1800; // Generous window for the transaction to actually
                                            // land, same reasoning as this workspace's other
                                            // real on-chain calls with a deadline/timeout param.

#[contracttype]
pub enum DataKey {
    Admin,
    Controller,
    Router,
    Pair,
    Asset,
    PairedAsset,
}

#[contract]
pub struct SoroswapAdapterContract;

#[contractimpl]
impl SoroswapAdapterContract {
    /// `pair` is supplied directly (computed off-chain once via the real factory's
    /// `get_pair(asset, paired_asset)`) rather than looked up on-chain from a stored factory
    /// address — this adapter never needs to know about the factory at all after deploy time,
    /// matching how adapter_blend is handed its pool address directly rather than discovering
    /// it itself.
    pub fn initialize(
        env: Env,
        admin: Address,
        controller: Address,
        router: Address,
        pair: Address,
        asset: Address,
        paired_asset: Address,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Controller, &controller);
        env.storage().instance().set(&DataKey::Router, &router);
        env.storage().instance().set(&DataKey::Pair, &pair);
        env.storage().instance().set(&DataKey::Asset, &asset);
        env.storage().instance().set(&DataKey::PairedAsset, &paired_asset);
    }

    /// Swaps roughly half of `amount` for `paired_asset`, then provides both as liquidity.
    /// Any dust left over from the swap/LP-sizing mismatch (real pools rarely accept an
    /// LP deposit at EXACTLY the ratio just swapped for) is deliberately left idle in this
    /// adapter's own balance rather than force-swept — total_value() below accounts for it
    /// correctly, so it's never lost or mispriced, just not immediately earning fees.
    pub fn deposit(env: Env, caller: Address, amount: i128) {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let router: Address = env.storage().instance().get(&DataKey::Router).unwrap();
        let pair: Address = env.storage().instance().get(&DataKey::Pair).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let paired_asset: Address = env.storage().instance().get(&DataKey::PairedAsset).unwrap();
        let this = env.current_contract_address();

        let half = amount / 2;
        let remaining = amount - half;

        let quoted = Self::router_get_amounts_out(&env, &router, half, &asset, &paired_asset);
        let min_out = (quoted * (10_000 - SLIPPAGE_TOLERANCE_BPS)) / 10_000;
        Self::authorize_transfers(&env, &[(asset.clone(), pair.clone(), half)]);
        let received_paired =
            Self::router_swap(&env, &router, &asset, &paired_asset, half, min_out, &this);

        // The router's real add_liquidity ALWAYS tries to derive side B from the FULL
        // amount_a_desired first (never from a pre-shrunk value), only falling back to
        // deriving A from amount_b_desired if that first derivation would exceed
        // amount_b_desired. A real failed testnet transaction caught the bug in an earlier
        // version of this function: passing an already-derived (rounded) amount as
        // amount_a_desired made the router re-derive B from that already-rounded number,
        // compounding a second floor-division loss and landing 1 unit below what had been
        // pre-authorized. The fix: always pass the FULL real remaining/received_paired as
        // desired (never a value this adapter already reduced), and predict the router's
        // actual final pair using that exact same single-derivation logic on those same full,
        // unrounded inputs — which, run once each way with no compounding, matches exactly.
        let (reserve_asset, reserve_paired) = Self::ordered_reserves(&env, &pair, &asset);
        let (final_asset, final_paired) = if reserve_asset > 0 && reserve_paired > 0 {
            let paired_from_full_asset = (remaining * reserve_paired) / reserve_asset;
            if paired_from_full_asset <= received_paired {
                (remaining, paired_from_full_asset)
            } else {
                let asset_from_full_paired = (received_paired * reserve_asset) / reserve_paired;
                (asset_from_full_paired, received_paired)
            }
        } else {
            (remaining, received_paired)
        };

        let deadline = env.ledger().timestamp() + DEADLINE_WINDOW_SECONDS;
        Self::authorize_transfers(
            &env,
            &[(asset.clone(), pair.clone(), final_asset), (paired_asset.clone(), pair.clone(), final_paired)],
        );
        let args: Vec<Val> = vec![
            &env,
            asset.into_val(&env),
            paired_asset.into_val(&env),
            remaining.into_val(&env),
            received_paired.into_val(&env),
            0i128.into_val(&env), // no interleaving is possible within this one atomic tx, so
            0i128.into_val(&env), // there's no real slippage to guard against beyond the exact
                                   // amounts just authorized above.
            this.into_val(&env),
            deadline.into_val(&env),
        ];
        let _: (i128, i128, i128) = env.invoke_contract(&router, &Symbol::new(&env, "add_liquidity"), args);
    }

    /// Recovers `amount` of `asset` for `to`, pulling from idle balance first, then the LP
    /// position (burning just enough LP for the current live share price to cover the
    /// shortfall), then any idle `paired_asset` swapped back to `asset` last. Returns the
    /// real amount actually sent — which may be less than requested if the combined idle +
    /// LP position's real current value can't cover it, the same honest contract
    /// adapter_blend's own withdraw() makes (no strict slippage guarantee is enforced here;
    /// the real result is authoritative, same reasoning as that adapter).
    pub fn withdraw(env: Env, caller: Address, amount: i128, to: Address) -> i128 {
        caller.require_auth();
        Self::require_controller(&env, &caller);
        assert!(amount > 0, "amount must be positive");

        let router: Address = env.storage().instance().get(&DataKey::Router).unwrap();
        let pair: Address = env.storage().instance().get(&DataKey::Pair).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let paired_asset: Address = env.storage().instance().get(&DataKey::PairedAsset).unwrap();
        let this = env.current_contract_address();
        let asset_token = soroban_sdk::token::TokenClient::new(&env, &asset);

        let idle_asset = asset_token.balance(&this);
        if idle_asset < amount {
            let lp_balance: i128 = soroban_sdk::token::TokenClient::new(&env, &pair).balance(&this);
            if lp_balance > 0 {
                let lp_value = Self::lp_value_in_asset(&env, &router, &pair, &asset, &paired_asset, lp_balance);
                if lp_value > 0 {
                    let shortfall = amount - idle_asset;
                    let lp_to_burn = (lp_balance * shortfall.min(lp_value)) / lp_value;
                    let lp_to_burn = lp_to_burn.min(lp_balance);
                    if lp_to_burn > 0 {
                        let deadline = env.ledger().timestamp() + DEADLINE_WINDOW_SECONDS;
                        // remove_liquidity transfers exactly `liquidity` (the LP token,
                        // i.e. the pair contract itself) from this adapter to the pair —
                        // no router-side reduction, so pre-authorizing this exact amount
                        // matches the real transfer precisely.
                        Self::authorize_transfers(&env, &[(pair.clone(), pair.clone(), lp_to_burn)]);
                        let args: Vec<Val> = vec![
                            &env,
                            asset.into_val(&env),
                            paired_asset.into_val(&env),
                            lp_to_burn.into_val(&env),
                            0i128.into_val(&env),
                            0i128.into_val(&env),
                            this.into_val(&env),
                            deadline.into_val(&env),
                        ];
                        let _: (i128, i128) =
                            env.invoke_contract(&router, &Symbol::new(&env, "remove_liquidity"), args);
                    }
                }
            }

            let paired_balance = soroban_sdk::token::TokenClient::new(&env, &paired_asset).balance(&this);
            if paired_balance > 0 {
                Self::authorize_transfers(&env, &[(paired_asset.clone(), pair.clone(), paired_balance)]);
                Self::router_swap(&env, &router, &paired_asset, &asset, paired_balance, 0, &this);
            }
        }

        let available = asset_token.balance(&this);
        let to_send = amount.min(available);
        assert!(to_send > 0, "no funds available to withdraw");
        asset_token.transfer(&this, &to, &to_send);
        to_send
    }

    /// Read-only: idle `asset` balance, plus idle `paired_asset` valued in `asset` terms via
    /// a real live router quote, plus this adapter's real LP position's real current value —
    /// never a locally-tracked approximation. Returns 0 if this adapter holds nothing yet.
    pub fn total_value(env: Env) -> i128 {
        let router: Address = env.storage().instance().get(&DataKey::Router).unwrap();
        let pair: Address = env.storage().instance().get(&DataKey::Pair).unwrap();
        let asset: Address = env.storage().instance().get(&DataKey::Asset).unwrap();
        let paired_asset: Address = env.storage().instance().get(&DataKey::PairedAsset).unwrap();
        let this = env.current_contract_address();

        let idle_asset = soroban_sdk::token::TokenClient::new(&env, &asset).balance(&this);
        let idle_paired = soroban_sdk::token::TokenClient::new(&env, &paired_asset).balance(&this);
        let idle_paired_in_asset = if idle_paired > 0 {
            Self::router_get_amounts_out(&env, &router, idle_paired, &paired_asset, &asset)
        } else {
            0
        };

        let lp_balance: i128 = soroban_sdk::token::TokenClient::new(&env, &pair).balance(&this);
        let lp_value = if lp_balance > 0 {
            Self::lp_value_in_asset(&env, &router, &pair, &asset, &paired_asset, lp_balance)
        } else {
            0
        };

        idle_asset + idle_paired_in_asset + lp_value
    }

    /// This adapter's real proportional share of the pool's real reserves (via
    /// `lp_balance / total_supply()`), with the `paired_asset` half additionally converted
    /// into `asset` terms via a real live router quote — not a fixed price assumption.
    fn lp_value_in_asset(
        env: &Env,
        router: &Address,
        pair: &Address,
        asset: &Address,
        paired_asset: &Address,
        lp_balance: i128,
    ) -> i128 {
        let total_supply: i128 = env.invoke_contract(pair, &Symbol::new(env, "total_supply"), Vec::new(env));
        if total_supply == 0 {
            return 0;
        }
        let (reserve_asset, reserve_paired) = Self::ordered_reserves(env, pair, asset);

        let owned_asset = (reserve_asset * lp_balance) / total_supply;
        let owned_paired = (reserve_paired * lp_balance) / total_supply;
        let owned_paired_in_asset = if owned_paired > 0 {
            Self::router_get_amounts_out(env, router, owned_paired, paired_asset, asset)
        } else {
            0
        };
        owned_asset + owned_paired_in_asset
    }

    /// Real, live `get_reserves()`/`token_0()` reads, remapped so the first element of the
    /// returned tuple always corresponds to `asset` regardless of the pool's real address-sorted
    /// token order (confirmed live to put USDC before XLM on this specific pair — never assumed).
    fn ordered_reserves(env: &Env, pair: &Address, asset: &Address) -> (i128, i128) {
        let token_0: Address = env.invoke_contract(pair, &Symbol::new(env, "token_0"), Vec::new(env));
        let (reserve_0, reserve_1): (i128, i128) =
            env.invoke_contract(pair, &Symbol::new(env, "get_reserves"), Vec::new(env));
        if &token_0 == asset {
            (reserve_0, reserve_1)
        } else {
            (reserve_1, reserve_0)
        }
    }

    /// Pre-authorizes one or more nested token transfers this adapter's OWN address will need
    /// to satisfy two calls deep (adapter -> router -> token), which Soroban does not
    /// auto-authorize the way a direct one-hop self-call would be — see this module's doc
    /// comment for why, and the real failed testnet transaction that first surfaced this. Each
    /// `(token, to, amount)` becomes `token.transfer(this, to, amount)` in the authorized tree;
    /// callers must pass the EXACT amount the downstream call will actually transfer, since
    /// Soroban matches authorization entries against the real invocation's args exactly.
    fn authorize_transfers(env: &Env, transfers: &[(Address, Address, i128)]) {
        let this = env.current_contract_address();
        let mut entries: Vec<InvokerContractAuthEntry> = Vec::new(env);
        for (token, to, amount) in transfers {
            entries.push_back(InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: token.clone(),
                    fn_name: Symbol::new(env, "transfer"),
                    args: vec![env, this.into_val(env), to.into_val(env), (*amount).into_val(env)],
                },
                sub_invocations: Vec::new(env),
            }));
        }
        env.authorize_as_current_contract(entries);
    }

    fn router_get_amounts_out(env: &Env, router: &Address, amount_in: i128, from: &Address, to: &Address) -> i128 {
        let path: Vec<Address> = vec![env, from.clone(), to.clone()];
        let args: Vec<Val> = vec![env, amount_in.into_val(env), path.into_val(env)];
        let amounts: Vec<i128> = env.invoke_contract(router, &Symbol::new(env, "router_get_amounts_out"), args);
        amounts.get(amounts.len() - 1).unwrap_or(0)
    }

    fn router_swap(env: &Env, router: &Address, from: &Address, to: &Address, amount_in: i128, min_out: i128, recipient: &Address) -> i128 {
        let path: Vec<Address> = vec![env, from.clone(), to.clone()];
        let deadline = env.ledger().timestamp() + DEADLINE_WINDOW_SECONDS;
        let args: Vec<Val> = vec![
            env,
            amount_in.into_val(env),
            min_out.into_val(env),
            path.into_val(env),
            recipient.into_val(env),
            deadline.into_val(env),
        ];
        let amounts: Vec<i128> =
            env.invoke_contract(router, &Symbol::new(env, "swap_exact_tokens_for_tokens"), args);
        amounts.get(amounts.len() - 1).unwrap_or(0)
    }

    fn require_controller(env: &Env, caller: &Address) {
        let controller: Address = env.storage().instance().get(&DataKey::Controller).unwrap();
        assert_eq!(*caller, controller, "caller is not this adapter's controller");
    }
}

#[cfg(test)]
mod test;
