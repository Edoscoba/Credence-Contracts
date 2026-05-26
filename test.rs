#![cfg(test)]

use super::*;
use credence_errors::ContractError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fn setup() -> (Env, Address, CredenceBondClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredenceBond, ());
    let client = CredenceBondClient::new(&env, &contract_id);
    (env, admin, client)
}

#[test]
fn test_not_initialized_errors() {
    let (env, _admin, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    let err = client
        .try_set_early_exit_config(&admin, &treasury, &500_u32)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::NotInitialized);
}

#[test]
fn test_bond_not_found_and_insufficient_balance() {
    let (env, _admin, client) = setup();

    // No bond exists yet
    let err = client.try_get_identity_state().unwrap_err().unwrap();
    assert_eq!(err, ContractError::BondNotFound);

    // Create a small bond and attempt to withdraw more than available
    let owner = Address::generate(&env);
    let _bond = client.create_bond(&owner, &100_i128, &1000_u64);
    let err2 = client.try_withdraw(&200_i128).unwrap_err().unwrap();
    assert_eq!(err2, ContractError::InsufficientBalance);
}

#[test]
fn test_request_withdrawal_not_rolling_and_already_requested() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);

    // Non-rolling bond -> NotRollingBond
    let _bond = client.create_bond(&owner, &100_i128, &1000_u64);
    let err = client.try_request_withdrawal().unwrap_err().unwrap();
    assert_eq!(err, ContractError::NotRollingBond);

    // Rolling bond: first request succeeds, second fails with WithdrawalAlreadyRequested
    let _rb = client.create_bond_with_rolling(&owner, &100_i128, &1000_u64, &true, &10_u64);
    let _ = client.request_withdrawal();
    let err2 = client.try_request_withdrawal().unwrap_err().unwrap();
    assert_eq!(err2, ContractError::WithdrawalAlreadyRequested);
}
