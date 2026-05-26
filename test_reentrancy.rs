#![cfg(test)]

use super::*;
use credence_errors::ContractError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Symbol};

fn setup() -> (Env, Address, CredenceBondClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredenceBond, ());
    let client = CredenceBondClient::new(&env, &contract_id);
    (env, admin, client)
}

#[test]
fn test_reentrancy_detected_on_withdraw_bond() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);

    // Create bond for owner
    let _ = client.create_bond(&owner, &100_i128, &1000_u64);

    // Manually set the locked flag to simulate re-entrancy
    env.storage()
        .instance()
        .set(&Symbol::new(&env, "locked"), &true);

    let err = client.try_withdraw_bond(&owner).unwrap_err().unwrap();
    assert_eq!(err, ContractError::ReentrancyDetected);
}
