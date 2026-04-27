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