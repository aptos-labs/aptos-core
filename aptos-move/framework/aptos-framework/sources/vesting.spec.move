spec aptos_framework::vesting {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: In order to retrieve the address of the underlying stake pool, the vesting start timestamp of the
    /// vesting contract, the duration of the vesting period, the remaining grant of a vesting contract, the beneficiary
    /// account of a shareholder in a vesting contract, the percentage of accumulated rewards that is paid to the
    /// operator as commission, the operator who runs the validator, the voter who will be voting on-chain, and the
    /// vesting schedule of a vesting contract, the supplied vesting contract should exist.
    /// Criticality: Low
    /// Implementation: The vesting_start_secs, period_duration_secs, remaining_grant, beneficiary,
    /// operator_commission_percentage, operator, voter, and vesting_schedule functions ensure that the supplied vesting
    /// contract address exists by calling the assert_vesting_contract_exists function.
    /// Enforcement: Formally verified via [high-level-req-1](assert_vesting_contract_exists).
    ///
    /// No.: 2
    /// Requirement: The vesting pool should not exceed a maximum of 30 shareholders.
    /// Criticality: Medium
    /// Implementation: The maximum number of shareholders a vesting pool can support is stored as a constant in
    /// MAXIMUM_SHAREHOLDERS which is passed to the pool_u64::create function.
    /// Enforcement: Formally verified via a [high-level-spec-2](global invariant).
    ///
    /// No.: 3
    /// Requirement: Retrieving all the vesting contracts of a given address and retrieving the list of beneficiaries from
    /// a vesting contract should never fail.
    /// Criticality: Medium
    /// Implementation: The function vesting_contracts checks if the supplied admin address contains an AdminStore
    /// resource and returns all the vesting contracts as a vector<address>. Otherwise it returns an empty vector. The
    /// function get_beneficiary checks for a given vesting contract, a specific shareholder exists, and if so, the
    /// beneficiary will be returned, otherwise it will simply return the address of the shareholder.
    /// Enforcement: Formally verified via [high-level-spec-3.1](vesting_contracts) and
    /// [high-level-spec-3.2](get_beneficiary).
    ///
    /// No.: 4
    /// Requirement: The shareholders should be able to start vesting only after the vesting cliff and the first vesting
    /// period have transpired.
    /// Criticality: High
    /// Implementation: The end of the vesting cliff is stored under VestingContract.vesting_schedule.start_timestamp_secs.
    /// The vest function always checks that timestamp::now_seconds is greater or equal to the end of the vesting cliff
    /// period.
    /// Enforcement: Audited the check for the end of vesting cliff: [https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/vesting.move#L566](vest) module.
    ///
    /// No.: 5
    /// Requirement: In order to retrieve the total accumulated rewards that have not been distributed, the accumulated
    /// rewards of a given beneficiary, the list of al shareholders in a vesting contract,the shareholder address given
    /// the beneficiary address in a given vesting contract, to terminate a vesting contract and to distribute any
    /// withdrawable stake from the stake pool, the supplied vesting contract should exist and be active.
    /// Criticality: Low
    /// Implementation: The distribute, terminate_vesting_contract, shareholder, shareholders, accumulated_rewards,
    /// and total_accumulated_rewards functions ensure that the supplied vesting contract address exists and is active
    /// by calling the assert_active_vesting_contract function.
    /// Enforcement: Formally verified via [high-level-spec-5](ActiveVestingContractAbortsIf).
    ///
    /// No.: 6
    /// Requirement: A new vesting schedule should not be allowed to start vesting in the past or to supply an empty
    /// schedule or for the period duration to be zero.
    /// Criticality: High
    /// Implementation: The create_vesting_schedule function ensures that the length of the schedule vector is greater
    /// than 0, that the period duration is greater than 0 and that the start_timestamp_secs is greater or equal to
    /// timestamp::now_seconds.
    /// Enforcement: Formally verified via [high-level-req-6](create_vesting_schedule).
    ///
    /// No.: 7
    /// Requirement: The shareholders should be able to vest the tokens from previous periods.
    /// Criticality: High
    /// Implementation: When vesting, the last_completed_period is checked against the next period to vest. This allows
    /// to unlock vested tokens for the next period since last vested, in case they didn't call vest for some periods.
    /// Enforcement: Audited that vesting doesn't skip periods, but gradually increments to allow shareholders to
    /// retrieve all the vested tokens.
    ///
    /// No.: 8
    /// Requirement: Actions such as obtaining a list of shareholders, calculating accrued rewards, distributing
    /// withdrawable stake, and terminating the vesting contract should be accessible exclusively while the vesting
    /// contract remains active.
    /// Criticality: Low
    /// Implementation: Restricting access to inactive vesting contracts is achieved through the
    /// assert_active_vesting_contract function.
    /// Enforcement: Formally verified via [high-level-spec-8](ActiveVestingContractAbortsIf).
    ///
    /// No.: 9
    /// Requirement: The ability to terminate a vesting contract should only be available to the owner.
    /// Criticality: High
    /// Implementation: Limiting the access of accounts to specific function, is achieved by asserting that the signer
    /// matches the admin of the VestingContract.
    /// Enforcement: Formally verified via [high-level-req-9](verify_admin).
    ///
    /// No.: 10
    /// Requirement: A new vesting contract should not be allowed to have an empty list of shareholders, have a different
    /// amount of shareholders than buy-ins, and provide a withdrawal address which is either reserved or not registered
    /// for apt.
    /// Criticality: High
    /// Implementation: The create_vesting_contract function ensures that the withdrawal_address is not a reserved
    /// address, that it is registered for apt, that the list of shareholders is non-empty, and that the amount of
    /// shareholders matches the amount of buy_ins.
    /// Enforcement: Formally verified via [high-level-req-10](create_vesting_contract).
    ///
    /// No.: 11
    /// Requirement: Creating a vesting contract account should require the signer (admin) to own an admin store and should
    /// enforce that the seed of the resource account is composed of the admin store's nonce, the vesting pool salt,
    /// and the custom contract creation seed.
    /// Criticality: Medium
    /// Implementation: The create_vesting_contract_account concatenates to the seed first the admin_store.nonce then
    /// the VESTING_POOL_SALT then the contract_creation_seed and then it is passed to the create_resource_account
    /// function.
    /// Enforcement: Enforced via [high-level-req-11](create_vesting_contract_account).
    /// </high-level-req>
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
        // property 2: The vesting pool should not exceed a maximum of 30 shareholders.
        /// [high-level-spec-2]
        invariant forall a: address where exists<VestingContract>(a):
            global<VestingContract>(a).grant_pool.shareholders_limit <= MAXIMUM_SHAREHOLDERS;
    }

    spec schema AbortsIfPermissionedSigner {
        use aptos_framework::permissioned_signer;
        s: signer;
        let perm = VestPermission {};
        aborts_if !permissioned_signer::spec_check_permission_exists(s, perm);
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
        /// [high-level-spec-3.1]
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
        // TODO: set because of timeout (property proved)
        pragma verify = false;

        include TotalAccumulatedRewardsAbortsIf;
    }

    spec schema TotalAccumulatedRewardsAbortsIf {
        vesting_contract_address: address;


        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
        let vesting_contract = global<VestingContract>(vesting_contract_address);

        let staker = vesting_contract_address;
        let operator = vesting_contract.staking.operator;
        let staking_contracts = global<staking_contract::Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);

        aborts_if !exists<staking_contract::Store>(staker);
        aborts_if !simple_map::spec_contains_key(staking_contracts, operator);

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if active + pending_active > MAX_U64;
        aborts_if total_active_stake < staking_contract.principal;
        aborts_if accumulated_rewards * staking_contract.commission_percentage > MAX_U64;
        // This two item both contribute to the timeout
        aborts_if (vesting_contract.remaining_grant + commission_amount) > total_active_stake;
        aborts_if total_active_stake < vesting_contract.remaining_grant;
    }

    spec accumulated_rewards(vesting_contract_address: address, shareholder_or_beneficiary: address): u64 {
        // TODO: A severe timeout can not be resolved.
        pragma verify = false;

        // This schema lead to timeout
        include TotalAccumulatedRewardsAbortsIf;

        let vesting_contract = global<VestingContract>(vesting_contract_address);
        let operator = vesting_contract.staking.operator;
        let staking_contracts = global<staking_contract::Store>(vesting_contract_address).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
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
        pragma opaque;
        include ActiveVestingContractAbortsIf<VestingContract>{contract_address: vesting_contract_address};
        ensures [abstract] result == spec_shareholder(vesting_contract_address, shareholder_or_beneficiary);
    }

    spec create_vesting_schedule(
        schedule: vector<FixedPoint32>,
        start_timestamp_secs: u64,
        period_duration: u64,
    ): VestingSchedule {
        /// [high-level-req-6]
        aborts_if !(len(schedule) > 0);
        aborts_if !(period_duration > 0);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if !(start_timestamp_secs >= timestamp::now_seconds());
    }

    spec create_vesting_contract {
        // TODO: Data invariant does not hold.
        pragma verify = false;
        /// [high-level-req-10]
        aborts_if withdrawal_address == @aptos_framework || withdrawal_address == @vm_reserved;
        aborts_if !exists<account::Account>(withdrawal_address);
        aborts_if !exists<coin::CoinStore<AptosCoin>>(withdrawal_address);
        aborts_if len(shareholders) == 0;
        // property 2: The vesting pool should not exceed a maximum of 30 shareholders.
        aborts_if simple_map::spec_len(buy_ins) != len(shareholders);
        ensures global<VestingContract>(result).grant_pool.shareholders_limit == 30;
    }

    spec unlock_rewards(contract_address: address) {
        // TODO: Calls `unlock_stake` which is not verified.
        // Current verification times out.
        pragma verify = false;
        include UnlockRewardsAbortsIf;
    }

    spec schema UnlockRewardsAbortsIf {
        contract_address: address;

        // Cause timeout here
        include TotalAccumulatedRewardsAbortsIf { vesting_contract_address: contract_address };

        let vesting_contract = global<VestingContract>(contract_address);
        let operator = vesting_contract.staking.operator;
        let staking_contracts = global<staking_contract::Store>(contract_address).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        let amount = total_active_stake - vesting_contract.remaining_grant - commission_amount;

        include UnlockStakeAbortsIf { vesting_contract, amount };
    }

    spec unlock_rewards_many(contract_addresses: vector<address>) {
        // TODO: Calls `unlock_rewards` in loop.
        pragma verify = false;
        aborts_if len(contract_addresses) == 0;
    }

    spec vest(contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
        include UnlockRewardsAbortsIf;
    }

    spec vest_many(contract_addresses: vector<address>) {
        // TODO: Calls `vest` in loop.
        pragma verify = false;
        aborts_if len(contract_addresses) == 0;
    }

    spec distribute(contract_address: address) {
        // TODO: Can't handle abort in loop.
        pragma verify = false;
        include ActiveVestingContractAbortsIf<VestingContract>;

        let vesting_contract = global<VestingContract>(contract_address);
        include WithdrawStakeAbortsIf { vesting_contract };
    }

    spec distribute_many(contract_addresses: vector<address>) {
        // TODO: Calls `distribute` in loop.
        pragma verify = false;
        aborts_if len(contract_addresses) == 0;
    }

    spec terminate_vesting_contract(admin: &signer, contract_address: address) {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
        include ActiveVestingContractAbortsIf<VestingContract>;

        let vesting_contract = global<VestingContract>(contract_address);
        include WithdrawStakeAbortsIf { vesting_contract };
    }

    spec admin_withdraw(admin: &signer, contract_address: address) {
        // TODO: Calls `withdraw_stake` which is not verified.
        pragma verify = false;

        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if vesting_contract.state != VESTING_POOL_TERMINATED;

        include VerifyAdminAbortsIf;
        include WithdrawStakeAbortsIf { vesting_contract };
    }

    spec update_operator(
        admin: &signer,
        contract_address: address,
        new_operator: address,
        commission_percentage: u64,
    ) {
        // TODO: Calls `staking_contract::switch_operator` which is not verified.
        pragma verify = false;

        include VerifyAdminAbortsIf;

        let vesting_contract = global<VestingContract>(contract_address);
        let acc = vesting_contract.signer_cap.account;
        let old_operator = vesting_contract.staking.operator;
        include staking_contract::ContractExistsAbortsIf { staker: acc, operator: old_operator };
        let store = global<staking_contract::Store>(acc);
        let staking_contracts = store.staking_contracts;
        aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);

        let staking_contract = simple_map::spec_get(staking_contracts, old_operator);
        include DistributeInternalAbortsIf { staker: acc, operator: old_operator, staking_contract, distribute_events: store.distribute_events };
    }

    spec update_operator_with_same_commission(
        admin: &signer,
        contract_address: address,
        new_operator: address,
    ) {
        pragma verify = false;
    }

    spec update_commission_percentage(
        admin: &signer,
        contract_address: address,
        new_commission_percentage: u64,
    ) {
        pragma verify = false;
    }

    spec update_voter(
        admin: &signer,
        contract_address: address,
        new_voter: address,
    ) {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
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
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
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
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        pragma aborts_if_is_partial;
        aborts_if !account::spec_exists_at(new_beneficiary);
        // TODO(fa_migration)
        // aborts_if !coin::spec_is_account_registered<AptosCoin>(new_beneficiary);
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

    spec set_beneficiary_for_operator(
        operator: &signer,
        new_beneficiary: address,
    ) {
        // TODO: temporary mockup
        pragma verify = false;
    }

    spec get_role_holder(contract_address: address, role: String): address {
        aborts_if !exists<VestingAccountManagement>(contract_address);
        let roles = global<VestingAccountManagement>(contract_address).roles;
        aborts_if !simple_map::spec_contains_key(roles,role);
    }

    spec get_vesting_account_signer(admin: &signer, contract_address: address): signer {
        pragma verify_duration_estimate = 120;
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
        pragma verify_duration_estimate = 300;
        let admin_addr = signer::address_of(admin);
        let admin_store = global<AdminStore>(admin_addr);
        let seed = bcs::to_bytes(admin_addr);
        let nonce = bcs::to_bytes(admin_store.nonce);

        let first = concat(seed, nonce);
        let second = concat(first, VESTING_POOL_SALT);
        let end = concat(second, contract_creation_seed);

        /// [high-level-req-11]
        let resource_addr = account::spec_create_resource_address(admin_addr, end);
        aborts_if !exists<AdminStore>(admin_addr);
        aborts_if len(account::ZERO_AUTH_KEY) != 32;
        aborts_if admin_store.nonce + 1 > MAX_U64;
        let ea = account::spec_exists_at(resource_addr);
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
        pragma verify_duration_estimate = 120;
        aborts_if permissioned_signer::spec_is_permissioned_signer(admin);
        /// [high-level-req-9]
        aborts_if signer::address_of(admin) != vesting_contract.admin;
        // include AbortsIfPermissionedSigner { s: admin };
    }

    spec assert_vesting_contract_exists(contract_address: address) {
        /// [high-level-req-1]
        aborts_if !exists<VestingContract>(contract_address);
    }

    spec assert_active_vesting_contract(contract_address: address) {
        include ActiveVestingContractAbortsIf<VestingContract>;
    }

    spec unlock_stake(vesting_contract: &VestingContract, amount: u64) {
        // TODO: Calls `staking_contract::unlock_stake` which is not verified.
        pragma verify = false;
        include UnlockStakeAbortsIf;
    }

    spec schema UnlockStakeAbortsIf {
        vesting_contract: &VestingContract;
        amount: u64;

        // verify staking_contract::unlock_stake()
        let acc = vesting_contract.signer_cap.account;
        let operator = vesting_contract.staking.operator;
        include amount != 0 ==> staking_contract::ContractExistsAbortsIf { staker: acc, operator };

        // verify staking_contract::distribute_internal()
        let store = global<staking_contract::Store>(acc);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        include amount != 0 ==> DistributeInternalAbortsIf { staker: acc, operator, staking_contract, distribute_events: store.distribute_events };
    }

    spec withdraw_stake(vesting_contract: &VestingContract, contract_address: address): Coin<AptosCoin> {
        // TODO: Calls `staking_contract::distribute` which is not verified.
        pragma verify = false;
        include WithdrawStakeAbortsIf;
    }

    spec schema WithdrawStakeAbortsIf {
        vesting_contract: &VestingContract;
        contract_address: address;

        let operator = vesting_contract.staking.operator;
        include staking_contract::ContractExistsAbortsIf { staker: contract_address, operator };

        // verify staking_contract::distribute_internal()
        let store = global<staking_contract::Store>(contract_address);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        include DistributeInternalAbortsIf { staker: contract_address, operator, staking_contract, distribute_events: store.distribute_events };
    }

    spec schema DistributeInternalAbortsIf {
        staker: address;    // The verification below does not contain the loop in staking_contract::update_distribution_pool().
        operator: address;
        staking_contract: staking_contract::StakingContract;
        distribute_events: EventHandle<staking_contract::DistributeEvent>;

        let pool_address = staking_contract.pool_address;
        aborts_if !exists<stake::StakePool>(pool_address);
        let stake_pool = global<stake::StakePool>(pool_address);
        let inactive = stake_pool.inactive.value;
        let pending_inactive = stake_pool.pending_inactive.value;
        aborts_if inactive + pending_inactive > MAX_U64;

        // verify stake::withdraw_with_cap()
        let total_potential_withdrawable = inactive + pending_inactive;
        let pool_address_1 = staking_contract.owner_cap.pool_address;
        aborts_if !exists<stake::StakePool>(pool_address_1);
        let stake_pool_1 = global<stake::StakePool>(pool_address_1);
        aborts_if !exists<stake::ValidatorSet>(@aptos_framework);
        let validator_set = global<stake::ValidatorSet>(@aptos_framework);
        let inactive_state = !stake::spec_contains(validator_set.pending_active, pool_address_1)
            && !stake::spec_contains(validator_set.active_validators, pool_address_1)
            && !stake::spec_contains(validator_set.pending_inactive, pool_address_1);
        let inactive_1 = stake_pool_1.inactive.value;
        let pending_inactive_1 = stake_pool_1.pending_inactive.value;
        let new_inactive_1 = inactive_1 + pending_inactive_1;
        aborts_if inactive_state && timestamp::spec_now_seconds() >= stake_pool_1.locked_until_secs
            && inactive_1 + pending_inactive_1 > MAX_U64;
    }

    spec get_beneficiary(contract: &VestingContract, shareholder: address): address {
        /// [high-level-spec-3.2]
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

        aborts_if permissioned_signer::spec_is_permissioned_signer(admin);
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        aborts_if signer::address_of(admin) != vesting_contract.admin;
    }

    spec schema ActiveVestingContractAbortsIf<VestingContract> {
        contract_address: address;
        /// [high-level-spec-5]
        aborts_if !exists<VestingContract>(contract_address);
        let vesting_contract = global<VestingContract>(contract_address);
        /// [high-level-spec-8]
        aborts_if vesting_contract.state != VESTING_POOL_ACTIVE;
    }
}
