#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::*;

#[contract]
struct DummyContract;

fn setup_test() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    (env, admin)
}

#[test]
fn test_initialize_sets_admin() {
    let (env, admin) = setup_test();
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        assert!(is_admin(&env, &admin));
        assert!(has_role(&env, &Role::Admin, &admin));
    });
}

#[test]
fn test_initialize_grants_all_roles_to_admin() {
    let (env, admin) = setup_test();
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        
        assert!(has_role(&env, &Role::Upgrader, &admin));
        assert!(has_role(&env, &Role::Pauser, &admin));
        assert!(has_role(&env, &Role::Manager, &admin));
        assert!(has_role(&env, &Role::Minter, &admin));
        assert!(has_role(&env, &Role::Organizer, &admin));
        assert!(has_role(&env, &Role::PaymentReleaser, &admin));
    });
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_fails_if_already_initialized() {
    let (env, admin) = setup_test();
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        // Second initialization should panic
        let admin2 = Address::generate(&env);
        initialize(&env, &admin2);
    });
}

#[test]
fn test_grant_role() {
    let (env, admin) = setup_test();
    let new_manager = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        grant_role(&env, &Role::Manager, &new_manager, &admin);
        assert!(has_role(&env, &Role::Manager, &new_manager));
    });
}

#[test]
fn test_revoke_role() {
    let (env, admin) = setup_test();
    let manager = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        // Grant the role
        grant_role(&env, &Role::Manager, &manager, &admin);
        assert!(has_role(&env, &Role::Manager, &manager));
    });

    // Re-enter contract context for revoke (fresh auth context)
    env.as_contract(&dummy_id, || {
        // Revoke the role
        revoke_role(&env, &Role::Manager, &manager, &admin);
        assert!(!has_role(&env, &Role::Manager, &manager));
    });
}

#[test]
fn test_renounce_role() {
    let (env, admin) = setup_test();
    let manager = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        grant_role(&env, &Role::Manager, &manager, &admin);
        assert!(has_role(&env, &Role::Manager, &manager));
        
        // Manager renounces their own role
        renounce_role(&env, &Role::Manager, &manager);
        assert!(!has_role(&env, &Role::Manager, &manager));
    });
}

#[test]
#[should_panic]
fn test_non_admin_cannot_grant_role() {
    let (env, admin) = setup_test();
    let non_admin = Address::generate(&env);
    let target = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        // This should panic because non_admin doesn't have admin role
        grant_role(&env, &Role::Manager, &target, &non_admin);
    });
}

#[test]
fn test_transfer_admin() {
    let (env, admin) = setup_test();
    let new_admin = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        transfer_admin(&env, &new_admin, &admin);
        
        assert!(is_admin(&env, &new_admin));
        assert!(!is_admin(&env, &admin));
    });
}

#[test]
fn test_has_any_role() {
    let (env, admin) = setup_test();
    let manager = Address::generate(&env);
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        grant_role(&env, &Role::Manager, &manager, &admin);
        
        let roles = [Role::Manager, Role::Minter];
        assert!(has_any_role(&env, &roles, &manager));
        
        let other_roles = [Role::Minter, Role::Upgrader];
        assert!(!has_any_role(&env, &other_roles, &manager));
    });
}

#[test]
fn test_get_account_roles() {
    let (env, admin) = setup_test();
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        initialize(&env, &admin);
        
        let roles = get_account_roles(&env, &admin);
        
        // Admin should have all roles
        assert_eq!(roles.len(), 7);
    });
}

#[test]
fn test_get_admin_returns_none_before_init() {
    let env = Env::default();
    let dummy_id = env.register(DummyContract, ());

    env.as_contract(&dummy_id, || {
        assert!(get_admin(&env).is_none());
    });
}
