module velor_framework::staking_proxy {
    use std::error;
    use std::signer;
    use std::vector;

    use velor_framework::permissioned_signer;
    use velor_framework::stake;
    use velor_framework::staking_contract;
    use velor_framework::vesting;

    struct StakeProxyPermission has copy, drop, store {}

    /// Signer does not have permission to perform stake proxy logic.
    const ENO_STAKE_PERMISSION: u64 = 28;

    /// Permissions
    inline fun check_stake_proxy_permission(s: &signer) {
        assert!(
            permissioned_signer::check_permission_exists(s, StakeProxyPermission {}),
            error::permission_denied(ENO_STAKE_PERMISSION),
        );
    }

    /// Grant permission to mutate staking on behalf of the master signer.
    public fun grant_permission(master: &signer, permissioned_signer: &signer) {
        permissioned_signer::authorize_unlimited(master, permissioned_signer, StakeProxyPermission {})
    }

    public entry fun set_operator(owner: &signer, old_operator: address, new_operator: address) {
        set_vesting_contract_operator(owner, old_operator, new_operator);
        set_staking_contract_operator(owner, old_operator, new_operator);
        set_stake_pool_operator(owner, new_operator);
    }

    public entry fun set_voter(owner: &signer, operator: address, new_voter: address) {
        set_vesting_contract_voter(owner, operator, new_voter);
        set_staking_contract_voter(owner, operator, new_voter);
        set_stake_pool_voter(owner, new_voter);
    }

