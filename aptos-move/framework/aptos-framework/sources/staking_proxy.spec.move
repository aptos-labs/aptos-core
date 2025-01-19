spec aptos_framework::staking_proxy {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: When updating the Vesting operator, it should be updated throughout all depending units.
    /// Criticality: Medium
    /// Implementation: The VestingContract contains a StakingInfo object that has an operator field, and this operator
    /// is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.
    /// Enforcement: Audited that it ensures the two operator fields hold the new value after the update.
    ///
    /// No.: 2
    /// Requirement: When updating the Vesting voter, it should be updated throughout all depending units.
    /// Criticality: Medium
    /// Implementation: The VestingContract contains a StakingInfo object that has an operator field, and this operator
    /// is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.
    /// Enforcement: Audited that it ensures the two operator fields hold the new value after the update.
    ///
    /// No.: 3
    /// Requirement: The operator and voter of a Vesting Contract should only be updated by the owner of the contract.
    /// Criticality: High
    /// Implementation: The owner-operator-voter model, as defined in the documentation, grants distinct abilities to
    /// each role. Therefore, it's crucial to ensure that only the owner has the authority to modify the operator or
    /// voter, to prevent the compromise of the StakePool.
    /// Enforcement: Audited that it ensures the signer owns the AdminStore resource and that the operator or voter
    /// intended for the update actually exists.
    ///
    /// No.: 4
    /// Requirement: The operator and voter of a Staking Contract should only be updated by the owner of the contract.
    /// Criticality: High
    /// Implementation: The owner-operator-voter model, as defined in the documentation, grants distinct abilities to
    /// each role. Therefore, it's crucial to ensure that only the owner has the authority to modify the operator or
    /// voter, to prevent the compromise of the StakePool.
    /// Enforcement: Audited the patterns of updating operators and voters in the staking contract.
    ///
    /// No.: 5
    /// Requirement: Staking Contract's operators should be unique inside a store.
    /// Criticality: Medium
    /// Implementation: Duplicates among operators could result in incorrectly updating the operator or voter
    /// associated with the incorrect StakingContract.
    /// Enforcement: Enforced via [https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/staking_contract.move#L87](SimpleMap).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    spec grant_permission {
        pragma aborts_if_is_partial;
        aborts_if !permissioned_signer::spec_is_permissioned_signer(permissioned_signer);
        aborts_if permissioned_signer::spec_is_permissioned_signer(master);
        aborts_if signer::address_of(master) != signer::address_of(permissioned_signer);
    }

    /// Aborts if conditions of SetStakePoolOperator are not met
    spec set_operator(owner: &signer, old_operator: address, new_operator: address) {
        pragma verify = false;
        pragma aborts_if_is_partial;
        // TODO: Can't verify due to timeout (>1000)
        // TODO: Can't verify `set_vesting_contract_operator` and `set_staking_contract_operator`
        include SetStakePoolOperator;
        include SetStakingContractOperator;
    }

    /// Aborts if conditions of SetStackingContractVoter and SetStackPoolVoterAbortsIf are not met
    spec set_voter(owner: &signer, operator: address, new_voter: address) {
        // TODO: Can't verify `set_vesting_contract_voter`
        pragma aborts_if_is_partial;
        pragma verify_duration_estimate = 120;
        include SetStakingContractVoter;
        include SetStakePoolVoterAbortsIf;
    }

    spec set_vesting_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        // TODO: Can't verify `update_voter` in while loop.
        pragma verify = false;
    }

    spec set_staking_contract_operator(owner: &signer, old_operator: address, new_operator: address) {
        pragma aborts_if_is_partial;
        pragma verify = false;
        // TODO: Verify timeout and can't verify `staking_contract::switch_operator`.
        include SetStakingContractOperator;
    }

    spec schema SetStakingContractOperator {
        use aptos_std::simple_map;
        use aptos_framework::staking_contract::{Store};
        use aptos_framework::coin;

        owner: signer;
        old_operator: address;
        new_operator: address;

        let owner_address = signer::address_of(owner);
        let store = global<Store>(owner_address);
        let staking_contract_exists = exists<Store>(owner_address) && simple_map::spec_contains_key(store.staking_contracts, old_operator);
        aborts_if staking_contract_exists && simple_map::spec_contains_key(store.staking_contracts, new_operator);

        let post post_store = global<Store>(owner_address);
        ensures staking_contract_exists ==> !simple_map::spec_contains_key(post_store.staking_contracts, old_operator);

        let staking_contract = simple_map::spec_get(store.staking_contracts, old_operator);
        let stake_pool = global<stake::StakePool>(staking_contract.pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        aborts_if staking_contract_exists && !exists<stake::StakePool>(staking_contract.pool_address);
        // the following property caused timeout
        ensures staking_contract_exists ==>
            simple_map::spec_get(post_store.staking_contracts, new_operator).principal == total_active_stake - commission_amount;

        let pool_address = staking_contract.owner_cap.pool_address;
        let current_commission_percentage = staking_contract.commission_percentage;
        aborts_if staking_contract_exists && commission_amount != 0 && !exists<stake::StakePool>(pool_address);
        ensures staking_contract_exists && commission_amount != 0 ==>
            global<stake::StakePool>(pool_address).operator_address == new_operator
            && simple_map::spec_get(post_store.staking_contracts, new_operator).commission_percentage == current_commission_percentage;

        ensures staking_contract_exists ==> simple_map::spec_contains_key(post_store.staking_contracts, new_operator);
    }

    spec set_vesting_contract_voter(owner: &signer, operator: address, new_voter: address) {
        // TODO: Can't verify `update_voter` in while loop.
        pragma verify = false;
    }

    /// Aborts if stake_pool is exists and when OwnerCapability or stake_pool_exists
    /// One of them are not exists
    spec set_stake_pool_operator(owner: &signer, new_operator: address) {
        include SetStakePoolOperator;
        include AbortsIfSignerPermissionStakeProxy {
            s: owner
        };
        include exists<stake::StakePool>(signer::address_of(owner)) ==> stake::AbortsIfSignerPermissionStake {
            s:owner
        };
    }

    spec schema SetStakePoolOperator {
        owner: &signer;
        new_operator: address;

        include AbortsIfSignerPermissionStakeProxy {
            s: owner
        };
        let owner_address = signer::address_of(owner);
        let ownership_cap = borrow_global<stake::OwnerCapability>(owner_address);
        let pool_address = ownership_cap.pool_address;
        aborts_if stake::stake_pool_exists(owner_address) && !(exists<stake::OwnerCapability>(owner_address) && stake::stake_pool_exists(pool_address));
        ensures stake::stake_pool_exists(owner_address) ==> global<stake::StakePool>(pool_address).operator_address == new_operator;
    }

    spec set_staking_contract_voter(owner: &signer, operator: address, new_voter: address) {
        include SetStakingContractVoter;
        include AbortsIfSignerPermissionStakeProxy {
            s: owner
        };
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
        let pool_address1 = staking_contract.owner_cap.pool_address;

        aborts_if staking_contract_exists && !exists<stake::StakePool>(pool_address);
        aborts_if staking_contract_exists && !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);

        ensures staking_contract_exists ==> global<stake::StakePool>(pool_address1).delegated_voter == new_voter;
    }

    spec set_stake_pool_voter(owner: &signer, new_voter: address) {
        include SetStakePoolVoterAbortsIf;
        include AbortsIfSignerPermissionStakeProxy {
            s: owner
        };
        include exists<stake::StakePool>(signer::address_of(owner)) ==> stake::AbortsIfSignerPermissionStake {
            s:owner
        };
    }

    spec schema SetStakePoolVoterAbortsIf {
        owner: &signer;
        new_voter: address;

        include AbortsIfSignerPermissionStakeProxy {
            s: owner
        };
        let owner_address = signer::address_of(owner);
        let ownership_cap = global<stake::OwnerCapability>(owner_address);
        let pool_address = ownership_cap.pool_address;
        aborts_if stake::stake_pool_exists(owner_address) && !(exists<stake::OwnerCapability>(owner_address) && stake::stake_pool_exists(pool_address));
        ensures stake::stake_pool_exists(owner_address) ==> global<stake::StakePool>(pool_address).delegated_voter == new_voter;
    }

    spec schema AbortsIfSignerPermissionStakeProxy {
        use aptos_framework::permissioned_signer;
        s: signer;
        let perm = StakeProxyPermission {};
        aborts_if !permissioned_signer::spec_check_permission_exists(s, perm);
    }
}
