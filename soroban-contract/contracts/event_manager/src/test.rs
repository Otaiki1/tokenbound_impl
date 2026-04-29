#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, IntoVal, Symbol};

// Mock contract to serve as an execution target
#[contract]
pub struct MockTargetContract;

#[contractimpl]
impl MockTargetContract {
    pub fn ping(env: Env, val: u32) -> u32 {
        val * 2
    }
}

#[test]
fn test_multisig_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let multisig_id = env.register_contract(None, MultiSigContract);
    let multisig_client = MultiSigContractClient::new(&env, &multisig_id);

    let target_id = env.register_contract(None, MockTargetContract);

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    // Initialize with 3 signers and a threshold of 2
    let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];
    multisig_client.init(&signers, &2);

    let args: Vec<Val> = vec![&env, 21u32.into_val(&env)];
    let function = Symbol::new(&env, "ping");

    // 1. Propose
    let proposal_id = multisig_client.propose(&signer1, &target_id, &function, &args);
    assert_eq!(proposal_id, 1);

    // 2. Approve (Signer 1 and Signer 2)
    multisig_client.approve(&signer1, &proposal_id);
    
    // Attempting to execute prematurely should fail
    let fail_exec = multisig_client.try_execute(&signer1, &proposal_id);
    assert_eq!(fail_exec.unwrap_err().unwrap(), Error::NotEnoughApprovals);

    multisig_client.approve(&signer2, &proposal_id);

    // 3. Execute
    let result: Val = multisig_client.execute(&signer1, &proposal_id);
    
    // 21 * 2 = 42
    assert_eq!(u32::from_val(&env, &result), 42);
}

#[test]
fn test_unauthorized_propose() {
    let env = Env::default();
    env.mock_all_auths();

    let multisig_id = env.register_contract(None, MultiSigContract);
    let multisig_client = MultiSigContractClient::new(&env, &multisig_id);

    let signer1 = Address::generate(&env);
    let non_signer = Address::generate(&env);

    let signers = vec![&env, signer1.clone()];
    multisig_client.init(&signers, &1);

    let target = Address::generate(&env);
    let res = multisig_client.try_propose(
        &non_signer,
        &target,
        &Symbol::new(&env, "ping"),
        &vec![&env]
    );

    assert_eq!(res.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_double_approve_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let multisig_id = env.register_contract(None, MultiSigContract);
    let multisig_client = MultiSigContractClient::new(&env, &multisig_id);

    let signer = Address::generate(&env);
    multisig_client.init(&vec![&env, signer.clone()], &1);

    let id = multisig_client.propose(&signer, &Address::generate(&env), &Symbol::new(&env, "x"), &vec![&env]);
    multisig_client.approve(&signer, &id);
    
    let res = multisig_client.try_approve(&signer, &id);
    assert_eq!(res.unwrap_err().unwrap(), Error::AlreadyApproved);
}