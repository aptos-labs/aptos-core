spec supra_framework::vesting_without_staking {
    spec module {
        pragma verify = false;
    }

    spec VestingRecord {
        // The initial amount should be greater than or equal to the left amount
        invariant init_amount >= left_amount;
    }

    spec vesting_start_secs {
        pragma verify = true;
        include VestingContractExists{contract_address: vesting_contract_address};
    }

    spec period_duration_secs {
        pragma verify = true;
        include VestingContractExists{contract_address: vesting_contract_address};
    }

    spec remaining_grant {
        pragma verify = true;
        include VestingContractExists{contract_address: vesting_contract_address};
        aborts_if !simple_map::spec_contains_key(global<VestingContract>(vesting_contract_address).shareholders, shareholder_address);
        // ensure that the remaining grant is equal to the left amount of the shareholder
        ensures result == simple_map::spec_get(global<VestingContract>(vesting_contract_address).shareholders, shareholder_address).left_amount;
    }

    spec beneficiary {
        pragma verify = true;
        include VestingContractExists{contract_address: vesting_contract_address};
    }

    spec vesting_contracts {
        pragma verify = true;
        aborts_if false;
        // ensure that an empty vector is returned if the admin doesn't exist
        ensures !exists<AdminStore>(admin) ==> result == vector::empty<address>();
        // ensure that the vesting contracts are returned if the admin exists
        ensures exists<AdminStore>(admin) ==> result == global<AdminStore>(admin).vesting_contracts;
    }

    spec vesting_schedule {
        pragma verify = true;
        include VestingContractExists{contract_address: vesting_contract_address};
        // ensure that the vesting schedule is returned if the vesting contract exists
        ensures result == global<VestingContract>(vesting_contract_address).vesting_schedule;
    }

    // spec shareholders {
    //     pragma verify = true;
    //     include VestingContractActive{contract_address: vesting_contract_address};
    // }

    spec create_vesting_schedule {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        aborts_if vector::length(schedule) == 0;
        aborts_if period_duration <= 0;
    }

    // spec create_vesting_contract {
    //     pragma verify = true;
    //     pragma aborts_if_is_partial = true;
    //     aborts_if system_addresses::is_reserved_address(withdrawal_address);
    // }

    spec vest {
        pragma verify = false;
        pragma aborts_if_is_partial = true;
        include VestingContractActive;
        let vesting_contract_pre = global<VestingContract>(contract_address);
        let post vesting_contract_post = global<VestingContract>(contract_address);
        let vesting_schedule = vesting_contract_pre.vesting_schedule;
        let last_vested_period = vesting_schedule.last_vested_period;
        let next_period_to_vest = last_vested_period + 1;
        let last_completed_period =
            (timestamp::spec_now_seconds() - vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;
        // ensure the vesting contract is the same if the vesting period is not reached
        ensures vesting_contract_pre.vesting_schedule.start_timestamp_secs > timestamp::spec_now_seconds() ==> vesting_contract_pre == vesting_contract_post;
        // ensure the vesting contract is the same if the last completed period is greater than the next period to vest
        ensures last_completed_period < next_period_to_vest ==> vesting_contract_pre == vesting_contract_post;
    }

    spec vest_individual {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        include VestingContractActive;
        let vesting_contract_pre = global<VestingContract>(contract_address);
        let post vesting_contract_post = global<VestingContract>(contract_address);
        // let vesting_record = simple_map::spec_get(vesting_contract_pre.shareholders, shareholder_address);
        // let vesting_schedule = vesting_contract_pre.vesting_schedule;
        // let last_vested_period = vesting_record.last_vested_period;
        // let next_period_to_vest = last_vested_period + 1;
        // let last_completed_period =
        //     (timestamp::spec_now_seconds() - vesting_schedule.start_timestamp_secs) / vesting_schedule.period_duration;
        // let post post_balance = coin::balance<SupraCoin>(contract_address);
        // ensure the vesting contract is the same if the vesting period is not reached
        ensures vesting_contract_pre.vesting_schedule.start_timestamp_secs > timestamp::spec_now_seconds() ==> vesting_contract_pre == vesting_contract_post;
        // ensure the vesting contract is the same if the last completed period is greater than the next period to vest
        // ensures (last_completed_period < next_period_to_vest && post_balance > 0) ==> vesting_contract_pre == vesting_contract_post;
    }

    spec vest_transfer {
        // TODO(fa_migration)
        pragma verify = true;
        let amount = min(vesting_record.left_amount, fixed_point32::spec_multiply_u64(vesting_record.init_amount, vesting_fraction));
        // Ensure that the amount is substracted from the left_amount
        ensures vesting_record.left_amount == old(vesting_record.left_amount) - amount;
        let address_from = signer_cap.account;
        let coin_store_from = global<coin::CoinStore<SupraCoin>>(address_from);
        let post coin_store_post_from = global<coin::CoinStore<SupraCoin>>(address_from);
        let coin_store_to = global<coin::CoinStore<SupraCoin>>(beneficiary);
        let post coin_store_post_to = global<coin::CoinStore<SupraCoin>>(beneficiary);
        // Ensure that the amount is transferred from the address_from to the beneficiary
        ensures beneficiary != address_from ==> coin_store_post_from.coin.value ==
            coin_store_from.coin.value - amount;
        ensures beneficiary != address_from ==> coin_store_post_to.coin.value == coin_store_to.coin.value + amount;
        ensures beneficiary == address_from ==> coin_store_post_from.coin.value == coin_store_from.coin.value;
    }

    spec remove_shareholder {
        // TODO(fa_migration)
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        include AdminAborts;
        let vesting_contract = global<VestingContract>(contract_address);
        let post vesting_contract_post = global<VestingContract>(contract_address);

        let balance_pre = global<coin::CoinStore<SupraCoin>>(vesting_contract.withdrawal_address).coin.value;
        let post balance_post = global<coin::CoinStore<SupraCoin>>(vesting_contract.withdrawal_address).coin.value;
        let shareholder_amount = simple_map::spec_get(vesting_contract.shareholders, shareholder_address).left_amount;
        // ensure that `withdrawal address` receives the `shareholder_amount`
        ensures vesting_contract_post.withdrawal_address != vesting_contract.signer_cap.account ==> balance_post == balance_pre + shareholder_amount;
        // ensure that `shareholder_address` is indeed removed from the contract
        ensures !simple_map::spec_contains_key(vesting_contract_post.shareholders, shareholder_address);
        // ensure that beneficiary doesn't exist for the corresponding shareholder
        ensures !simple_map::spec_contains_key(vesting_contract_post.beneficiaries, shareholder_address);
    }

    // spec terminate_vesting_contract {
    //     pragma verify = true;
    //     // include AdminAborts;
    //     // include VestingContractActive;
    // }

    spec admin_withdraw {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        let vesting_contract = global<VestingContract>(contract_address);
        let balance_pre = global<coin::CoinStore<SupraCoin>>(vesting_contract.withdrawal_address).coin.value;
        let post balance_post = global<coin::CoinStore<SupraCoin>>(vesting_contract.withdrawal_address).coin.value;
        let post balance_contract = global<coin::CoinStore<SupraCoin>>(contract_address).coin.value;

        aborts_if !(global<VestingContract>(contract_address).state == VESTING_POOL_TERMINATED);
        // // ensure that the `withdrawal_address` receives the remaining balance
        // ensures (vesting_contract.signer_cap.account != vesting_contract.withdrawal_address) ==> balance_post == balance_pre + coin::balance<SupraCoin>(contract_address);
        // // ensure that the contract balance is 0
        // ensures (vesting_contract.signer_cap.account != vesting_contract.withdrawal_address) ==> balance_contract == 0;
    }

    spec set_beneficiary {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        let vesting_contract_pre = global<VestingContract>(contract_address);
        let post vesting_contract_post = global<VestingContract>(contract_address);
        include AdminAborts{vesting_contract: vesting_contract_pre};
        // ensure that the beneficiary is set to the new_beneficiary
        ensures simple_map::spec_get(vesting_contract_post.beneficiaries, shareholder) == new_beneficiary;
    }

    spec reset_beneficiary {
        pragma verify = true;
        let post vesting_contract = global<VestingContract>(contract_address);
        // ensure that the beneficiary is removed for the shareholder
        ensures !simple_map::spec_contains_key(vesting_contract.beneficiaries, shareholder);
    }

    spec set_management_role {
        pragma verify = true;
    }

    spec set_beneficiary_resetter {
        pragma verify = true;
    }

    spec get_role_holder {
        pragma verify = true;
    }

    spec get_vesting_account_signer {
        pragma verify = true;
        let vesting_contract = global<VestingContract>(contract_address);
        include AdminAborts;
        aborts_if !exists<VestingContract>(contract_address);
    }

    spec get_vesting_account_signer_internal {
        pragma verify = true;
        aborts_if false;
        let address = vesting_contract.signer_cap.account;
        // ensure that the address is returned if the vesting contract exists
        ensures signer::address_of(result) == address;
    }

    spec create_vesting_contract_account {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        aborts_if !exists<AdminStore>(signer::address_of(admin));
    }

    spec verify_admin {
        pragma verify = true;
        include AdminAborts;
    }
    spec schema AdminAborts {
        admin: &signer;
        vesting_contract: &VestingContract;
        aborts_if signer::address_of(admin) != vesting_contract.admin;
    }

    spec assert_vesting_contract_exists {
        pragma verify = true;
        include VestingContractExists;
    }

    spec schema VestingContractExists {
        contract_address: address;
        aborts_if !exists<VestingContract>(contract_address);
    }

    spec assert_active_vesting_contract {
        pragma verify = true;
        include VestingContractActive;
    }

    spec schema VestingContractActive {
        include VestingContractExists;
        contract_address: address;
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if !(vesting_contract.state == VESTING_POOL_ACTIVE);
    }

    spec get_beneficiary {
        pragma verify = true;
        pragma opaque;
        aborts_if false;
        // ensure that the beneficiary is returned if it exists, note this is used in distribute_to_shareholder function
        ensures simple_map::spec_contains_key(contract.beneficiaries, shareholder) ==> result == simple_map::spec_get(contract.beneficiaries, shareholder);
        // ensure that the shareholder is returned if the beneficiary doesn't exist, note this is used in distribute_to_shareholder function
        ensures !simple_map::spec_contains_key(contract.beneficiaries, shareholder) ==> result == shareholder;
    }

    spec set_terminate_vesting_contract {
        pragma verify = true;
        aborts_if !exists<VestingContract>(contract_address);
        let post vesting_contract_post = global<VestingContract>(contract_address);
        // ensure that the state of the vesting contract is set to VESTING_POOL_TERMINATED
        ensures vesting_contract_post.state == VESTING_POOL_TERMINATED;
    }
}
