spec aptos_framework::staking_proxy {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Aborts if conditions of SetStakePoolOperator are not met
    spec set_operator(owner: &signer, old_operator: address, new_operator: address) {
        // TODO: Can't verify `set_vesting_contract_operator` and `set_staking_contract_operator`
        pragma aborts_if_is_partial;
        include SetStakePoolOperator;
    }

    /// Aborts if conditions of SetStackingContractVoter and SetStackPoolVoterAbortsIf are not met
    spec set_voter(owner: &signer, operator: address, new_voter: address) {
        // TODO: Can't verify `set_vesting_contract_voter`
        pragma aborts_if_is_partial;
        include SetStakingContractVoter;
        include SetStakePoolVoterAbortsIf;
    }

    spec set_vesting_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        // TODO: Can't verify `update_voter` in while loop.
        pragma aborts_if_is_partial;

        let owner_address = signer::address_of(owner);
        let vesting_contracts = global<vesting::AdminStore>(owner_address).vesting_contracts;
        let post post_vesting_contracts = global<vesting::AdminStore>(owner_address).vesting_contracts;
        ensures exists i in 0..len(vesting_contracts): (global<vesting::VestingContract>(vesting_contracts[i])).staking.operator == old_operator
            ==>
            {
                let operator_addr = find_vesting_contract(post_vesting_contracts, old_operator, 0);
                global<vesting::VestingContract>(operator_addr).staking.operator == new_operator
            };
    }

    spec fun find_vesting_contract(vesting_contracts: vector<address>, old_operator: address, count: num): address {
        let operator = global<vesting::VestingContract>(vesting_contracts[count]).staking.operator;
        if (operator == old_operator) {
            vesting_contracts[count]
        } else {
            find_vesting_contract(vesting_contracts, old_operator, count + 1)
        }
    }

    spec set_staking_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        use aptos_std::simple_map;
        use aptos_framework::staking_contract::{Store};
        // TODO: Verify timeout and can't verify `staking_contract::switch_operator`.
        pragma aborts_if_is_partial;

        let owner_address = signer::address_of(owner);
        let store = borrow_global<Store>(owner_address);
        let staking_contract_exists = exists<Store>(owner_address) && simple_map::spec_contains_key(store.staking_contracts, old_operator);
    }

    spec set_vesting_contract_voter(owner: &signer, operator: address, new_voter: address) {
        // TODO: Can't verify `update_voter` in while loop.
        pragma aborts_if_is_partial;
    }

    /// Aborts if stake_pool is exists and when OwnerCapability or stake_pool_exists
    /// One of them are not exists
    spec set_stake_pool_operator(owner: &signer, new_operator: address) {
        include SetStakePoolOperator;
    }

    spec schema SetStakePoolOperator {
        owner: &signer;
        new_operator: address;

        let owner_address = signer::address_of(owner);
        let ownership_cap = borrow_global<stake::OwnerCapability>(owner_address);
        let pool_address = ownership_cap.pool_address;
        aborts_if stake::stake_pool_exists(owner_address) && !(exists<stake::OwnerCapability>(owner_address) && stake::stake_pool_exists(pool_address));
    }

    spec set_staking_contract_voter(owner: &signer, operator: address, new_voter: address) {
        include SetStakingContractVoter;
    }

    /// Make sure staking_contract_exists first
    /// Then abort if the resource is not exist
    spec schema SetStakingContractVoter {
        use aptos_std::simple_map;
        use aptos_framework::staking_contract::{Store};

        owner: &signer;
        operator: address;
        new_voter: address;

        let owner_address = signer::address_of(owner);
        let staker = owner_address;
        let store = global<Store>(staker);
        let staking_contract_exists = exists<Store>(staker) && simple_map::spec_contains_key(store.staking_contracts, operator);
        let staker_address = owner_address;
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.pool_address;

        aborts_if staking_contract_exists && !exists<stake::StakePool>(pool_address);
        aborts_if staking_contract_exists && !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);

    }

    spec set_stake_pool_voter(owner: &signer, new_voter: address) {
        include SetStakePoolVoterAbortsIf;
    }

    spec schema SetStakePoolVoterAbortsIf {
        owner: &signer;
        new_voter: address;

        let owner_address = signer::address_of(owner);
        let ownership_cap = global<stake::OwnerCapability>(owner_address);
        let pool_address = ownership_cap.pool_address;
        aborts_if stake::stake_pool_exists(owner_address) && !(exists<stake::OwnerCapability>(owner_address) && stake::stake_pool_exists(pool_address));
    }
}
