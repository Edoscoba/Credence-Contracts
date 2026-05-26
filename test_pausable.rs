use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};

const AMOUNT: i128 = 1_000;
const DURATION: u64 = 1_000;
const NOTICE: u64 = 100;

fn setup() -> (Env, CredenceBondClient<'static>, Address, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(CredenceBond, ());
    let client = CredenceBondClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let identity = Address::generate(&e);
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract(token_admin);
    client.initialize(&admin, &token_id);
    (e, client, admin, identity, token_id)
}

fn create_active_bond(
    e: &Env,
    client: &CredenceBondClient<'static>,
    identity: &Address,
    token_id: &Address,
    rolling: bool,
) {
    let token_admin = token::StellarAssetClient::new(e, token_id);
    let token_client = token::TokenClient::new(e, token_id);
    token_admin.mint(identity, &AMOUNT);
    token_client.approve(identity, &client.address, &AMOUNT, &1_000_u32);
    client.create_bond(identity, &AMOUNT, &DURATION, &rolling, &NOTICE);
}

#[test]
fn pause_and_unpause_are_admin_gated() {
    let (e, client, admin, _, _) = setup();
    let non_admin = Address::generate(&e);

    assert!(!client.is_paused());
    assert!(client.try_pause(&non_admin).is_err());

    client.pause(&admin);
    assert!(client.is_paused());

    assert!(client.try_unpause(&non_admin).is_err());
    client.unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
fn paused_blocks_required_bond_mutators() {
    let (e, client, admin, identity, token_id) = setup();
    let treasury = Address::generate(&e);

    create_active_bond(&e, &client, &identity, &token_id, true);
    client.set_early_exit_config(&admin, &treasury, &500_u32);
    client.deposit_fees(&25_i128);
    client.pause(&admin);

    assert!(client
        .try_create_bond(&identity, &AMOUNT, &DURATION, &false, &NOTICE)
        .is_err());
    assert!(client.try_top_up(&100_i128).is_err());
    assert!(client.try_withdraw(&100_i128).is_err());
    assert!(client.try_withdraw_early(&100_i128).is_err());
    assert!(client.try_request_withdrawal().is_err());
    assert!(client.try_renew_if_rolling().is_err());
    assert!(client.try_slash_bond(&admin, &100_i128).is_err());
    assert!(client.try_withdraw_bond(&identity).is_err());
    assert!(client.try_collect_fees(&admin).is_err());
}

#[test]
fn paused_blocks_other_mutating_entrypoints() {
    let (e, client, admin, identity, token_id) = setup();
    let attester = Address::generate(&e);
    let treasury = Address::generate(&e);
    create_active_bond(&e, &client, &identity, &token_id, false);
    client.pause(&admin);

    assert!(client
        .try_set_early_exit_config(&admin, &treasury, &500_u32)
        .is_err());
    assert!(client.try_register_attester(&attester).is_err());
    assert!(client.try_unregister_attester(&attester).is_err());
    assert!(client
        .try_set_attester_stake(&admin, &attester, &100_i128)
        .is_err());
    assert!(client
        .try_set_weight_config(&admin, &100_u32, &1_000_u32)
        .is_err());
    assert!(client.try_extend_duration(&100_u64).is_err());
    assert!(client.try_deposit_fees(&25_i128).is_err());
    assert!(client.try_set_callback(&treasury).is_err());
}

#[test]
fn views_remain_callable_while_paused() {
    let (e, client, admin, identity, token_id) = setup();
    create_active_bond(&e, &client, &identity, &token_id, false);
    client.pause(&admin);

    assert!(client.is_paused());
    let bond = client.get_identity_state();
    assert_eq!(bond.identity, identity);
    assert_eq!(client.get_tier(), BondTier::Bronze);
    assert_eq!(client.get_nonce(&identity), 0);
}

#[test]
fn unpause_restores_mutating_paths_after_active_lockup_pause() {
    let (e, client, admin, identity, token_id) = setup();
    create_active_bond(&e, &client, &identity, &token_id, true);

    e.ledger().with_mut(|li| {
        li.timestamp = 10;
    });
    client.pause(&admin);
    assert!(client.try_request_withdrawal().is_err());

    client.unpause(&admin);
    let bond = client.request_withdrawal();
    assert_eq!(bond.withdrawal_requested_at, 10);
}
