use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String, Vec};

fn setup() -> (Env, CredenceBondClient<'static>, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(CredenceBond, ());
    let client = CredenceBondClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract(token_admin);
    client.initialize(&admin, &token_id);
    (e, client, admin)
}

fn add(
    e: &Env,
    client: &CredenceBondClient<'static>,
    attester: &Address,
    subject: &Address,
    data: &str,
) -> Attestation {
    client.add_attestation(
        attester,
        subject,
        &String::from_str(e, data),
        &client.get_nonce(attester),
    )
}

fn revoke(client: &CredenceBondClient<'static>, attester: &Address, id: u64) {
    client.revoke_attestation(attester, &id, &client.get_nonce(attester));
}

fn assert_live_index(
    client: &CredenceBondClient<'static>,
    subject: &Address,
    expected_ids: &[u64],
) {
    let ids: Vec<u64> = client.get_subject_attestations(subject);
    assert_eq!(client.get_subject_attestation_count(subject), ids.len());
    assert_eq!(ids.len(), expected_ids.len() as u32);

    for (i, expected_id) in expected_ids.iter().enumerate() {
        let id = ids.get_unchecked(i as u32);
        assert_eq!(id, *expected_id);
        assert!(!client.get_attestation(&id).revoked);
    }
}

#[test]
fn add_n_revoke_m_keeps_count_and_subject_index_consistent() {
    let (e, client, admin) = setup();
    let attester = Address::generate(&e);
    let subject = Address::generate(&e);
    client.register_attester(&attester);

    let a0 = add(&e, &client, &attester, &subject, "a0");
    let a1 = add(&e, &client, &attester, &subject, "a1");
    let a2 = add(&e, &client, &attester, &subject, "a2");
    let a3 = add(&e, &client, &attester, &subject, "a3");
    let a4 = add(&e, &client, &attester, &subject, "a4");
    assert_live_index(&client, &subject, &[a0.id, a1.id, a2.id, a3.id, a4.id]);

    revoke(&client, &attester, a1.id);
    revoke(&client, &attester, a3.id);

    assert!(client.get_attestation(&a1.id).revoked);
    assert!(client.get_attestation(&a3.id).revoked);
    assert_live_index(&client, &subject, &[a0.id, a2.id, a4.id]);

    // Keep admin live so setup authorization is explicitly used by the test.
    assert!(client.is_attester(&attester));
    assert_ne!(admin, attester);
}

#[test]
fn revoke_all_leaves_empty_live_index_and_zero_count() {
    let (e, client, _) = setup();
    let attester = Address::generate(&e);
    let subject = Address::generate(&e);
    client.register_attester(&attester);

    let a0 = add(&e, &client, &attester, &subject, "all0");
    let a1 = add(&e, &client, &attester, &subject, "all1");

    revoke(&client, &attester, a0.id);
    revoke(&client, &attester, a1.id);

    assert_live_index(&client, &subject, &[]);
}

#[test]
fn re_add_after_revoke_uses_new_live_id_without_drift() {
    let (e, client, _) = setup();
    let attester = Address::generate(&e);
    let subject = Address::generate(&e);
    client.register_attester(&attester);

    let first = add(&e, &client, &attester, &subject, "same-claim");
    revoke(&client, &attester, first.id);

    let second = add(&e, &client, &attester, &subject, "same-claim");

    assert_ne!(first.id, second.id);
    assert!(client.get_attestation(&first.id).revoked);
    assert_live_index(&client, &subject, &[second.id]);
}

#[test]
#[should_panic]
fn revoke_nonexistent_attestation_does_not_change_accounting() {
    let (e, client, _) = setup();
    let attester = Address::generate(&e);
    client.register_attester(&attester);

    revoke(&client, &attester, 9_999);
}

#[test]
fn long_add_revoke_sequence_has_no_count_drift() {
    let (e, client, _) = setup();
    let attester = Address::generate(&e);
    let subject = Address::generate(&e);
    client.register_attester(&attester);

    let a0 = add(&e, &client, &attester, &subject, "long0");
    let a1 = add(&e, &client, &attester, &subject, "long1");
    let a2 = add(&e, &client, &attester, &subject, "long2");
    let a3 = add(&e, &client, &attester, &subject, "long3");
    let a4 = add(&e, &client, &attester, &subject, "long4");
    let a5 = add(&e, &client, &attester, &subject, "long5");
    let a6 = add(&e, &client, &attester, &subject, "long6");
    let a7 = add(&e, &client, &attester, &subject, "long7");

    revoke(&client, &attester, a0.id);
    revoke(&client, &attester, a2.id);
    revoke(&client, &attester, a4.id);
    revoke(&client, &attester, a6.id);

    assert_live_index(&client, &subject, &[a1.id, a3.id, a5.id, a7.id]);
}
