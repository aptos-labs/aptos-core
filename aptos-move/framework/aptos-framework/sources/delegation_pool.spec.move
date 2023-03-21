spec aptos_framework::delegation_pool {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec owner_cap_exists(addr: address): bool {
        aborts_if false;
    }

    spec get_owned_pool_address(owner: address): address {
        aborts_if !exists<DelegationPoolOwnership>(owner);
    }

    spec delegation_pool_exists(addr: address): bool {
        aborts_if false;
    }

    spec observed_lockup_cycle(pool_address: address): u64 {
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec operator_commission_percentage(pool_address: address): u64 {
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec shareholders_count_active_pool(pool_address: address): u64 {
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) {
        aborts_if !exists<DelegationPool>(pool_address);
        aborts_if !stake::stake_pool_exists(pool_address);
    }

    spec get_pending_withdrawal(
        pool_address: address,
        delegator_address: address
    ): (bool, u64) {
        pragma verify = false;
    }

    spec get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) {
        pragma verify = false;
    }

    spec get_add_stake_fee(pool_address: address, amount: u64): u64 {
        pragma verify = false;
    }

    spec can_withdraw_pending_inactive(pool_address: address): bool {
        pragma verify = false;
    }

    spec initialize_delegation_pool(
        owner: &signer,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
    ) {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        //Request 1: asserts_if
        //ERROR
        //Prover Can't Resolve features::
        requires features::spec_is_enabled(11);
        //Request 2: asserts_if exists<DelegationPoolOwnership>(owner) precondition
        //OK WITH QUESTION
        //Why need <DPO> in per-condition? the DP haven't been create yet
        requires exists<DelegationPoolOwnership>(signer::address_of(owner));
        //Request 3: asserts_if operator_commission_percentage > MAX_FEE
        //OK
        aborts_if operator_commission_percentage > MAX_FEE;
        //Request 4: exists<DelegationPoolOwnership>(owner) postcondition
        //OK
        ensures exists<DelegationPoolOwnership>(signer::address_of(owner));
        //Request 5: let pool_address = global<DelegationPoolOwnership>(owner).pool_address;
        //OK
        let pool_address = global<DelegationPoolOwnership>(signer::address_of(owner)).pool_address;
        //Request 6: exists<DelegationPool>(pool_address)
        //OK
        ensures exists<DelegationPool>(pool_address);
        //Request 7: exists<StakePool>(pool_address)
        //OK
        ensures stake::stake_pool_exists(pool_address);
        //Request 8: table::contains(pool.inactive_shares, pool.OLC): shares pool of pending_inactive stake always exists (cannot be deleted unless it becomes inactive)
        //OK
        let pool = global<DelegationPool>(pool_address);
        aborts_if !table::spec_contains(pool.inactive_shares, pool.observed_lockup_cycle);
        //Request 9: total_coins(pool.active_shares) == active + pending_active on StakePool
        //OK
        let stake_pool = global<stake::StakePool>(pool_address);
        let active_ =  coin::value(stake_pool.active);
        let inactive_ =  coin::value(stake_pool.inactive);
        let pending_active_ = coin::value(stake_pool.pending_active);
        let pending_inactive_ = coin::value(stake_pool.pending_inactive);
        invariant pool.active_shares.total_coins == active_ + pending_active_;
        //Request 10: total_coins(pool.inactive_shares[pool.OLC]) == pending_inactive
        //OK
        let inactive_pool_ = table::spec_get(pool.inactive_shares,pool.observed_lockup_cycle);
        invariant inactive_pool_.total_coins == pending_inactive_;
        //Request 11: total_coins_inactive == inactive on StakePool
        //OK
        invariant pool.total_coins_inactive == inactive_;
    }

    spec assert_owner_cap_exists(owner: address) {
        aborts_if !owner_cap_exists(owner);
    }

    spec assert_delegation_pool_exists(pool_address: address) {
        aborts_if !delegation_pool_exists(pool_address);
    }

    spec assert_min_active_balance(pool: &DelegationPool, delegator_address: address) {
        let pool_u64 = pool.active_shares;
        include AssertMinActiveBalanceAbortsIf;
    }
    spec schema AssertMinActiveBalanceAbortsIf {
        pool_u64: pool_u64::Pool;
        delegator_address: address;
        let shares = pool_u64::spec_shares(pool_u64, delegator_address);
        let total_coins = pool_u64.total_coins;
        let balance = pool_u64::spec_shares_to_amount_with_total_coins(pool_u64, shares, total_coins);
        aborts_if pool_u64.total_coins > 0 && pool_u64.total_shares > 0 && (shares * total_coins) / pool_u64.total_shares > MAX_U64;
        aborts_if balance < MIN_COINS_ON_SHARES_POOL;
    }

    spec assert_min_pending_inactive_balance(pool: &DelegationPool, delegator_address: address) {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
        let pool_u64 = table::spec_get(pool.inactive_shares, observed_lockup_cycle);
        include AssertMinActiveBalanceAbortsIf;
    }

    spec coins_to_redeem_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool,
        shareholder: address,
        amount: u64,
    ): u64 {
        include AmountToSharesToRedeemAbortsIf {
            shares_pool: src_shares_pool,
        };
    }

    spec coins_to_transfer_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool,
        dst_shares_pool: &pool_u64::Pool,
        shareholder: address,
        amount: u64,
    ): u64 {
        pragma verify = false;
    }

    spec retrieve_stake_pool_owner(pool: &DelegationPool): signer {
        aborts_if false;
    }

    spec get_pool_address(pool: &DelegationPool): address {
        aborts_if false;
    }

    spec olc_with_index(index: u64): ObservedLockupCycle {
        aborts_if false;
    }

    spec set_operator(
        owner: &signer,
        new_operator: address
    ) {
        pragma verify = false;
    }

    spec set_delegated_voter(
        owner: &signer,
        new_voter: address
    ) {
        pragma verify = false;
    }

    spec add_stake(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
    }

    spec unlock(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
    }

    spec reactivate_stake(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
    }

    spec withdraw(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
    }

    spec withdraw_internal(pool: &mut DelegationPool, delegator_address: address, amount: u64) {
        pragma verify = false;
    }

    spec pending_withdrawal_exists(pool: &DelegationPool, delegator_address: address): (bool, ObservedLockupCycle) {
        pragma verify = false;
    }

    spec pending_inactive_shares_pool_mut(pool: &mut DelegationPool): &mut pool_u64::Pool {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
    }

    spec pending_inactive_shares_pool(pool: &DelegationPool): &pool_u64::Pool {
        pragma verify = false;
    }

    spec execute_pending_withdrawal(pool: &mut DelegationPool, delegator_address: address) {
        pragma verify = false;
    }

    spec buy_in_pending_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        pragma verify = false;
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
    }

    spec amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        include AmountToSharesToRedeemAbortsIf;
        ensures result == spec_amount_to_shares_to_redeem(shares_pool, shareholder, coins_amount);
    }
    spec schema AmountToSharesToRedeemAbortsIf {
        shares_pool: pool_u64::Pool;
        shareholder: address;
        let shares = pool_u64::spec_shares(shares_pool, shareholder);
        let total_coins = shares_pool.total_coins;
        aborts_if shares_pool.total_coins > 0 && shares_pool.total_shares > 0 && (shares * total_coins) / shares_pool.total_shares > MAX_U64;
    }

    spec fun spec_amount_to_shares_to_redeem(
        shares_pool: pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        let shares = pool_u64::spec_shares(shares_pool, shareholder);
        let total_coins = shares_pool.total_coins;
        let balance = pool_u64::spec_shares_to_amount_with_total_coins(shares_pool, shares, total_coins);
        if (coins_amount >= balance) {
            shares
        } else {
            pool_u64::spec_amount_to_shares_with_total_coins(shares_pool, coins_amount, total_coins)
        }
    }

    spec redeem_active_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        let shares_pool = pool.active_shares;

        include AmountToSharesToRedeemAbortsIf;

        let shares_to_redeem = spec_amount_to_shares_to_redeem(pool.active_shares, shareholder, coins_amount);
        let redeemed_coins = pool_u64::spec_shares_to_amount_with_total_coins(shares_pool, shares_to_redeem, shares_pool.total_coins);

        aborts_if pool_u64::spec_shares(shares_pool, shareholder) < shares_to_redeem;
        aborts_if shares_pool.total_coins < redeemed_coins;
        aborts_if shares_pool.total_shares < shares_to_redeem;
    }

    spec redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: ObservedLockupCycle,
    ): u64 {
        pragma verify = false;
    }

    spec calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        pragma verify = false;
    }

    spec synchronize_delegation_pool(pool_address: address) {
        pragma aborts_if_is_strict = false;
        let post pool = global<DelegationPool>(pool_address);
        let pre_pool = global<DelegationPool>(pool_address);
        let stake_pool = global<stake::StakePool>(pool_address);
        let inactive = stake_pool.inactive.value;
        ensures pool.observed_lockup_cycle.index == pre_pool.observed_lockup_cycle.index + 1 ==> table::spec_contains(pool.inactive_shares,pool.observed_lockup_cycle);
    }

    spec multiply_then_divide(x: u64, y: u64, z: u64): u64 {
        aborts_if (x * y) / z > MAX_U64;
        aborts_if z == 0;
        ensures result == x * y / z;
        ensures z != 0;
    }

    spec to_u128(num: u64): u128 {
        aborts_if false;
    }
}
