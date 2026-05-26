#![cfg(test)]

use super::*;
use credence_errors::ContractError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};

fn setup() -> (Env, Address, CredenceBondClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredenceBond, ());
    let client = CredenceBondClient::new(&env, &contract_id);
    client.initialize(&admin);
    (env, admin, client)
}

#[test]
fn test_unauthorized_attester_and_not_found_revocation() {
    let (env, _admin, client) = setup();
    let attester = Address::generate(&env);
    let subject = Address::generate(&env);
    let data = String::from_str(&env, "payload");

    // Attester not registered -> UnauthorizedAttester
    let err = client
        .try_add_attestation(&attester, &subject, &data)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::UnauthorizedAttester);

    // Revoking non-existent attestation -> AttestationNotFound
    let err2 = client.try_revoke_attestation(&attester, &1_u64).unwrap_err().unwrap();
    assert_eq!(err2, ContractError::AttestationNotFound);
}

#[test]
fn test_attestation_revocation_flow_errors() {
    let (env, admin, client) = setup();
    let attester = Address::generate(&env);
    let other = Address::generate(&env);
    let subject = Address::generate(&env);
    let data = String::from_str(&env, "payload");

    // Register attester
    client.register_attester(&admin, &attester);

    // Add attestation
    let a = client.add_attestation(&attester, &subject, &data);

    // Non-original attester attempting revoke -> NotOriginalAttester
    let err = client.try_revoke_attestation(&other, &a.id).unwrap_err().unwrap();
    assert_eq!(err, ContractError::NotOriginalAttester);

    // Original attester revokes successfully, then double-revoke errors
    client.revoke_attestation(&attester, &a.id);
    let err2 = client.try_revoke_attestation(&attester, &a.id).unwrap_err().unwrap();
    assert_eq!(err2, ContractError::AttestationAlreadyRevoked);
}
