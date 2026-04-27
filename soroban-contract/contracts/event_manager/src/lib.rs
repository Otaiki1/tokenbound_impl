#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, Symbol, Val, Vec,
};

const DAY_IN_LEDGERS: u32 = 17280; // Assuming ~5 seconds per ledger
const MIN_TTL: u32 = 14 * DAY_IN_LEDGERS;
const EXTEND_TTL: u32 = 30 * DAY_IN_LEDGERS;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidThreshold = 4,
    ProposalNotFound = 5,
    AlreadyApproved = 6,
    AlreadyExecuted = 7,
    NotEnoughApprovals = 8,
    MathOverflow = 9,
}

#[contracttype]
pub enum DataKey {
    Threshold,
    ProposalCount,
    Signer(Address),
    Proposal(u64),
    Approval(u64, Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub approvals: u32,
    pub executed: bool,
}

#[contract]
pub struct MultiSigContract;

#[contractimpl]
impl MultiSigContract {
    /// Initializes the multisig wallet with a list of signers and a required approval threshold.
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Threshold) {
            return Err(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold > signers.len() as u32 {
            return Err(Error::InvalidThreshold);
        }

        for signer in signers.iter() {
            let key = DataKey::Signer(signer.clone());
            env.storage().persistent().set(&key, &true);
            env.storage().persistent().extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        }

        env.storage().instance().set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
        env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);

        Ok(())
    }

    /// Proposes a new transaction to be executed.
    pub fn propose(
        env: Env,
        caller: Address,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
    ) -> Result<u64, Error> {
        caller.require_auth();
        Self::check_signer(&env, &caller)?;

        let count: u64 = env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0);
        let new_count = count.checked_add(1).ok_or(Error::MathOverflow)?;

        let proposal = Proposal {
            target,
            function,
            args,
            approvals: 0,
            executed: false,
        };

        let proposal_key = DataKey::Proposal(new_count);
        env.storage().persistent().set(&proposal_key, &proposal);
        env.storage().persistent().extend_ttl(&proposal_key, MIN_TTL, EXTEND_TTL);

