#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

use upgradeable::UPGRADE_DELAY_LEDGERS;

const START_LEDGER: u32 = 1_000;

fn setup() -> (Env, UpgradeableReferenceClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(START_LEDGER);

    let admin = Address::generate(&env);
    let contract_id = env.register(UpgradeableReference, (admin.clone(),));
    let client = UpgradeableReferenceClient::new(&env, &contract_id);
    (env, client, admin)
}

fn dummy_hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

// ── Initialisation & version ─────────────────────────────────────────────

#[test]
fn initial_state_is_correct() {
    let (_env, client, admin) = setup();
    assert_eq!(client.version(), 1);
    assert_eq!(client.admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.get(), 0);
}

// ── Pause guard ──────────────────────────────────────────────────────────

#[test]
fn increment_works_when_unpaused() {
    let (env, client, _admin) = setup();
    let caller = Address::generate(&env);
    assert_eq!(client.increment(&caller), 1);
    assert_eq!(client.increment(&caller), 2);
    assert_eq!(client.get(), 2);
}

#[test]
#[should_panic(expected = "contract is paused")]
fn increment_blocked_when_paused() {
    let (env, client, _admin) = setup();
    client.pause();
    let caller = Address::generate(&env);
    client.increment(&caller);
}

#[test]
fn unpause_restores_increment() {
    let (env, client, _admin) = setup();
    client.pause();
    client.unpause();
    let caller = Address::generate(&env);
    assert_eq!(client.increment(&caller), 1);
}

// ── Upgrade timelock ─────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "no pending upgrade")]
fn commit_without_schedule_fails() {
    let (_env, client, _admin) = setup();
    client.commit_upgrade();
}

#[test]
#[should_panic(expected = "timelock not elapsed")]
fn commit_before_delay_fails() {
    let (env, client, _admin) = setup();
    let new_hash = dummy_hash(&env, 0xAB);
    client.schedule_upgrade(&new_hash);

    // Advance just shy of the delay.
    env.ledger()
        .set_sequence_number(START_LEDGER + UPGRADE_DELAY_LEDGERS - 1);
    client.commit_upgrade();
}

#[test]
#[should_panic(expected = "no pending upgrade")]
fn cancel_then_commit_fails() {
    let (env, client, _admin) = setup();
    let new_hash = dummy_hash(&env, 0xCD);
    client.schedule_upgrade(&new_hash);
    client.cancel_upgrade();

    env.ledger()
        .set_sequence_number(START_LEDGER + UPGRADE_DELAY_LEDGERS);
    client.commit_upgrade();
}

// ── Admin transfer ───────────────────────────────────────────────────────

#[test]
fn transfer_admin_updates_owner() {
    let (env, client, _admin) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin);
    assert_eq!(client.admin(), new_admin);
}

#[test]
fn version_unchanged_by_admin_transfer() {
    let (env, client, _admin) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin);
    assert_eq!(client.version(), 1);
}
