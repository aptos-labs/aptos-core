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
        // TODO: Verification out of resources/timeout
        pragma verify = false;
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
    }

    spec accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64 {
        // TODO: Uses `total_accumulated_rewards` which is not verified.
        pragma verify = false;
    }

    spec shareholders(vesting_contract_address: address): vector<address> {
        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
    }

    spec shareholder(vesting_contract_address: address, shareholder_or_beneficiary: address): address {
        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
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
    }

    spec unlock_rewards(contract_address: address) {
        // TODO: Calls `unlock_stake` which is not verified.
        pragma verify = false;
    }

    spec unlock_rewards_many(contract_addresses: vector<address>) {
        // TODO: Calls `unlock_rewards` in loop.
        pragma verify = false;
    }

    spec vest(contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
    }

    spec vest_many(contract_addresses: vector<address>) {
        // TODO: Calls `vest` in loop.
        pragma verify = false;
    }

    spec distribute(contract_address: address) {
        // TODO: Can't handle abort in loop.
        pragma verify = false;
    }

    spec distribute_many(contract_addresses: vector<address>) {
        // TODO: Calls `distribute` in loop.
        pragma verify = false;
    }

    spec terminate_vesting_contract(admin: &signer, contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
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
        // TODO: Unable to handle abort from `stake::assert_stake_pool_exists`.
        pragma aborts_if_is_partial;
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract1 = global<VestingContract>(contract_address);
        aborts_if signer::address_of(admin) != vesting_contract1.admin;

        let operator = vesting_contract1.staking.operator;
        let staker = vesting_contract1.signer_cap.account;

        include staking_contract::ContractExistsAbortsIf;
        include staking_contract::IncreaseLockupWithCapAbortsIf;
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
        // TODO: The abort of functions on either side of a logical operator can not be handled.
        pragma aborts_if_is_partial;
        aborts_if !exists<VestingContract>(contract_address);
        let post vesting_contract = global<VestingContract>(contract_address);
        ensures !simple_map::spec_contains_key(vesting_contract.beneficiaries,shareholder);
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
        // TODO: disabled due to timeout
        pragma verify=false;
        // TODO: Could not verify `coin::register` because can't get the `account_signer`.
        pragma aborts_if_is_partial;
        let admin_addr = signer::address_of(admin);
        let admin_store = global<AdminStore>(admin_addr);
        let seed = bcs::to_bytes(admin_addr);
        let nonce = bcs::to_bytes(admin_store.nonce);

        let first = concat(seed,nonce);
        let second = concat(first,VESTING_POOL_SALT);
        let end = concat(second,contract_creation_seed);

        let resource_addr = account::spec_create_resource_address(admin_addr, end);
        aborts_if !exists<AdminStore>(admin_addr);
        aborts_if len(account::ZERO_AUTH_KEY) != 32;
        aborts_if admin_store.nonce + 1 > MAX_U64;
        let ea = account::exists_at(resource_addr);
        include if (ea) account::CreateResourceAccountAbortsIf else account::CreateAccountAbortsIf {addr: resource_addr};
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
        pragma verify = false;
    }

    spec withdraw_stake(vesting_contract: &VestingContract, contract_address: address): Coin<AptosCoin> {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
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