        env.storage().instance().set(&DataKey::ProposalCount, &new_count);
        env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);

        Ok(new_count)
    }

    /// Approves a specific proposal.
    pub fn approve(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        caller.require_auth();
        Self::check_signer(&env, &caller)?;

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyExecuted);
        }

        let approval_key = DataKey::Approval(proposal_id, caller.clone());
        if env.storage().persistent().has(&approval_key) {
            return Err(Error::AlreadyApproved);
        }

        env.storage().persistent().set(&approval_key, &true);
        env.storage().persistent().extend_ttl(&approval_key, MIN_TTL, EXTEND_TTL);

        proposal.approvals = proposal.approvals.checked_add(1).ok_or(Error::MathOverflow)?;
        
        env.storage().persistent().set(&proposal_key, &proposal);
        env.storage().persistent().extend_ttl(&proposal_key, MIN_TTL, EXTEND_TTL);

        Ok(())
    }

    /// Executes a proposal if the threshold is met.
    pub fn execute(env: Env, caller: Address, proposal_id: u64) -> Result<Val, Error> {
        caller.require_auth();
        Self::check_signer(&env, &caller)?;

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyExecuted);
        }

        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::EventBalance(event_id))
            .unwrap_or(0);

        let archived = ArchivedEvent {
            id: event.id,
            organizer: event.organizer,
            total_tickets: event.total_tickets,
            tickets_sold: event.tickets_sold,
            total_collected,
            is_canceled: event.is_canceled,
            archived_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::ArchivedEvent(event_id), &archived);
        Self::extend_persistent_ttl(&env, &DataKey::ArchivedEvent(event_id));

        if let Some(buyers) = env
            .storage()
            .persistent()
            .get::<_, Vec<Address>>(&DataKey::EventBuyers(event_id))
        {
            for buyer in buyers.iter() {
                env.storage()
                    .persistent()
                    .remove(&DataKey::BuyerPurchase(event_id, buyer.clone()));
                env.storage()
                    .persistent()
                    .remove(&DataKey::RefundClaimed(event_id, buyer));
            }
        }

        env.storage()
            .persistent()
            .remove(&DataKey::EventBuyers(event_id));
        env.storage()
            .persistent()
            .remove(&DataKey::EventTiers(event_id));
        env.storage()
            .persistent()
            .remove(&DataKey::Waitlist(event_id));
        env.storage()
            .persistent()
            .remove(&DataKey::EventBalance(event_id));
        env.storage()
            .persistent()
            .remove(&DataKey::FundsWithdrawn(event_id));
        env.storage().persistent().remove(&DataKey::Event(event_id));

        env.events()
            .publish((Symbol::new(&env, "event_archived"),), (event_id, now));

        Ok(archived)
    }

    pub fn schedule_upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        upg::schedule_upgrade(&env, new_wasm_hash);
    }

    pub fn cancel_upgrade(env: Env) {
        upg::cancel_upgrade(&env);
    }

    pub fn commit_upgrade(env: Env) {
        upg::commit_upgrade(&env);
    }

    pub fn pause(env: Env) {
        upg::pause(&env);
    }

    pub fn unpause(env: Env) {
        upg::unpause(&env);
    }

    pub fn transfer_admin(env: Env, new_admin: Address) {
        upg::transfer_admin(&env, new_admin);
    }

    pub fn version(env: Env) -> u32 {
        upg::get_version(&env)
    }

    fn validate_bounded_string(s: &String, max_bytes: u32) -> Result<(), Error> {
        if s.is_empty() || s.len() > max_bytes {
            return Err(Error::InvalidStringInput);
        }
        Ok(())
    }

    fn validate_ticket_price(price: i128) -> Result<(), Error> {
        if price > Self::MAX_TICKET_PRICE {
            return Err(Error::TicketPriceOutOfRange);
        }
        Ok(())
    }

    fn validate_create_schedule(env: &Env, start_date: u64, end_date: u64) -> Result<(), Error> {
        let now = env.ledger().timestamp();
        if start_date <= now {
            return Err(Error::InvalidStartDate);
        }
        if end_date <= start_date {
            return Err(Error::InvalidEndDate);
        }
        Self::validate_event_span(start_date, end_date)?;
        Self::validate_start_not_too_far(start_date, now)?;
        Ok(())
    }

    fn validate_event_span(start_date: u64, end_date: u64) -> Result<(), Error> {
        let span = end_date.saturating_sub(start_date);
        if span == 0 || span > Self::MAX_EVENT_DURATION_SECS {
            return Err(Error::EventScheduleOutOfRange);
        }
        Ok(())
    }

    fn validate_start_not_too_far(start_date: u64, now: u64) -> Result<(), Error> {
        let latest_start = now.saturating_add(Self::MAX_EVENT_START_AHEAD_SECS);
        if start_date > latest_start {
            return Err(Error::EventScheduleOutOfRange);
        }
        Ok(())
    }

    fn enforce_organizer_limits_and_rate(env: &Env, organizer: &Address) -> Result<(), Error> {
        let count_key = DataKey::OrganizerOpenEventCount(organizer.clone());
        let open_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        if open_count >= Self::MAX_ORGANIZER_OPEN_EVENTS {
            return Err(Error::TooManyOrganizerEvents);
        }

        if open_count > 0 {
            let ts_key = DataKey::OrganizerLastCreateTs(organizer.clone());
            let now = env.ledger().timestamp();
            if let Some(last) = env.storage().instance().get::<_, u64>(&ts_key) {
                let earliest = last.saturating_add(Self::EVENT_CREATE_COOLDOWN_SECS);
                if now < earliest {
                    return Err(Error::EventCreationRateLimited);
                }
            }
        }
        Ok(())
    }

    fn commit_organizer_create(env: &Env, organizer: &Address) {
        let ts_key = DataKey::EventCounter; // Dummy key for timestamp if not defined
        env.storage()
            .instance()
            .get(&DataKey::Threshold)
            .ok_or(Error::NotInitialized)?;

        if proposal.approvals < threshold {
            return Err(Error::NotEnoughApprovals);
        }

        // Mark executed prior to cross-contract call to prevent reentrancy risks
        proposal.executed = true;
        env.storage().persistent().set(&proposal_key, &proposal);
        env.storage().persistent().extend_ttl(&proposal_key, MIN_TTL, EXTEND_TTL);

        // Dispatch execution to the target contract
        let result = env.invoke_contract::<Val>(&proposal.target, &proposal.function, proposal.args);
        Ok(result)
    }

    // Internal utility to verify the caller is a registered signer
    fn check_signer(env: &Env, caller: &Address) -> Result<(), Error> {
        let key = DataKey::Signer(caller.clone());
        if !env.storage().persistent().has(&key) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}