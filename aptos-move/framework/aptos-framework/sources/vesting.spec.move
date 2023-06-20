spec aptos_framework::vesting {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec stake_pool_address(vesting_contract_address: address): address {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec vesting_start_secs(vesting_contract_address: address): u64 {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec period_duration_secs(vesting_contract_address: address): u64 {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec remaining_grant(vesting_contract_address: address): u64 {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec beneficiary(vesting_contract_address: address, shareholder: address): address {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec operator_commission_percentage(vesting_contract_address: address): u64 {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec vesting_contracts(admin: address): vector<address> {
        aborts_if false;
    }

    spec operator(vesting_contract_address: address): address {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec voter(vesting_contract_address: address): address {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec vesting_schedule(vesting_contract_address: address): VestingSchedule {
        aborts_if !exists<VestingContract>(vesting_contract_address);
    }

    spec total_accumulated_rewards(vesting_contract_address: address): u64 {
        // A severe timeout will occur without using partial. 
        pragma aborts_if_is_partial;

        include TotalAccumulatedRewardsAbortsIf;
    }

    spec schema TotalAccumulatedRewardsAbortsIf {
        vesting_contract_address: address;

        requires staking_contract.commission_percentage >= 0 && staking_contract.commission_percentage <= 100;

        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
        let vesting_contract = global<VestingContract>(vesting_contract_address);

        let staker = vesting_contract_address;
        let operator = vesting_contract.staking.operator;
        let staking_contracts = global<staking_contract::Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);

        aborts_if !exists<staking_contract::Store>(staker);
        aborts_if !simple_map::spec_contains_key(staking_contracts, operator);

        let pool_address = staking_contract.pool_address;
        let stake_pool = borrow_global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if active + pending_active > MAX_U64;
        aborts_if total_active_stake < staking_contract.principal;
        aborts_if accumulated_rewards * staking_contract.commission_percentage > MAX_U64;
        aborts_if (vesting_contract.remaining_grant + commission_amount) > total_active_stake;
        aborts_if total_active_stake < vesting_contract.remaining_grant;
    }

    spec accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64 {
        // TODO: A severe timeout can not be resolved.
        pragma verify = false;
        pragma aborts_if_is_partial;

        include TotalAccumulatedRewardsAbortsIf;
        let vesting_contract = global<VestingContract>(vesting_contract_address);
        let operator = vesting_contract.staking.operator;
        let staking_contracts = global<staking_contract::Store>(vesting_contract_address).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        let pool_address = staking_contract.pool_address;
        let stake_pool = borrow_global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        let total_accumulated_rewards = total_active_stake - vesting_contract.remaining_grant - commission_amount;

        let shareholder = spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);
        let pool = vesting_contract.grant_pool;
        let shares = pool_u64::spec_shares(pool, shareholder);
        aborts_if pool.total_coins > 0 && pool.total_shares > 0
            && (shares * total_accumulated_rewards) / pool.total_shares > MAX_U64;

        ensures result == pool_u64::spec_shares_to_amount_with_total_coins(pool, shares, total_accumulated_rewards);
    }

    spec shareholders(vesting_contract_address: address): vector<address> {
        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
    }

    spec fun spec_shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address;

    spec shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address {
        pragma verify = true;
        pragma opaque;
        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
        ensures [abstract] result == spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);
    }

    spec create_vesting_schedule(
        schedule: vector<FixedPoint32>,
        start_timestamp_secs: u64,
        period_duration: u64,
    ): VestingSchedule {
        aborts_if !(len(schedule) > 0);
        aborts_if !(period_duration > 0);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if !(start_timestamp_secs >= timestamp::now_seconds());
    }

    spec create_vesting_contract {
        // TODO: Data invariant does not hold.
        pragma verify = false;
        pragma aborts_if_is_partial;
    }

    spec unlock_rewards(contract_address: address) {
        // TODO: Calls `unlock_stake` which is not verified.
        pragma verify = false;
        pragma aborts_if_is_partial;
        include TotalAccumulatedRewardsAbortsIf { vesting_contract_address: contract_address };
    }

    spec unlock_rewards_many(contract_addresses: vector<address>) {
        // TODO: Calls `unlock_rewards` in loop.
        pragma verify = false;
        pragma aborts_if_is_partial;
    }

    spec vest(contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
        pragma aborts_if_is_partial;
    }

    spec vest_many(contract_addresses: vector<address>) {
        // TODO: Calls `vest` in loop.
        pragma aborts_if_is_partial;
    }

    spec distribute(contract_address: address) {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
    }

    spec distribute_many(contract_addresses: vector<address>) {
        // TODO: Calls `distribute` in loop.
        pragma aborts_if_is_partial;
    }

    spec terminate_vesting_contract(admin: &signer, contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma aborts_if_is_partial;
    }

    spec admin_withdraw(admin: &signer, contract_address: address) {
        // TODO: Calls `withdraw_stake` which is not verified.
        pragma aborts_if_is_partial;
        include VerifyAdminAbortsIf;
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if vesting_contract.state != VESTING_POOL_TERMINATED;
    }

    spec update_operator(
        admin: &signer,
        contract_address: address,
        new_operator: address,
        commission_percentage: u64,
    ) {
        // TODO: Calls `staking_contract::switch_operator` which is not verified.
        pragma aborts_if_is_partial;
        include VerifyAdminAbortsIf;
    }

    spec update_operator_with_same_commission(
        admin: &signer,
        contract_address: address,
        new_operator: address,
    ) {
        pragma verify = false;
    }

    spec update_voter(
        admin: &signer,
        contract_address: address,
        new_voter: address,
    ) {
        include VerifyAdminAbortsIf;

        let vesting_contract = global<VestingContract>(contract_address);
        let operator = vesting_contract.staking.operator;
        let staker = vesting_contract.signer_cap.account;

        include staking_contract::UpdateVoterSchema;
    }

    spec reset_lockup(
        admin: &signer,
        contract_address: address,
    ) {
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if signer::address_of(admin) != vesting_contract.admin;

        let operator = vesting_contract.staking.operator;
        let staker = vesting_contract.signer_cap.account;

        include staking_contract::ContractExistsAbortsIf {staker, operator};
        include staking_contract::IncreaseLockupWithCapAbortsIf {staker, operator};

        let store = global<staking_contract::Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.owner_cap.pool_address;
        aborts_if !exists<stake::StakePool>(vesting_contract.staking.pool_address);
    }

    spec set_beneficiary(
        admin: &signer,
        contract_address: address,
        shareholder: address,
        new_beneficiary: address,
    ) {
        aborts_if !account::exists_at(new_beneficiary);
        aborts_if !coin::is_account_registered<AptosCoin>(new_beneficiary);
        include VerifyAdminAbortsIf;
        let post vesting_contract = global<VestingContract>(contract_address);
        ensures simple_map::spec_contains_key(vesting_contract.beneficiaries,shareholder);
    }

    spec reset_beneficiary(
        account: &signer,
        contract_address: address,
        shareholder: address,
    ) { 
        aborts_if !exists<VestingContract>(contract_address);

        let addr = signer::address_of(account);
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if addr != vesting_contract.admin && !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);
        aborts_if addr != vesting_contract.admin && !exists<VestingAccountManagement>(contract_address);
        let roles = global<VestingAccountManagement>(contract_address).roles;
        let role = std::string::spec_utf8(ROLE_BENEFICIARY_RESETTER);
        aborts_if addr != vesting_contract.admin && !simple_map::spec_contains_key(roles, role);
        aborts_if addr != vesting_contract.admin && addr != simple_map::spec_get(roles, role);

        let post post_vesting_contract = global<VestingContract>(contract_address);
        ensures !simple_map::spec_contains_key(post_vesting_contract.beneficiaries,shareholder);
    }

    spec set_management_role(
        admin: &signer,
        contract_address: address,
        role: String,
        role_holder: address,
    ) {
        pragma aborts_if_is_partial;
        include SetManagementRoleAbortsIf;
    }

    spec set_beneficiary_resetter(
        admin: &signer,
        contract_address: address,
        beneficiary_resetter: address,
    ) {
        pragma aborts_if_is_partial;
        aborts_if !std::string::spec_internal_check_utf8(ROLE_BENEFICIARY_RESETTER);
        include SetManagementRoleAbortsIf;
    }

    spec get_role_holder(contract_address: address, role: String): address {
        aborts_if !exists<VestingAccountManagement>(contract_address);
        let roles = global<VestingAccountManagement>(contract_address).roles;
        aborts_if !simple_map::spec_contains_key(roles,role);
    }

    spec get_vesting_account_signer(admin: &signer, contract_address: address): signer {
        include VerifyAdminAbortsIf;
    }

    spec get_vesting_account_signer_internal(vesting_contract: &VestingContract): signer {
        aborts_if false;
    }

    spec fun spec_get_vesting_account_signer(vesting_contract: VestingContract): signer;

    spec create_vesting_contract_account(
        admin: &signer,
        contract_creation_seed: vector<u8>,
    ): (signer, SignerCapability) {
        let admin_addr = signer::address_of(admin);
        let admin_store = global<AdminStore>(admin_addr);
        let seed = bcs::to_bytes(admin_addr);
        let nonce = bcs::to_bytes(admin_store.nonce);

        let first = concat(seed, nonce);
        let second = concat(first, VESTING_POOL_SALT);
        let end = concat(second, contract_creation_seed);

        let resource_addr = account::spec_create_resource_address(admin_addr, end);
        aborts_if !exists<AdminStore>(admin_addr);
        aborts_if len(account::ZERO_AUTH_KEY) != 32;
        aborts_if admin_store.nonce + 1 > MAX_U64;
        let ea = account::exists_at(resource_addr);
        include if (ea) account::CreateResourceAccountAbortsIf else account::CreateAccountAbortsIf {addr: resource_addr};

        let acc = global<account::Account>(resource_addr);
        let post post_acc = global<account::Account>(resource_addr);
        aborts_if !exists<coin::CoinStore<AptosCoin>>(resource_addr) && !aptos_std::type_info::spec_is_struct<AptosCoin>();
        aborts_if !exists<coin::CoinStore<AptosCoin>>(resource_addr) && ea && acc.guid_creation_num + 2 > MAX_U64;
        aborts_if !exists<coin::CoinStore<AptosCoin>>(resource_addr) && ea && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        ensures exists<account::Account>(resource_addr) && post_acc.authentication_key == account::ZERO_AUTH_KEY && 
                exists<coin::CoinStore<AptosCoin>>(resource_addr);
        ensures signer::address_of(result_1) == resource_addr;
        ensures result_2.account == resource_addr;
    }

    spec verify_admin(admin: &signer, vesting_contract: &VestingContract) {
        aborts_if signer::address_of(admin) != vesting_contract.admin;
    }

    spec assert_vesting_contract_exists(contract_address: address) {
        aborts_if !exists<VestingContract>(contract_address);
    }

    spec assert_active_vesting_contract(contract_address: address) {
        include ActiveVestingContractAbortsIf<VestingContract>;
    }

    spec unlock_stake(vesting_contract: &VestingContract, amount: u64) {
        // TODO: Calls `staking_contract::unlock_stake` which is not verified.
        pragma aborts_if_is_partial;
    }

    spec withdraw_stake(vesting_contract: &VestingContract, contract_address: address): Coin<AptosCoin> {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma aborts_if_is_partial;
    }

    spec get_beneficiary(contract: &VestingContract, shareholder: address): address {
        aborts_if false;
    }

    spec schema SetManagementRoleAbortsIf {
        contract_address: address;
        admin: signer;
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        // aborts_if !exists<VestingAccountManagement>(contract_address) ==> exists<VestingAccountManagement>(vesting_contract.signer_cap.account);
        aborts_if signer::address_of(admin) != vesting_contract.admin;
    }

    spec schema VerifyAdminAbortsIf {
        contract_address: address;
        admin: signer;
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if signer::address_of(admin) != vesting_contract.admin;
    }

    spec schema ActiveVestingContractAbortsIf<VestingContract> {
        contract_address: address;
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if vesting_contract.state != VESTING_POOL_ACTIVE;
    }
}
