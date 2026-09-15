#![cfg(test)]
//! Unit tests for the audit-log contract. Run with `cargo test`.

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    vec, Address, BytesN, Env, IntoVal, String,
};

fn setup() -> (Env, AuditorContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AuditorContract, ());
    let client = AuditorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

#[test]
fn initialize_sets_admin() {
    let (_env, client, admin) = setup();
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_entry_count(), 0);
    assert_eq!(client.is_paused(), false);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // AlreadyInitialized
fn initialize_twice_panics() {
    let (_env, client, admin) = setup();
    client.initialize(&admin);
}

#[test]
fn grant_and_revoke_auditor() {
    let (env, client, admin) = setup();
    let auditor = Address::generate(&env);
    assert_eq!(client.is_auditor(&auditor), false);

    client.grant_auditor(&auditor);
    assert_eq!(client.is_auditor(&auditor), true);

    client.revoke_auditor(&auditor);
    // `events().all()` reflects only the most recent contract invocation, so
    // capture it right after revoke (issue #15) — before any further calls.
    let events = env.events().all();
    assert_eq!(client.is_auditor(&auditor), false);

    assert_eq!(
        events,
        vec![
            &env,
            (
                client.address.clone(),
                (symbol_short!("revoked"), admin.clone()).into_val(&env),
                auditor.clone().into_val(&env)
            ),
        ]
    );
}

#[test]
fn record_happy_path_and_chain() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);

    let id0 = client.record(
        &auditor,
        &subject,
        &Category::Payment,
        &hash(&env, 1),
        &String::from_str(&env, "first entry"),
        &(env.ledger().timestamp() + 1000),
    );
    assert_eq!(id0, 0);

    let id1 = client.record(
        &auditor,
        &subject,
        &Category::Compliance,
        &hash(&env, 2),
        &String::from_str(&env, "second entry"),
        &(env.ledger().timestamp() + 1000),
    );
    assert_eq!(id1, 1);
    assert_eq!(client.get_entry_count(), 2);

    // Genesis entry links to zero; second links to the first's hash.
    let e0 = client.get_entry(&0);
    let e1 = client.get_entry(&1);
    assert_eq!(e0.prev_hash, hash(&env, 0));
    assert_eq!(e1.prev_hash, e0.entry_hash);
    assert_eq!(client.get_last_hash(), e1.entry_hash);

    assert!(client.verify_chain(&0, &1));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // NotAuthorized
fn record_requires_auditor_role() {
    let (env, client, _admin) = setup();
    let not_auditor = Address::generate(&env);
    let subject = Address::generate(&env);
    client.record(
        &not_auditor,
        &subject,
        &Category::Other,
        &hash(&env, 1),
        &String::from_str(&env, "x"),
        &(env.ledger().timestamp() + 100),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // Paused
fn record_blocked_when_paused() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    client.pause();
    client.record(
        &auditor,
        &subject,
        &Category::Other,
        &hash(&env, 1),
        &String::from_str(&env, "x"),
        &(env.ledger().timestamp() + 100),
    );
}

#[test]
fn pause_unpause_cycle() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);

    client.pause();
    assert!(client.is_paused());
    client.unpause();
    assert!(!client.is_paused());

    // Recording works again after unpause.
    let id = client.record(
        &auditor,
        &subject,
        &Category::Security,
        &hash(&env, 7),
        &String::from_str(&env, "ok"),
        &(env.ledger().timestamp() + 100),
    );
    assert_eq!(id, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // InvalidDataHash
fn record_rejects_zero_data_hash() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    client.record(
        &auditor,
        &subject,
        &Category::Other,
        &hash(&env, 0), // all-zero — rejected (#11)
        &String::from_str(&env, "x"),
        &(env.ledger().timestamp() + 100),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // InvalidExpiry
fn record_rejects_expiry_equal_to_now() {
    // Off-by-one boundary (#12/#13): expires_at == now must be rejected.
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    let now = env.ledger().timestamp();
    client.record(
        &auditor,
        &subject,
        &Category::Other,
        &hash(&env, 1),
        &String::from_str(&env, "x"),
        &now, // exactly now — already stale
    );
}

#[test]
fn record_accepts_expiry_one_second_ahead() {
    // The matching boundary case: now + 1 is accepted.
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    let now = env.ledger().timestamp();
    let id = client.record(
        &auditor,
        &subject,
        &Category::Other,
        &hash(&env, 1),
        &String::from_str(&env, "x"),
        &(now + 1),
    );
    assert_eq!(id, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // MemoTooLong
fn record_rejects_oversized_memo() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    // Build a memo longer than MAX_MEMO_LEN (512).
    let big = core::str::from_utf8(&[b'a'; 513]).unwrap();
    client.record(
        &auditor,
        &subject,
        &Category::Other,
        &hash(&env, 1),
        &String::from_str(&env, big),
        &(env.ledger().timestamp() + 100),
    );
}

#[test]
fn get_entries_pagination() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    for i in 0..5u8 {
        client.record(
            &auditor,
            &subject,
            &Category::Other,
            &hash(&env, i + 1),
            &String::from_str(&env, "e"),
            &(env.ledger().timestamp() + 100),
        );
    }
    let page = client.get_entries(&1, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().id, 1);
    assert_eq!(page.get(1).unwrap().id, 2);

    // Past the end returns empty.
    let empty = client.get_entries(&99, &10);
    assert_eq!(empty.len(), 0);
}

#[test]
fn verify_chain_detects_tampering() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);
    for i in 0..3u8 {
        client.record(
            &auditor,
            &subject,
            &Category::Other,
            &hash(&env, i + 1),
            &String::from_str(&env, "e"),
            &(env.ledger().timestamp() + 100),
        );
    }
    assert!(client.verify_chain(&0, &2));

    // Tamper: directly overwrite entry #1's stored data_hash inside the
    // contract's storage, leaving its (now stale) entry_hash in place.
    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        let mut e1: AuditEntry = env
            .storage()
            .persistent()
            .get(&DataKey::Entry(1))
            .unwrap();
        e1.data_hash = hash(&env, 200); // mutate a hashed field
        env.storage().persistent().set(&DataKey::Entry(1), &e1);
    });

    // The chain must now fail verification.
    assert!(!client.verify_chain(&0, &2));
}

#[test]
fn advancing_ledger_time_still_records() {
    let (env, client, _admin) = setup();
    let auditor = Address::generate(&env);
    client.grant_auditor(&auditor);
    let subject = Address::generate(&env);

    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let id = client.record(
        &auditor,
        &subject,
        &Category::Payment,
        &hash(&env, 9),
        &String::from_str(&env, "later"),
        &2_000_000,
    );
    let e = client.get_entry(&id);
    assert_eq!(e.timestamp, 1_000_000);
    assert_eq!(e.expires_at, 2_000_000);
}