    public entry fun set_vesting_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        check_stake_proxy_permission(owner);
        let owner_address = signer::address_of(owner);
        let vesting_contracts = &vesting::vesting_contracts(owner_address);
        vector::for_each_ref(vesting_contracts, |vesting_contract| {
            let vesting_contract = *vesting_contract;
            if (vesting::operator(vesting_contract) == old_operator) {
                let current_commission_percentage = vesting::operator_commission_percentage(vesting_contract);
                vesting::update_operator(owner, vesting_contract, new_operator, current_commission_percentage);
            };
        });
    }

    public entry fun set_staking_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        check_stake_proxy_permission(owner);
        let owner_address = signer::address_of(owner);
        if (staking_contract::staking_contract_exists(owner_address, old_operator)) {
            let current_commission_percentage = staking_contract::commission_percentage(owner_address, old_operator);
            staking_contract::switch_operator(owner, old_operator, new_operator, current_commission_percentage);
        };
    }

    public entry fun set_stake_pool_operator(owner: &signer, new_operator: address) {
        check_stake_proxy_permission(owner);
        let owner_address = signer::address_of(owner);
        if (stake::stake_pool_exists(owner_address)) {
            stake::set_operator(owner, new_operator);
        };
    }

    public entry fun set_vesting_contract_voter(owner: &signer, operator: address, new_voter: address) {
        check_stake_proxy_permission(owner);
        let owner_address = signer::address_of(owner);
        let vesting_contracts = &vesting::vesting_contracts(owner_address);
        vector::for_each_ref(vesting_contracts, |vesting_contract| {
            let vesting_contract = *vesting_contract;
            if (vesting::operator(vesting_contract) == operator) {
                vesting::update_voter(owner, vesting_contract, new_voter);
            };
        });
    }

    public entry fun set_staking_contract_voter(owner: &signer, operator: address, new_voter: address) {
        check_stake_proxy_permission(owner);
        let owner_address = signer::address_of(owner);
        if (staking_contract::staking_contract_exists(owner_address, operator)) {
            staking_contract::update_voter(owner, operator, new_voter);
        };
    }

    public entry fun set_stake_pool_voter(owner: &signer, new_voter: address) {
        check_stake_proxy_permission(owner);
        if (stake::stake_pool_exists(signer::address_of(owner))) {
            stake::set_delegated_voter(owner, new_voter);
        };
    }

    #[test_only]
    const INITIAL_BALANCE: u64 = 100000000000000; // 1M APT coins with 8 decimals.

    #[test(
        velor_framework = @0x1,
        owner = @0x123,
        operator_1 = @0x234,
        operator_2 = @0x345,
        new_operator = @0x567,
    )]
    public entry fun test_set_operator(
        velor_framework: &signer,
        owner: &signer,
        operator_1: &signer,
        operator_2: &signer,
        new_operator: &signer,
    ) {
        let owner_address = signer::address_of(owner);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);
        let new_operator_address = signer::address_of(new_operator);
        vesting::setup(
            velor_framework, &vector[owner_address, operator_1_address, operator_2_address, new_operator_address]);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_1, INITIAL_BALANCE, 0);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_2, INITIAL_BALANCE, 0);

        let vesting_contract_1 = vesting::setup_vesting_contract(owner, &vector[@11], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_1, operator_1_address, 0);
        let vesting_contract_2 = vesting::setup_vesting_contract(owner, &vector[@12], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_2, operator_2_address, 0);

        let (_sk, pk, pop) = stake::generate_identity();
        stake::initialize_test_validator(&pk, &pop, owner, INITIAL_BALANCE, false, false);
        stake::set_operator(owner, operator_1_address);

        set_operator(owner, operator_1_address, new_operator_address);
        // Stake pool's operator has been switched from operator 1 to new operator.
        assert!(stake::get_operator(owner_address) == new_operator_address, 0);
        // Staking contract has been switched from operator 1 to new operator.
        // Staking contract with operator_2 should stay unchanged.
        assert!(staking_contract::staking_contract_exists(owner_address, new_operator_address), 1);
        assert!(!staking_contract::staking_contract_exists(owner_address, operator_1_address), 2);
        assert!(staking_contract::staking_contract_exists(owner_address, operator_2_address), 3);
        // Vesting contract 1 has been switched from operator 1 to new operator while vesting contract 2 stays unchanged
        assert!(vesting::operator(vesting_contract_1) == new_operator_address, 4);
        assert!(vesting::operator(vesting_contract_2) == operator_2_address, 5);
    }

    #[test(
        velor_framework = @0x1,
        owner = @0x123,
        operator_1 = @0x234,
        operator_2 = @0x345,
        new_operator = @0x567,
    )]
    public entry fun test_set_operator_nothing_to_change(
        velor_framework: &signer,
        owner: &signer,
        operator_1: &signer,
        operator_2: &signer,
        new_operator: &signer,
    ) {
        let owner_address = signer::address_of(owner);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);
        let new_operator_address = signer::address_of(new_operator);
        vesting::setup(
            velor_framework, &vector[owner_address, operator_1_address, operator_2_address, new_operator_address]);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_2, INITIAL_BALANCE, 0);

        let vesting_contract_2 = vesting::setup_vesting_contract(owner, &vector[@12], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_2, operator_2_address, 0);

        set_operator(owner, operator_1_address, new_operator_address);
        // No staking or vesting contracts changed.
        assert!(!staking_contract::staking_contract_exists(owner_address, new_operator_address), 0);
        assert!(staking_contract::staking_contract_exists(owner_address, operator_2_address), 1);
        assert!(vesting::operator(vesting_contract_2) == operator_2_address, 2);
    }

    #[test(
        velor_framework = @0x1,
        owner = @0x123,
        operator_1 = @0x234,
        operator_2 = @0x345,
        new_voter = @0x567,
    )]
    public entry fun test_set_voter(
        velor_framework: &signer,
        owner: &signer,
        operator_1: &signer,
        operator_2: &signer,
        new_voter: &signer,
    ) {
        let owner_address = signer::address_of(owner);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);
        let new_voter_address = signer::address_of(new_voter);
        vesting::setup(
            velor_framework, &vector[owner_address, operator_1_address, operator_2_address, new_voter_address]);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_1, INITIAL_BALANCE, 0);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_2, INITIAL_BALANCE, 0);

        let vesting_contract_1 = vesting::setup_vesting_contract(owner, &vector[@11], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_1, operator_1_address, 0);
        let vesting_contract_2 = vesting::setup_vesting_contract(owner, &vector[@12], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_2, operator_2_address, 0);

        let (_sk, pk, pop) = stake::generate_identity();
        stake::initialize_test_validator(&pk, &pop, owner, INITIAL_BALANCE, false, false);

        set_voter(owner, operator_1_address, new_voter_address);
        // Stake pool's voter has been updated.
        assert!(stake::get_delegated_voter(owner_address) == new_voter_address, 0);
        // Staking contract with operator 1's voter has been updated.
        // Staking contract with operator_2 should stay unchanged.
        let stake_pool_address_1 = staking_contract::stake_pool_address(owner_address, operator_1_address);
        let stake_pool_address_2 = staking_contract::stake_pool_address(owner_address, operator_2_address);
        assert!(stake::get_delegated_voter(stake_pool_address_1) == new_voter_address, 1);
        assert!(stake::get_delegated_voter(stake_pool_address_2) == operator_2_address, 2);
        // Vesting contract 1's voter has been updated while vesting contract 2's stays unchanged.
        assert!(vesting::voter(vesting_contract_1) == new_voter_address, 3);
        assert!(vesting::voter(vesting_contract_2) == owner_address, 4);
    }

    #[test(
        velor_framework = @0x1,
        owner = @0x123,
        operator_1 = @0x234,
        operator_2 = @0x345,
        new_voter = @0x567,
    )]
    public entry fun test_set_voter_nothing_to_change(
        velor_framework: &signer,
        owner: &signer,
        operator_1: &signer,
        operator_2: &signer,
        new_voter: &signer,
    ) {
        let owner_address = signer::address_of(owner);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);
        let new_voter_address = signer::address_of(new_voter);
        vesting::setup(
            velor_framework, &vector[owner_address, operator_1_address, operator_2_address, new_voter_address]);
        staking_contract::setup_staking_contract(velor_framework, owner, operator_2, INITIAL_BALANCE, 0);

        let vesting_contract_2 = vesting::setup_vesting_contract(owner, &vector[@12], &vector[INITIAL_BALANCE], owner_address, 0);
        vesting::update_operator(owner, vesting_contract_2, operator_2_address, 0);

        set_operator(owner, operator_1_address, new_voter_address);
        // No staking or vesting contracts changed.
        let stake_pool_address = staking_contract::stake_pool_address(owner_address, operator_2_address);
        assert!(stake::get_delegated_voter(stake_pool_address) == operator_2_address, 0);
        assert!(vesting::voter(vesting_contract_2) == owner_address, 1);
    }
}
