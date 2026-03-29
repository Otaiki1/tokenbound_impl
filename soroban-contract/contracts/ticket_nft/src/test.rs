#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Env, String};

#[test]
fn test_minting() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");

    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    // Mint first ticket
    let token_id1 = client.mint_ticket_nft(&user1);
    assert_eq!(token_id1, 1);
    assert_eq!(client.owner_of(&token_id1), user1);
    assert_eq!(client.balance_of(&user1), 1);

    // Mint second ticket
    let token_id2 = client.mint_ticket_nft(&user2);
    assert_eq!(token_id2, 2);
    assert_eq!(client.owner_of(&token_id2), user2);
    assert_eq!(client.balance_of(&user2), 1);
}

#[test]
fn test_cannot_mint_twice_to_same_user() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    client.mint_ticket_nft(&user);
    let result = client.try_mint_ticket_nft(&user);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user1);

    client.transfer_from(&user1, &user2, &token_id);

    assert_eq!(client.owner_of(&token_id), user2);
    assert_eq!(client.balance_of(&user1), 0);
    assert_eq!(client.balance_of(&user2), 1);
}

#[test]
fn test_cannot_transfer_to_user_with_ticket() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id1 = client.mint_ticket_nft(&user1);
    let _token_id2 = client.mint_ticket_nft(&user2);

    let result = client.try_transfer_from(&user1, &user2, &token_id1);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
#[should_panic] // Only authorized minter can mint
fn test_only_minter_can_mint() {
    let env = Env::default();
    // env.mock_all_auths(); // Don't mock auth to test failure

    let minter = Address::generate(&env);
    let user = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    // Without mock_all_auths, require_auth() will fail for the minter
    let _ = client.mint_ticket_nft(&user);
}

#[test]
fn test_burn() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user);
    assert!(client.is_valid(&token_id));

    client.burn(&token_id);
    assert!(!client.is_valid(&token_id));
    assert_eq!(client.balance_of(&user), 0);
}

#[test]
fn test_cannot_transfer_burned_token() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user1);
    client.burn(&token_id);

    let result = client.try_transfer_from(&user1, &user2, &token_id);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");

    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    assert_eq!(client.name(), name);
    assert_eq!(client.symbol(), symbol);

    let token_id = client.mint_ticket_nft(&user);
    
    // Check default URI (base_uri)
    assert_eq!(client.token_uri(&token_id), base_uri);

    // Set custom URI
    let custom_uri = String::from_str(&env, "https://api.example.com/ticket/1-custom");
    client.set_token_uri(&token_id, &custom_uri);
    assert_eq!(client.token_uri(&token_id), custom_uri);
}

#[test]
fn test_transfer_cooldown() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    // Set 1 day cooldown
    client.set_transfer_restrictions(&true, &86400, &0);

    let token_id = client.mint_ticket_nft(&user1);
    
    // Try to transfer immediately
    let result = client.try_transfer_from(&user1, &user2, &token_id);
    assert!(result.is_err());
    // assert_eq!(result.unwrap().unwrap_err(), Error::TransferCooldownActive.into());

    // Advance time by 1 day
    env.ledger().set_timestamp(86400 + 1);
    
    client.transfer_from(&user1, &user2, &token_id);
    assert_eq!(client.owner_of(&token_id), user2);
}

#[test]
fn test_transfers_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user1);

    // Disable transfers
    client.set_transfer_restrictions(&false, &0, &0);
    
    let result = client.try_transfer_from(&user1, &user2, &token_id);
    assert!(result.is_err());
    // assert_eq!(result.unwrap().unwrap_err(), Error::TransferDisabled.into());
}

#[test]
fn test_blocklist() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user1);

    // Block user2
    client.set_blocklist(&user2, &true);
    assert!(client.is_blocked(&user2));
    
    let result = client.try_transfer_from(&user1, &user2, &token_id);
    assert!(result.is_err());
    // assert_eq!(result.unwrap().unwrap_err(), Error::AddressBlocked.into());

    // Unblock user2
    client.set_blocklist(&user2, &false);
    assert!(!client.is_blocked(&user2));
    
    client.transfer_from(&user1, &user2, &token_id);
    assert_eq!(client.owner_of(&token_id), user2);
}

#[test]
fn test_original_price() {
    let env = Env::default();
    env.mock_all_auths();

    let minter = Address::generate(&env);
    let user = Address::generate(&env);

    let name = String::from_str(&env, "Ticket NFT");
    let symbol = String::from_str(&env, "TKT");
    let base_uri = String::from_str(&env, "https://api.example.com/ticket/");
    let contract_id = env.register(TicketNft, (minter.clone(), name.clone(), symbol.clone(), base_uri.clone()));
    let client = TicketNftClient::new(&env, &contract_id);

    let token_id = client.mint_ticket_nft(&user);
    
    client.set_original_price(&token_id, &100i128);
    assert_eq!(client.get_original_price(&token_id), 100i128);
}
