//! Ticket NFT Contract
//!
//! Minimal NFT implementation for event tickets.

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String};

// Error handling
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    UserAlreadyHasTicket = 1,
    InvalidTokenId = 2,
    Unauthorized = 3,
    RecipientAlreadyHasTicket = 4,
    NotInitialized = 5,
    TransferDisabled = 6,
    TransferCooldownActive = 7,
    AddressBlocked = 8,
}

/// Storage keys for the NFT contract
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Address with minting privileges
    Minter,
    /// Next token ID to mint
    NextTokenId,
    /// Contract name
    Name,
    /// Contract symbol
    Symbol,
    /// Base URI for metadata
    BaseUri,
    /// Token ownership: token_id -> owner
    Owner(u128),
    /// Balance: owner -> count
    Balance(Address),
    /// Per-token metadata URI
    TokenUri(u128),
    /// Global transfer toggle
    TransfersEnabled,
    /// Cooldown period in seconds
    TransferCooldown,
    /// Timestamp of last transfer: token_id -> timestamp
    LastTransfer(u128),
    /// Blocklisted addresses: address -> blocked
    Blocklist(Address),
    /// Original purchase price for resale cap: token_id -> price
    OriginalPrice(u128),
    /// Transfer fee in basis points (1/10000)
    TransferFeeBps,
    /// Organizer address for fees
    Organizer,
}

/// Ticket NFT Contract
///
/// Minimal NFT implementation for event tickets.
/// Each user can only hold one ticket per event.
#[contract]
pub struct TicketNft;

#[contractimpl]
impl TicketNft {
    /// Initialize the NFT contract with a minter address
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `minter` - Address that can mint new tickets
    pub fn __constructor(env: Env, minter: Address, name: String, symbol: String, base_uri: String) {
        env.storage().instance().set(&DataKey::Minter, &minter);
        env.storage().instance().set(&DataKey::NextTokenId, &1u128);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::BaseUri, &base_uri);
        
        // Default restrictions
        env.storage().instance().set(&DataKey::TransfersEnabled, &true);
        env.storage().instance().set(&DataKey::TransferCooldown, &0u64);
        env.storage().instance().set(&DataKey::TransferFeeBps, &0u32);
        env.storage().instance().set(&DataKey::Organizer, &minter);

