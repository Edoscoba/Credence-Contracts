use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};

const INITIAL_BOND: i128 = 1_000;
const TOP_UP: i128 = 250;
const DURATION: u64 = 1_000;

fn setup() -> (
    Env,
    CredenceBondClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(CredenceBond, ());
    let client = CredenceBondClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let identity = Address::generate(&e);
    let treasury = Address::generate(&e);
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract(token_admin);
    client.initialize(&admin, &token_id);
    (e, client, admin, identity, treasury, token_id)
}

fn mint_and_approve(
    e: &Env,
    client: &CredenceBondClient<'static>,
    token_id: &Address,
    owner: &Address,
    balance: i128,
    allowance: i128,
) {
    let stellar = token::StellarAssetClient::new(e, token_id);
    let token = token::TokenClient::new(e, token_id);
    stellar.mint(owner, &balance);
    token.approve(owner, &client.address, &allowance, &1_000_u32);
}

#[test]
fn create_bond_escrows_real_tokens() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND,
        INITIAL_BOND,
    );

    let bond = client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);

    assert_eq!(bond.bonded_amount, INITIAL_BOND);
    assert_eq!(token.balance(&identity), 0);
    assert_eq!(token.balance(&client.address), INITIAL_BOND);
    assert_eq!(
        token.balance(&client.address),
        bond.bonded_amount - bond.slashed_amount
    );
}

#[test]
fn top_up_pulls_additional_tokens() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND + TOP_UP,
        INITIAL_BOND + TOP_UP,
    );
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);

    let bond = client.top_up(&TOP_UP);

    assert_eq!(bond.bonded_amount, INITIAL_BOND + TOP_UP);
    assert_eq!(token.balance(&identity), 0);
    assert_eq!(token.balance(&client.address), INITIAL_BOND + TOP_UP);
}

#[test]
fn withdraw_pushes_tokens_back_to_identity() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND,
        INITIAL_BOND,
    );
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);

    let bond = client.withdraw(&400_i128);

    assert_eq!(bond.bonded_amount, 600);
    assert_eq!(token.balance(&identity), 400);
    assert_eq!(token.balance(&client.address), 600);
    assert_eq!(
        token.balance(&client.address),
        bond.bonded_amount - bond.slashed_amount
    );
}

#[test]
fn withdraw_early_splits_net_and_penalty() {
    let (e, client, admin, identity, treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND,
        INITIAL_BOND,
    );
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);
    client.set_early_exit_config(&admin, &treasury, &1_000_u32);
    e.ledger().with_mut(|li| {
        li.timestamp = 100;
    });

    let amount = 500_i128;
    let bond = client.withdraw_early(&amount);

    let penalty = 45_i128;
    let net = amount - penalty;
    assert_eq!(bond.bonded_amount, 500);
    assert_eq!(token.balance(&identity), net);
    assert_eq!(token.balance(&treasury), penalty);
    assert_eq!(token.balance(&client.address), 500);
}

#[test]
fn withdraw_early_supports_treasury_equal_to_caller() {
    let (e, client, admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND,
        INITIAL_BOND,
    );
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);
    client.set_early_exit_config(&admin, &identity, &1_000_u32);
    e.ledger().with_mut(|li| {
        li.timestamp = 100;
    });

    client.withdraw_early(&500_i128);

    assert_eq!(token.balance(&identity), 500);
    assert_eq!(token.balance(&client.address), 500);
}

#[test]
fn create_bond_rejects_missing_allowance_and_rolls_back_state() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    let stellar = token::StellarAssetClient::new(&e, &token_id);
    stellar.mint(&identity, &INITIAL_BOND);

    assert!(client
        .try_create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64)
        .is_err());

    assert_eq!(token.balance(&identity), INITIAL_BOND);
    assert_eq!(token.balance(&client.address), 0);
    assert!(!e.storage().instance().has(&DataKey::Bond));
}

#[test]
fn top_up_rejects_insufficient_allowance_and_preserves_state() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND + TOP_UP,
        INITIAL_BOND,
    );
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);

    assert!(client.try_top_up(&TOP_UP).is_err());

    let bond = client.get_identity_state();
    assert_eq!(bond.bonded_amount, INITIAL_BOND);
    assert_eq!(token.balance(&identity), TOP_UP);
    assert_eq!(token.balance(&client.address), INITIAL_BOND);
}

#[test]
fn zero_amount_paths_reject_before_token_movement() {
    let (e, client, _admin, identity, _treasury, token_id) = setup();
    let token = token::TokenClient::new(&e, &token_id);
    mint_and_approve(
        &e,
        &client,
        &token_id,
        &identity,
        INITIAL_BOND,
        INITIAL_BOND,
    );

    assert!(client
        .try_create_bond(&identity, &0_i128, &DURATION, &false, &0_u64)
        .is_err());
    client.create_bond(&identity, &INITIAL_BOND, &DURATION, &false, &0_u64);
    assert!(client.try_top_up(&0_i128).is_err());
    assert!(client.try_withdraw(&0_i128).is_err());

    assert_eq!(token.balance(&identity), 0);
    assert_eq!(token.balance(&client.address), INITIAL_BOND);
}
