#![no_std]

use core::convert::TryFrom;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, IntoVal, String,
    Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    EventNotFound = 2,
    EventAlreadyCanceled = 3,
    CannotSellMoreTickets = 4,
    InvalidStartDate = 5,
    InvalidEndDate = 6,
    NegativeTicketPrice = 7,
    InvalidTicketCount = 8,
    CounterOverflow = 9,
    FactoryNotInitialized = 10,
}

#[contracttype]
pub enum DataKey {
    Event(u32),
    EventCounter,
    TicketFactory,
    RefundClaimed(u32, Address),
    EventBuyers(u32),
    BuyerPurchase(u32, Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub id: u32,
    pub theme: String,
    pub organizer: Address,
    pub event_type: String,
    pub total_tickets: u128,
    pub tickets_sold: u128,
    pub ticket_price: i128,
    pub start_date: u64,
    pub end_date: u64,
    pub is_canceled: bool,
    pub ticket_nft_addr: Address,
    pub payment_token: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuyerPurchase {
    pub quantity: u128,
    pub total_paid: i128,
}

#[contract]
pub struct EventManager;

#[contractimpl]
impl EventManager {
    pub fn initialize(env: Env, ticket_factory: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::TicketFactory) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage()
            .instance()
            .set(&DataKey::TicketFactory, &ticket_factory);
        env.storage().instance().set(&DataKey::EventCounter, &0u32);
        env.storage()
            .instance()
            .extend_ttl(Self::ttl_threshold(), Self::ttl_extend_to());

        Ok(())
    }

    pub fn create_event(
        env: Env,
        organizer: Address,
        theme: String,
        event_type: String,
        start_date: u64,
        end_date: u64,
        ticket_price: i128,
        total_tickets: u128,
        payment_token: Address,
    ) -> Result<u32, Error> {
        organizer.require_auth();

        Self::validate_event_params(&env, start_date, end_date, ticket_price, total_tickets)?;

        let event_id = Self::get_and_increment_counter(&env)?;
        let ticket_nft_addr =
            Self::deploy_ticket_nft(&env, event_id).ok_or(Error::FactoryNotInitialized)?;

        let event = Event {
            id: event_id,
            theme,
            organizer: organizer.clone(),
            event_type,
            total_tickets,
            tickets_sold: 0,
            ticket_price,
            start_date,
            end_date,
            is_canceled: false,
            ticket_nft_addr: ticket_nft_addr.clone(),
            payment_token,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        Self::extend_persistent_ttl(&env, &DataKey::Event(event_id));

        env.events().publish(
            (Symbol::new(&env, "event_created"),),
            (event_id, organizer, ticket_nft_addr),
        );

        Ok(event_id)
    }

    pub fn get_event(env: Env, event_id: u32) -> Result<Event, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(Error::EventNotFound)
    }

    pub fn get_event_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::EventCounter)
            .unwrap_or(0)
    }

    pub fn get_all_events(env: Env) -> Vec<Event> {
        let count = Self::get_event_count(env.clone());
        let mut events = Vec::new(&env);

        for event_id in 0..count {
            if let Some(event) = env.storage().persistent().get(&DataKey::Event(event_id)) {
                events.push_back(event);
            }
        }

        events
    }

    pub fn get_buyer_purchase(env: Env, event_id: u32, buyer: Address) -> Option<BuyerPurchase> {
        env.storage()
            .persistent()
            .get(&DataKey::BuyerPurchase(event_id, buyer))
    }

    pub fn cancel_event(env: Env, event_id: u32) -> Result<(), Error> {
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(Error::EventNotFound)?;

        event.organizer.require_auth();

        if event.is_canceled {
            return Err(Error::EventAlreadyCanceled);
        }

        event.is_canceled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        Self::extend_persistent_ttl(&env, &DataKey::Event(event_id));

        env.events()
            .publish((Symbol::new(&env, "event_canceled"),), event_id);

        Ok(())
    }

    pub fn claim_refund(env: Env, claimer: Address, event_id: u32) {
        claimer.require_auth();

        let event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap_or_else(|| panic!("Event not found"));

        if !event.is_canceled {
            panic!("Event is not canceled");
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::RefundClaimed(event_id, claimer.clone()))
        {
            panic!("Refund already claimed");
        }

        let purchase: BuyerPurchase = env
            .storage()
            .persistent()
            .get(&DataKey::BuyerPurchase(event_id, claimer.clone()))
            .unwrap_or_else(|| panic!("Claimer did not purchase a ticket for this event"));

        env.storage()
            .persistent()
            .set(&DataKey::RefundClaimed(event_id, claimer.clone()), &true);
        Self::extend_persistent_ttl(&env, &DataKey::RefundClaimed(event_id, claimer.clone()));

        if purchase.total_paid > 0 {
            let token_client = soroban_sdk::token::Client::new(&env, &event.payment_token);
            token_client.transfer(&event.organizer, &claimer, &purchase.total_paid);
        }

        env.events().publish(
            (Symbol::new(&env, "refund_claimed"),),
            (event_id, claimer, purchase.quantity, purchase.total_paid),
        );
    }

    pub fn update_event(
        env: Env,
        event_id: u32,
        theme: Option<String>,
        ticket_price: Option<i128>,
        total_tickets: Option<u128>,
        start_date: Option<u64>,
        end_date: Option<u64>,
    ) {
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap_or_else(|| panic!("Event not found"));

        event.organizer.require_auth();

        if event.is_canceled {
            panic!("Cannot update a canceled event");
        }

        let current_time = env.ledger().timestamp();

        if let Some(next_theme) = theme {
            event.theme = next_theme;
        }

        if let Some(next_price) = ticket_price {
            if next_price < 0 {
                panic!("Ticket price cannot be negative");
            }
            event.ticket_price = next_price;
        }

        if let Some(next_total) = total_tickets {
            if next_total == 0 {
                panic!("Total tickets must be greater than 0");
            }
            if next_total < event.tickets_sold {
                panic!("Cannot reduce total_tickets below tickets_sold");
            }
            event.total_tickets = next_total;
        }

        let effective_end = end_date.unwrap_or(event.end_date);
        if let Some(next_start) = start_date {
            if next_start < current_time {
                panic!("Start date cannot be in the past");
            }
            if next_start >= effective_end {
                panic!("Start date must be before end date");
            }
            event.start_date = next_start;
        }

        let effective_start = start_date.unwrap_or(event.start_date);
        if let Some(next_end) = end_date {
            if next_end < current_time {
                panic!("End date cannot be in the past");
            }
            if next_end <= effective_start {
                panic!("End date must be after start date");
            }
            event.end_date = next_end;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        Self::extend_persistent_ttl(&env, &DataKey::Event(event_id));

        env.events().publish(
            (Symbol::new(&env, "event_updated"),),
            (event_id, event.organizer),
        );
    }

    pub fn update_tickets_sold(env: Env, event_id: u32, amount: u128) -> Result<(), Error> {
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(Error::EventNotFound)?;

        event.ticket_nft_addr.require_auth();

        event.tickets_sold = event
            .tickets_sold
            .checked_add(amount)
            .ok_or(Error::CounterOverflow)?;

        if event.tickets_sold > event.total_tickets {
            return Err(Error::CannotSellMoreTickets);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        Self::extend_persistent_ttl(&env, &DataKey::Event(event_id));

        Ok(())
    }

    pub fn purchase_ticket(env: Env, buyer: Address, event_id: u32) {
        Self::purchase_tickets(env, buyer, event_id, 1);
    }

    pub fn purchase_tickets(env: Env, buyer: Address, event_id: u32, quantity: u128) {
        buyer.require_auth();

        if quantity == 0 {
            panic!("Quantity must be greater than 0");
        }

        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap_or_else(|| panic!("Event not found"));

        if event.is_canceled {
            panic!("Event is canceled");
        }

        let next_tickets_sold = event
            .tickets_sold
            .checked_add(quantity)
            .unwrap_or_else(|| panic!("Ticket quantity overflow"));

        if next_tickets_sold > event.total_tickets {
            panic!("Event is sold out");
        }

        let total_price = Self::calculate_total_price(event.ticket_price, quantity);
        if total_price > 0 {
            let token_client = soroban_sdk::token::Client::new(&env, &event.payment_token);
            token_client.transfer(&buyer, &event.organizer, &total_price);
        }

        for _ in 0..quantity {
            env.invoke_contract::<u128>(
                &event.ticket_nft_addr,
                &Symbol::new(&env, "mint_ticket_nft"),
                soroban_sdk::vec![&env, buyer.clone().into_val(&env)],
            );
        }

        Self::record_purchase(&env, event_id, buyer.clone(), quantity, total_price);

        event.tickets_sold = next_tickets_sold;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        Self::extend_persistent_ttl(&env, &DataKey::Event(event_id));

        env.events().publish(
            (Symbol::new(&env, "ticket_purchased"),),
            (event_id, buyer, quantity, total_price, event.ticket_nft_addr),
        );
    }

    fn validate_event_params(
        env: &Env,
        start_date: u64,
        end_date: u64,
        ticket_price: i128,
        total_tickets: u128,
    ) -> Result<(), Error> {
        let current_time = env.ledger().timestamp();

        if start_date <= current_time {
            return Err(Error::InvalidStartDate);
        }

        if end_date <= start_date {
            return Err(Error::InvalidEndDate);
        }

        if ticket_price < 0 {
            return Err(Error::NegativeTicketPrice);
        }

        if total_tickets == 0 {
            return Err(Error::InvalidTicketCount);
        }

        Ok(())
    }

    fn get_and_increment_counter(env: &Env) -> Result<u32, Error> {
        let current: u32 = env
            .storage()
            .instance()
            .get(&DataKey::EventCounter)
            .unwrap_or(0);

        let next = current.checked_add(1).ok_or(Error::CounterOverflow)?;
        env.storage().instance().set(&DataKey::EventCounter, &next);
        env.storage()
            .instance()
            .extend_ttl(Self::ttl_threshold(), Self::ttl_extend_to());

        Ok(current)
    }

    fn deploy_ticket_nft(env: &Env, event_id: u32) -> Option<Address> {
        let factory_addr: Address = env.storage().instance().get(&DataKey::TicketFactory)?;
        let mut salt_bytes = [0u8; 32];
        salt_bytes[28..32].copy_from_slice(&event_id.to_be_bytes());
        let salt = BytesN::from_array(env, &salt_bytes);

        Some(env.invoke_contract::<Address>(
            &factory_addr,
            &Symbol::new(env, "deploy_ticket"),
            soroban_sdk::vec![
                env,
                env.current_contract_address().into_val(env),
                salt.into_val(env)
            ],
        ))
    }

    fn record_purchase(
        env: &Env,
        event_id: u32,
        buyer: Address,
        quantity: u128,
        total_paid: i128,
    ) {
        let key = DataKey::BuyerPurchase(event_id, buyer.clone());
        let existing = env.storage().persistent().get::<_, BuyerPurchase>(&key);

        if let Some(mut purchase) = existing {
            purchase.quantity = purchase
                .quantity
                .checked_add(quantity)
                .unwrap_or_else(|| panic!("Purchase quantity overflow"));
            purchase.total_paid = purchase
                .total_paid
                .checked_add(total_paid)
                .unwrap_or_else(|| panic!("Purchase total overflow"));
            env.storage().persistent().set(&key, &purchase);
        } else {
            let purchase = BuyerPurchase {
                quantity,
                total_paid,
            };
            env.storage().persistent().set(&key, &purchase);

            let buyers_key = DataKey::EventBuyers(event_id);
            let mut buyers: Vec<Address> = env
                .storage()
                .persistent()
                .get(&buyers_key)
                .unwrap_or_else(|| Vec::new(env));
            buyers.push_back(buyer.clone());
            env.storage().persistent().set(&buyers_key, &buyers);
            Self::extend_persistent_ttl(env, &buyers_key);
        }

        Self::extend_persistent_ttl(env, &key);
    }

    fn calculate_total_price(ticket_price: i128, quantity: u128) -> i128 {
        if ticket_price <= 0 {
            return 0;
        }

        let quantity_i128 =
            i128::try_from(quantity).unwrap_or_else(|_| panic!("Quantity exceeds pricing range"));
        let subtotal = ticket_price
            .checked_mul(quantity_i128)
            .unwrap_or_else(|| panic!("Price overflow"));

        let discount_bps = if quantity >= 10 {
            1_000i128
        } else if quantity >= 5 {
            500i128
        } else {
            0i128
        };

        subtotal
            .checked_mul(10_000i128 - discount_bps)
            .and_then(|value| value.checked_div(10_000))
            .unwrap_or_else(|| panic!("Discount calculation overflow"))
    }

    fn extend_persistent_ttl(env: &Env, key: &DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, Self::ttl_threshold(), Self::ttl_extend_to());
    }

    const fn ttl_threshold() -> u32 {
        30 * 24 * 60 * 60 / 5
    }

    const fn ttl_extend_to() -> u32 {
        100 * 24 * 60 * 60 / 5
    }
}

#[cfg(test)]
mod test;
