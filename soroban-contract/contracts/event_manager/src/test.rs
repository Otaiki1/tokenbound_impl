#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, BytesN, Env};

#[contract]
pub struct MockContract;

#[contractimpl]
impl MockContract {
    pub fn deploy_ticket(env: Env, _minter: Address, _salt: BytesN<32>) -> Address {
        env.current_contract_address()
    }

    pub fn mint_ticket_nft(_env: Env, _recipient: Address) -> u128 {
        1
    }

    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
}

fn setup() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn create_sample_event(env: &Env, client: &EventManagerClient<'_>, payment_token: &Address) -> u32 {
    let organizer = Address::generate(env);
    client.create_event(
        &organizer,
        &String::from_str(env, "Stellar Meetup"),
        &String::from_str(env, "Conference"),
        &(env.ledger().timestamp() + 86_400),
        &(env.ledger().timestamp() + 172_800),
        &100i128,
        &20u128,
        payment_token,
    )
}

#[test]
fn test_create_event() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let organizer = Address::generate(&env);

    let event_id = client.create_event(
        &organizer,
        &String::from_str(&env, "Rust Conference 2026"),
        &String::from_str(&env, "Conference"),
        &(env.ledger().timestamp() + 86_400),
        &(env.ledger().timestamp() + 172_800),
        &1000_0000000,
        &500u128,
        &mock_addr,
    );

    let event = client.get_event(&event_id);
    assert_eq!(event_id, 0);
    assert_eq!(event.id, 0);
    assert_eq!(event.total_tickets, 500);
    assert_eq!(event.tickets_sold, 0);
    assert_eq!(event.payment_token, mock_addr);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_create_event_rejects_past_start_date() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    env.ledger().set_timestamp(1000);

    let organizer = Address::generate(&env);
    client.create_event(
        &organizer,
        &String::from_str(&env, "Past Event"),
        &String::from_str(&env, "Conference"),
        &999u64,
        &2000u64,
        &1000_0000000,
        &100u128,
        &mock_addr,
    );
}

#[test]
fn test_cancel_event_marks_event_canceled() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let event_id = create_sample_event(&env, &client, &mock_addr);

    client.cancel_event(&event_id);

    let event = client.get_event(&event_id);
    assert!(event.is_canceled);
}

#[test]
fn test_purchase_ticket_increments_tickets_sold() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let event_id = create_sample_event(&env, &client, &mock_addr);
    let buyer = Address::generate(&env);

    client.purchase_ticket(&buyer, &event_id);

    let event = client.get_event(&event_id);
    let purchase = client.get_buyer_purchase(&event_id, &buyer).unwrap();

    assert_eq!(event.tickets_sold, 1);
    assert_eq!(purchase.quantity, 1);
    assert_eq!(purchase.total_paid, 100);
}

#[test]
fn test_purchase_tickets_applies_group_discount() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let event_id = create_sample_event(&env, &client, &mock_addr);
    let buyer = Address::generate(&env);

    client.purchase_tickets(&buyer, &event_id, &5u128);

    let event = client.get_event(&event_id);
    let purchase = client.get_buyer_purchase(&event_id, &buyer).unwrap();

    assert_eq!(event.tickets_sold, 5);
    assert_eq!(purchase.quantity, 5);
    assert_eq!(purchase.total_paid, 475);
}

#[test]
fn test_batch_purchase_refund_uses_total_paid() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let event_id = create_sample_event(&env, &client, &mock_addr);
    let buyer = Address::generate(&env);

    client.purchase_tickets(&buyer, &event_id, &10u128);
    client.cancel_event(&event_id);
    client.claim_refund(&buyer, &event_id);
}

#[test]
#[should_panic(expected = "Refund already claimed")]
fn test_refund_cannot_be_claimed_twice() {
    let env = setup();
    let contract_id = env.register(EventManager, ());
    let client = EventManagerClient::new(&env, &contract_id);
    let mock_addr = env.register(MockContract, ());
    client.initialize(&mock_addr);
    let event_id = create_sample_event(&env, &client, &mock_addr);
    let buyer = Address::generate(&env);

    client.purchase_tickets(&buyer, &event_id, &2u128);
    client.cancel_event(&event_id);
    client.claim_refund(&buyer, &event_id);
    client.claim_refund(&buyer, &event_id);
}