        // Extend instance TTL
        env.storage()
            .instance()
            .extend_ttl(30 * 24 * 60 * 60 / 5, 100 * 24 * 60 * 60 / 5);
    }

    /// Mint a new ticket NFT to the recipient
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipient` - Address to receive the ticket
    ///
    /// # Returns
    /// The token ID of the minted ticket
    ///
    /// # Errors
    /// - If caller is not the minter
    /// - If recipient already has a ticket
    pub fn mint_ticket_nft(env: Env, recipient: Address) -> Result<u128, Error> {
        // Authorize: only minter can mint
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        // Check if user already has a ticket (one per user)
        let current_balance: u128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(recipient.clone()))
            .unwrap_or(0);

        if current_balance > 0 {
            return Err(Error::UserAlreadyHasTicket);
        }

        // Get next token ID
        let token_id: u128 = env
            .storage()
            .instance()
            .get(&DataKey::NextTokenId)
            .unwrap_or(1);

        env.storage()
            .persistent()
            .set(&DataKey::Owner(token_id), &recipient);

        // Extend persistent TTL for owner
        env.storage().persistent().extend_ttl(
            &DataKey::Owner(token_id),
            30 * 24 * 60 * 60 / 5,
            100 * 24 * 60 * 60 / 5,
        );

        env.storage()
            .persistent()
            .set(&DataKey::Balance(recipient.clone()), &1u128);

        // Extend persistent TTL for balance
        env.storage().persistent().extend_ttl(
            &DataKey::Balance(recipient),
            30 * 24 * 60 * 60 / 5,
            100 * 24 * 60 * 60 / 5,
        );

        env.storage()
            .instance()
            .set(&DataKey::NextTokenId, &(token_id + 1));

        // Record mint time as last transfer
        env.storage()
            .persistent()
            .set(&DataKey::LastTransfer(token_id), &env.ledger().timestamp());
        
        // Extend persistent TTL for LastTransfer
        env.storage().persistent().extend_ttl(
            &DataKey::LastTransfer(token_id),
            30 * 24 * 60 * 60 / 5,
            100 * 24 * 60 * 60 / 5,
        );

        // Extend instance TTL on update
        env.storage()
            .instance()
            .extend_ttl(30 * 24 * 60 * 60 / 5, 100 * 24 * 60 * 60 / 5);

        Ok(token_id)
    }

    /// Get the owner of a token
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `token_id` - The token ID to query
    pub fn owner_of(env: Env, token_id: u128) -> Result<Address, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Owner(token_id))
            .ok_or(Error::InvalidTokenId)
    }

    /// Get the balance of an owner
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `owner` - The address to query
    pub fn balance_of(env: Env, owner: Address) -> u128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(owner))
            .unwrap_or(0)
    }

    /// Transfer a ticket NFT from one address to another
    ///
    /// Enforces the one-ticket-per-user rule for the recipient.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `from` - Current owner of the ticket
    /// * `to` - Recipient address
    /// * `token_id` - The token ID to transfer
    ///
    /// # Errors
    /// - If `from` is not the owner
    /// - If `to` already has a ticket
    pub fn transfer_from(env: Env, from: Address, to: Address, token_id: u128) -> Result<(), Error> {
        from.require_auth();

        if !Self::is_valid(env.clone(), token_id) {
            return Err(Error::InvalidTokenId);
        }

        let owner = Self::owner_of(env.clone(), token_id)?;
        if owner != from {
            return Err(Error::Unauthorized);
        }

        if Self::balance_of(env.clone(), to.clone()) > 0 {
            return Err(Error::RecipientAlreadyHasTicket);
        }

        // Check global transfer toggle
        let enabled: bool = env.storage().instance().get(&DataKey::TransfersEnabled).unwrap_or(true);
        if !enabled {
            return Err(Error::TransferDisabled);
        }

        // Check blocklist
        if env.storage().persistent().has(&DataKey::Blocklist(from.clone())) || 
           env.storage().persistent().has(&DataKey::Blocklist(to.clone())) {
            return Err(Error::AddressBlocked);
        }

        // Check cooldown
        let cooldown: u64 = env.storage().instance().get(&DataKey::TransferCooldown).unwrap_or(0);
        if cooldown > 0 {
            let last_transfer: u64 = env.storage().persistent().get(&DataKey::LastTransfer(token_id)).unwrap_or(0);
            if env.ledger().timestamp() < last_transfer + cooldown {
                return Err(Error::TransferCooldownActive);
            }
        }

        // Update ownership
        env.storage()
            .persistent()
            .set(&DataKey::Owner(token_id), &to);

        // Update last transfer time
        env.storage()
            .persistent()
            .set(&DataKey::LastTransfer(token_id), &env.ledger().timestamp());

        // Update balances
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &0u128);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &1u128);
        
        Ok(())
    }

    /// Burn a ticket NFT, removing it from existence
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `token_id` - The token ID to burn
    ///
    /// # Errors
    /// - If token_id does not exist
    pub fn burn(env: Env, token_id: u128) -> Result<(), Error> {
        let owner = Self::owner_of(env.clone(), token_id)?;

        // Authorize: only owner can burn
        owner.require_auth();

        env.storage().persistent().remove(&DataKey::Owner(token_id));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(owner), &0u128);

        Ok(())
    }

    /// Check if a token is valid (exists and not burned)
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `token_id` - The token ID to check
    pub fn is_valid(env: Env, token_id: u128) -> bool {
        env.storage().persistent().has(&DataKey::Owner(token_id))
    }

    /// Get the minter address
    ///
    /// # Arguments
    /// * `env` - The contract environment
    pub fn get_minter(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)
    }

    /// Get the contract name
    pub fn name(env: Env) -> String {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }

    /// Get the contract symbol
    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&DataKey::Symbol).unwrap()
    }

    /// Get the token URI for a specific token
    pub fn token_uri(env: Env, token_id: u128) -> String {
        if !Self::is_valid(env.clone(), token_id) {
            panic!("Invalid token ID");
        }

        if let Some(uri) = env.storage().persistent().get(&DataKey::TokenUri(token_id)) {
            uri
        } else {
            env.storage().instance().get(&DataKey::BaseUri).unwrap()
        }
    }

    /// Set the token URI for a specific token (minter-only)
    pub fn set_token_uri(env: Env, token_id: u128, uri: String) -> Result<(), Error> {
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        if !Self::is_valid(env.clone(), token_id) {
            return Err(Error::InvalidTokenId);
        }

        env.storage().persistent().set(&DataKey::TokenUri(token_id), &uri);
        
        // Extend persistent TTL for URI
        env.storage().persistent().extend_ttl(
            &DataKey::TokenUri(token_id),
            30 * 24 * 60 * 60 / 5,
            100 * 24 * 60 * 60 / 5,
        );

        Ok(())
    }

    /// Update transfer restrictions (minter-only)
    pub fn set_transfer_restrictions(env: Env, enabled: bool, cooldown: u64, fee_bps: u32) -> Result<(), Error> {
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        env.storage().instance().set(&DataKey::TransfersEnabled, &enabled);
        env.storage().instance().set(&DataKey::TransferCooldown, &cooldown);
        env.storage().instance().set(&DataKey::TransferFeeBps, &fee_bps);

        Ok(())
    }

    /// Set organizer address (minter-only)
    pub fn set_organizer(env: Env, organizer: Address) -> Result<(), Error> {
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        env.storage().instance().set(&DataKey::Organizer, &organizer);

        Ok(())
    }

    /// Get transfer fee bps
    pub fn get_transfer_fee_bps(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::TransferFeeBps).unwrap_or(0)
    }

    /// Get organizer address
    pub fn get_organizer(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Organizer).unwrap()
    }

    /// Add or remove from blocklist (minter-only)
    pub fn set_blocklist(env: Env, address: Address, blocked: bool) -> Result<(), Error> {
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        if blocked {
            env.storage().persistent().set(&DataKey::Blocklist(address.clone()), &true);
            env.storage().persistent().extend_ttl(
                &DataKey::Blocklist(address),
                30 * 24 * 60 * 60 / 5,
                100 * 24 * 60 * 60 / 5,
            );
        } else {
            env.storage().persistent().remove(&DataKey::Blocklist(address));
        }

        Ok(())
    }

    /// Set original price for per-token resale cap (minter-only)
    pub fn set_original_price(env: Env, token_id: u128, price: i128) -> Result<(), Error> {
        let minter: Address = env.storage().instance().get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();

        if !Self::is_valid(env.clone(), token_id) {
            return Err(Error::InvalidTokenId);
        }

        env.storage().persistent().set(&DataKey::OriginalPrice(token_id), &price);
        env.storage().persistent().extend_ttl(
            &DataKey::OriginalPrice(token_id),
            30 * 24 * 60 * 60 / 5,
            100 * 24 * 60 * 60 / 5,
        );

        Ok(())
    }

    /// Get original price of a token
    pub fn get_original_price(env: Env, token_id: u128) -> i128 {
        env.storage().persistent().get(&DataKey::OriginalPrice(token_id)).unwrap_or(0)
    }

    /// Check if an address is blocked
    pub fn is_blocked(env: Env, address: Address) -> bool {
        env.storage().persistent().has(&DataKey::Blocklist(address))
    }
}

#[cfg(test)]
mod test;
