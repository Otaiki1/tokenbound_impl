//! DAO Governance Contract Template for Soroban
//!
//! A flexible governance contract supporting:
//! - Token-based voting power
//! - Multiple proposal types (general, contract calls, parameter changes)
//! - Configurable voting periods and quorum
//! - Proposal execution with timelocks
//! - Member management and delegation

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, IntoVal, String,
    Symbol, Vec, Map, Val,
};

use upgradeable as upg;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DaoError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    ProposalNotFound = 4,
    ProposalNotActive = 5,
    AlreadyVoted = 6,
    VotingEnded = 7,
    QuorumNotReached = 8,
    ProposalFailed = 9,
    ExecutionTimelockActive = 10,
    InsufficientVotingPower = 11,
    InvalidProposalType = 12,
    InvalidParameters = 13,
    MemberNotFound = 14,
    MemberAlreadyExists = 15,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalType {
    /// General proposal with no automatic execution
    General,
    /// Execute contract calls
    ContractCall,
    /// Change DAO parameters
    ParameterChange,
    /// Add or remove members
    MemberManagement,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub contract_calls: Vec<ContractCall>,
    pub parameter_changes: Map<Symbol, Val>,
    pub new_members: Vec<Address>,
    pub remove_members: Vec<Address>,
    pub start_time: u64,
    pub end_time: u64,
    pub execution_time: u64, // 0 means immediate execution
    pub executed: bool,
    pub cancelled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractCall {
    pub contract_address: Address,
    pub function_name: Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: u32,
    pub in_favor: bool,
    pub voting_power: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaoConfig {
    pub voting_token: Address,
    pub voting_period: u64, // seconds
    pub execution_delay: u64, // seconds
    pub quorum_percentage: u32, // percentage (0-100)
    pub proposal_threshold: u128, // minimum voting power to create proposal
    pub max_members: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    Proposal(u32),
    ProposalVotes(u32),
    Member(Address),
    AllMembers,
    ProposalCount,
    Votes(Address, u32), // (voter, proposal_id)
}

#[contract]
pub struct DaoGovernance;

#[contractimpl]
impl DaoGovernance {
    /// Initialize the DAO with configuration
    pub fn initialize(
        env: Env,
        admin: Address,
        config: DaoConfig,
    ) -> Result<(), DaoError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(DaoError::AlreadyInitialized);
        }

        admin.require_auth();
        upg::set_admin(&env, &admin);
        upg::init_version(&env);

        // Validate config
        if config.quorum_percentage > 100 {
            return Err(DaoError::InvalidParameters);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::ProposalCount, &0u32);

        // Initialize empty member list
        let empty_members: Vec<Address> = Vec::new(&env);
        env.storage().instance().set(&DataKey::AllMembers, &empty_members);

        upg::extend_instance_ttl(&env);

        env.events().publish(
            (Symbol::new(&env, "dao_initialized"),),
            (admin, config.voting_token),
        );

        Ok(())
    }

    /// Add a member to the DAO
    pub fn add_member(env: Env, member: Address) -> Result<(), DaoError> {
        upg::require_not_paused(&env);
        Self::require_admin(&env)?;

        let mut members: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Member(member.clone()))
            .unwrap_or(Vec::new(&env));

        if members.contains(&member) {
            return Err(DaoError::MemberAlreadyExists);
        }

        members.push_back(member.clone());
        env.storage().instance().set(&DataKey::AllMembers, &members);
        env.storage().instance().set(&DataKey::Member(member.clone()), &true);

        Self::extend_instance_ttl(&env);

        env.events().publish(
            (Symbol::new(&env, "member_added"),),
            (member,),
        );

        Ok(())
    }

    /// Remove a member from the DAO
    pub fn remove_member(env: Env, member: Address) -> Result<(), DaoError> {
        upg::require_not_paused(&env);
        Self::require_admin(&env)?;

        let mut members: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllMembers)
            .unwrap_or(Vec::new(&env));

        if let Some(index) = members.iter().position(|m| m == member) {
            members.remove(index);
            env.storage().instance().set(&DataKey::AllMembers, &members);
            env.storage().instance().remove(&DataKey::Member(member.clone()));

            Self::extend_instance_ttl(&env);

            env.events().publish(
                (Symbol::new(&env, "member_removed"),),
                (member,),
            );

            Ok(())
        } else {
            Err(DaoError::MemberNotFound)
        }
    }

    /// Create a new proposal
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        proposal_type: ProposalType,
        title: String,
        description: String,
        contract_calls: Vec<ContractCall>,
        parameter_changes: Map<Symbol, Val>,
        new_members: Vec<Address>,
        remove_members: Vec<Address>,
        execution_delay: u64,
    ) -> Result<u32, DaoError> {
        upg::require_not_paused(&env);
        proposer.require_auth();

        // Check if proposer is a member
        if !Self::is_member(&env, &proposer) {
            return Err(DaoError::Unauthorized);
        }

        // Check proposal threshold
        let config = Self::get_config(&env);
        let proposer_voting_power = Self::get_voting_power(&env, &proposer, &config.voting_token);
        if proposer_voting_power < config.proposal_threshold {
            return Err(DaoError::InsufficientVotingPower);
        }

        // Validate proposal type and parameters
        match proposal_type {
            ProposalType::ContractCall => {
                if contract_calls.is_empty() {
                    return Err(DaoError::InvalidParameters);
                }
            }
            ProposalType::ParameterChange => {
                if parameter_changes.is_empty() {
                    return Err(DaoError::InvalidParameters);
                }
            }
            ProposalType::MemberManagement => {
                if new_members.is_empty() && remove_members.is_empty() {
                    return Err(DaoError::InvalidParameters);
                }
            }
            ProposalType::General => {
                // No specific validation for general proposals
            }
        }

        let proposal_id = Self::get_and_increment_proposal_count(&env);
        let current_time = env.ledger().timestamp();

        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            proposal_type,
            title,
            description,
            contract_calls,
            parameter_changes,
            new_members,
            remove_members,
            start_time: current_time,
            end_time: current_time + config.voting_period,
            execution_time: current_time + config.voting_period + execution_delay,
            executed: false,
            cancelled: false,
        };

        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        Self::extend_persistent_ttl(&env, &DataKey::Proposal(proposal_id));

        env.events().publish(
            (Symbol::new(&env, "proposal_created"),),
            (proposal_id, proposer, proposal.proposal_type),
        );

        Ok(proposal_id)
    }

    /// Cast a vote on a proposal
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: u32,
        in_favor: bool,
    ) -> Result<(), DaoError> {
        upg::require_not_paused(&env);
        voter.require_auth();

        // Check if voter is a member
        if !Self::is_member(&env, &voter) {
            return Err(DaoError::Unauthorized);
        }

        // Get proposal
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(DaoError::ProposalNotFound)?;

        let current_time = env.ledger().timestamp();

        // Check voting period
        if current_time < proposal.start_time || current_time > proposal.end_time {
            return Err(DaoError::VotingEnded);
        }

        if proposal.executed || proposal.cancelled {
            return Err(DaoError::ProposalNotActive);
        }

        // Check if already voted
        if env.storage().persistent().has(&DataKey::Votes(voter.clone(), proposal_id)) {
            return Err(DaoError::AlreadyVoted);
        }

        // Get voting power
        let config = Self::get_config(&env);
        let voting_power = Self::get_voting_power(&env, &voter, &config.voting_token);

        if voting_power == 0 {
            return Err(DaoError::InsufficientVotingPower);
        }

        // Record vote
        let vote = Vote {
            voter: voter.clone(),
            proposal_id,
            in_favor,
            voting_power,
            timestamp: current_time,
        };

        env.storage().persistent().set(&DataKey::Votes(voter.clone(), proposal_id), &vote);

        // Update proposal votes
        let mut votes: Map<Address, Vote> = env
            .storage()
            .persistent()
            .get(&DataKey::ProposalVotes(proposal_id))
            .unwrap_or(Map::new(&env));

        votes.set(voter.clone(), vote);
        env.storage().persistent().set(&DataKey::ProposalVotes(proposal_id), &votes);

        Self::extend_persistent_ttl(&env, &DataKey::Proposal(proposal_id));
        Self::extend_persistent_ttl(&env, &DataKey::ProposalVotes(proposal_id));

        env.events().publish(
            (Symbol::new(&env, "vote_cast"),),
            (proposal_id, voter, in_favor, voting_power),
        );

        Ok(())
    }

    /// Execute a passed proposal
    pub fn execute_proposal(env: Env, executor: Address, proposal_id: u32) -> Result<(), DaoError> {
        upg::require_not_paused(&env);
        executor.require_auth();

        // Get proposal
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(DaoError::ProposalNotFound)?;

        if proposal.executed {
            return Err(DaoError::ProposalFailed); // Already executed
        }

        if proposal.cancelled {
            return Err(DaoError::ProposalNotActive);
        }

        let current_time = env.ledger().timestamp();

        // Check if voting period has ended
        if current_time <= proposal.end_time {
            return Err(DaoError::VotingEnded); // Still voting
        }

        // Check execution timelock
        if current_time < proposal.execution_time {
            return Err(DaoError::ExecutionTimelockActive);
        }

        // Check if proposal passed
        let config = Self::get_config(&env);
        let (votes_for, votes_against, total_votes) = Self::count_votes(&env, proposal_id);

        let total_voting_power = Self::get_total_voting_power(&env, &config.voting_token);
        let quorum_reached = (total_votes * 100) / total_voting_power >= config.quorum_percentage as u128;

        if !quorum_reached || votes_for <= votes_against {
            return Err(DaoError::QuorumNotReached);
        }

        // Execute based on proposal type
        match proposal.proposal_type {
            ProposalType::ContractCall => {
                Self::execute_contract_calls(&env, &proposal.contract_calls)?;
            }
            ProposalType::ParameterChange => {
                Self::execute_parameter_changes(&env, &proposal.parameter_changes)?;
            }
            ProposalType::MemberManagement => {
                Self::execute_member_changes(&env, &proposal.new_members, &proposal.remove_members)?;
            }
            ProposalType::General => {
                // No execution needed for general proposals
            }
        }

        // Mark as executed
        proposal.executed = true;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        Self::extend_persistent_ttl(&env, &DataKey::Proposal(proposal_id));

        env.events().publish(
            (Symbol::new(&env, "proposal_executed"),),
            (proposal_id, votes_for, votes_against),
        );

        Ok(())
    }

    /// Cancel a proposal (only by proposer or admin)
    pub fn cancel_proposal(env: Env, canceller: Address, proposal_id: u32) -> Result<(), DaoError> {
        upg::require_not_paused(&env);
        canceller.require_auth();

        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(DaoError::ProposalNotFound)?;

        // Only proposer or admin can cancel
        if proposal.proposer != canceller && !Self::is_admin(&env, &canceller) {
            return Err(DaoError::Unauthorized);
        }

        if proposal.executed {
            return Err(DaoError::ProposalFailed);
        }

        proposal.cancelled = true;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        Self::extend_persistent_ttl(&env, &DataKey::Proposal(proposal_id));

        env.events().publish(
            (Symbol::new(&env, "proposal_cancelled"),),
            (proposal_id, canceller),
        );

        Ok(())
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// Get proposal details
    pub fn get_proposal(env: Env, proposal_id: u32) -> Option<Proposal> {
        env.storage().persistent().get(&DataKey::Proposal(proposal_id))
    }

    /// Get votes for a proposal
    pub fn get_proposal_votes(env: Env, proposal_id: u32) -> Map<Address, Vote> {
        env.storage()
            .persistent()
            .get(&DataKey::ProposalVotes(proposal_id))
            .unwrap_or(Map::new(&env))
    }

    /// Get voting power of an address
    pub fn get_voting_power_view(env: Env, address: Address) -> u128 {
        let config = Self::get_config(&env);
        Self::get_voting_power(&env, &address, &config.voting_token)
    }

    /// Check if address is a member
    pub fn is_member_view(env: Env, address: Address) -> bool {
        Self::is_member(&env, &address)
    }

    /// Get all members
    pub fn get_members(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::AllMembers)
            .unwrap_or(Vec::new(&env))
    }

    /// Get DAO configuration
    pub fn get_config_view(env: Env) -> DaoConfig {
        Self::get_config(&env)
    }

    // ── Internal Helper Functions ────────────────────────────────────────────

    fn get_config(env: &Env) -> DaoConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .expect("DAO not initialized")
    }

    fn is_member(env: &Env, address: &Address) -> bool {
        env.storage().instance().has(&DataKey::Member(address.clone()))
    }

    fn is_admin(env: &Env, address: &Address) -> bool {
        upg::get_admin(env) == *address
    }

    fn require_admin(env: &Env) -> Result<(), DaoError> {
        if !Self::is_admin(env, &env.invoker()) {
            return Err(DaoError::Unauthorized);
        }
        Ok(())
    }

    fn get_voting_power(env: &Env, address: &Address, token: &Address) -> u128 {
        // Get balance from voting token contract
        let token_client = soroban_sdk::token::Client::new(env, token);
        token_client.balance(address)
    }

    fn get_total_voting_power(env: &Env, token: &Address) -> u128 {
        // For simplicity, we'll use the token's total supply
        // In a real implementation, you might want to track only member voting power
        let token_client = soroban_sdk::token::Client::new(env, token);
        token_client.total_supply()
    }

    fn get_and_increment_proposal_count(env: &Env) -> u32 {
        let mut count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        count += 1;
        env.storage().instance().set(&DataKey::ProposalCount, &count);
        count
    }

    fn count_votes(env: &Env, proposal_id: u32) -> (u128, u128, u128) {
        let votes: Map<Address, Vote> = env
            .storage()
            .persistent()
            .get(&DataKey::ProposalVotes(proposal_id))
            .unwrap_or(Map::new(env));

        let mut votes_for = 0u128;
        let mut votes_against = 0u128;
        let mut total_votes = 0u128;

        for vote in votes.values() {
            total_votes += vote.voting_power;
            if vote.in_favor {
                votes_for += vote.voting_power;
            } else {
                votes_against += vote.voting_power;
            }
        }

        (votes_for, votes_against, total_votes)
    }

    fn execute_contract_calls(env: &Env, calls: &Vec<ContractCall>) -> Result<(), DaoError> {
        for call in calls.iter() {
            // Note: In a real implementation, you'd want to add safety checks
            // and potentially use a timelock for sensitive operations
            env.invoke_contract::<Val>(
                &call.contract_address,
                &call.function_name,
                call.args.clone(),
            );
        }
        Ok(())
    }

    fn execute_parameter_changes(env: &Env, changes: &Map<Symbol, Val>) -> Result<(), DaoError> {
        // This would update DAO configuration
        // Implementation depends on what parameters are configurable
        for (key, value) in changes.iter() {
            match key {
                // Add parameter update logic here
                _ => return Err(DaoError::InvalidParameters),
            }
        }
        Ok(())
    }

    fn execute_member_changes(
        env: &Env,
        new_members: &Vec<Address>,
        remove_members: &Vec<Address>,
    ) -> Result<(), DaoError> {
        for member in new_members.iter() {
            Self::add_member_internal(env, &member)?;
        }
        for member in remove_members.iter() {
            Self::remove_member_internal(env, &member)?;
        }
        Ok(())
    }

    fn add_member_internal(env: &Env, member: &Address) -> Result<(), DaoError> {
        let mut members: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllMembers)
            .unwrap_or(Vec::new(env));

        if !members.contains(member) {
            members.push_back(member.clone());
            env.storage().instance().set(&DataKey::AllMembers, &members);
            env.storage().instance().set(&DataKey::Member(member.clone()), &true);
        }
        Ok(())
    }

    fn remove_member_internal(env: &Env, member: &Address) -> Result<(), DaoError> {
        let mut members: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllMembers)
            .unwrap_or(Vec::new(env));

        if let Some(index) = members.iter().position(|m| *m == *member) {
            members.remove(index);
            env.storage().instance().set(&DataKey::AllMembers, &members);
            env.storage().instance().remove(&DataKey::Member(member.clone()));
        }
        Ok(())
    }

    fn extend_persistent_ttl(env: &Env, key: &DataKey) {
        upg::extend_persistent_ttl(env, key);
    }

    fn extend_instance_ttl(env: &Env) {
        upg::extend_instance_ttl(env);
    }
}