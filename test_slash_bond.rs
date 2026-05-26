use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, IntoVal, Symbol, TryIntoVal, Val};

const AMOUNT: i128 = 1_000;
const DURATION: u64 = 1_000;

fn setup() -> (Env, CredenceBondClient<'static>, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(CredenceBond, ());
    let client = CredenceBondClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let identity = Address::generate(&e);
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract(token_admin);
    client.initialize(&admin, &token_id);
    let stellar = token::StellarAssetClient::new(&e, &token_id);
    let token = token::TokenClient::new(&e, &token_id);
    stellar.mint(&identity, &AMOUNT);
    token.approve(&identity, &client.address, &AMOUNT, &1_000_u32);
    client.create_bond(&identity, &AMOUNT, &DURATION, &false, &0_u64);
    (e, client, admin, identity)
}

fn last_bond_slashed_event(e: &Env) -> (Address, i128, i128) {
    let events = e.events().all();
    let (_, topics, data) = events.get_unchecked(events.len() - 1);
    let event_name: Val = Symbol::new(e, "bond_slashed").into_val(e);
    assert_eq!(topics.get_unchecked(0), event_name);
    data.try_into_val(e).unwrap()
}

#[test]
fn slash_bond_rejects_negative_amount() {
    let (_, client, admin, _) = setup();

    assert!(client.try_slash_bond(&admin, &-1_i128).is_err());

    let bond = client.get_identity_state();
    assert_eq!(bond.slashed_amount, 0);
    assert!(!client.is_locked());
}

#[test]
fn slash_bond_rejects_zero_amount() {
    let (_, client, admin, _) = setup();

    assert!(client.try_slash_bond(&admin, &0_i128).is_err());

    let bond = client.get_identity_state();
    assert_eq!(bond.slashed_amount, 0);
    assert!(!client.is_locked());
}

#[test]
fn slash_bond_emits_canonical_event_payload() {
    let (e, client, admin, identity) = setup();

    let total_slashed = client.slash_bond(&admin, &250_i128);

    assert_eq!(total_slashed, 250);
    assert_eq!(client.get_identity_state().slashed_amount, 250);
    assert_eq!(last_bond_slashed_event(&e), (identity, 250, 250));
}

#[test]
fn slash_bond_allows_exact_cap() {
    let (e, client, admin, identity) = setup();

    let total_slashed = client.slash_bond(&admin, &AMOUNT);

    assert_eq!(total_slashed, AMOUNT);
    assert_eq!(client.get_identity_state().slashed_amount, AMOUNT);
    assert_eq!(last_bond_slashed_event(&e), (identity, AMOUNT, AMOUNT));
}

#[test]
fn slash_bond_rejects_over_cap_and_preserves_state() {
    let (_, client, admin, _) = setup();
    client.slash_bond(&admin, &750_i128);

    assert!(client.try_slash_bond(&admin, &251_i128).is_err());

    let bond = client.get_identity_state();
    assert_eq!(bond.slashed_amount, 750);
    assert!(!client.is_locked());
}

#[test]
fn slash_bond_keeps_slashed_amount_monotonic_and_bounded() {
    let (_, client, admin, _) = setup();

    let first = client.slash_bond(&admin, &100_i128);
    let second = client.slash_bond(&admin, &300_i128);
    let third = client.slash_bond(&admin, &600_i128);

    assert_eq!(first, 100);
    assert_eq!(second, 400);
    assert_eq!(third, AMOUNT);

    let bond = client.get_identity_state();
    assert_eq!(bond.slashed_amount, AMOUNT);
    assert!(bond.slashed_amount <= bond.bonded_amount);
}
