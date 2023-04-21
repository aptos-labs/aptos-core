spec aptos_framework::delegation_pool {
    spec module {
        pragma verify = false;
        pragma aborts_if_is_partial;
        apply stake::ResourceRequirement to *;
        apply NullShareholderAddrRequirement to
        withdraw, add_stake, unlock, reactivate_stake, synchronize_delegation_pool;
        apply GlobalPendingWithdrawlOwner to *;
        // TODO(Teng): long verification time
        apply TotalCoinInactiveLEInactiveStakePool to
        withdraw, add_stake, unlock, reactivate_stake, synchronize_delegation_pool;
        // TODO(Teng): Timeout
        // apply RedeemInactive to
        // withdraw, add_stake, unlock, reactivate_stake, synchronize_delegation_pool;
        apply WithdrawRequirements  {
            pool: global<DelegationPool>(pool_address)
        } to withdraw, add_stake, unlock, reactivate_stake, synchronize_delegation_pool;

        // Global property 1 [OK]:
        invariant forall addr: address: exists<DelegationPool>(addr) ==> exists<stake::StakePool>(addr);

        // Global property 2 [OK]:
        invariant forall addr: address: exists<DelegationPool>(addr) ==> (global<DelegationPool>(addr).stake_pool_signer_cap.account == addr);

        // Used in redeem_inactive_shares
        global ghost_share_redeem: u64;

        // Used in synchronize_delegation_pool
        global sync_commission_active: u64;
        global sync_commission_pending_inactive: u64;
        global sync_delegation_pool_snap_before_buy_in: DelegationPool;
        global sync_inactive_snap_before_buy_in: pool_u64::Pool;

        // Used in add_stake
        global add_stake_pool_share_after_sync: pool_u64::Pool;
        global add_stake_pool_share_before_2nd_buy_in: pool_u64::Pool;

        // Used in unlock/reactivate
        global ghost_source_pool: pool_u64::Pool;
        global ghost_dest_pool: pool_u64::Pool;
        global pre_inactive_shares_OLC_delegator: u64;
        global pre_active_shares_OLC_delegator: u64;
        global ghost_amount: u64;
    }

    spec DelegationPool {

        // Global property 3 [ISSUE]:
        // Requires sum operator

        // TODO(Teng): Timeout
        // Global property 5
        // invariant forall delegator_address: address, i: u64:
        //     pool_u64::spec_shares(table::spec_get(
        //         inactive_shares, ObservedLockupCycle{index: i}), delegator_address) != 0 ==>
        //         (table::spec_contains(pending_withdrawals, delegator_address) &&
        //             table::spec_get(pending_withdrawals, delegator_address).index == i);

        // TODO(Teng): long verification time
        // Global property 6
        invariant forall delegator_address: address: table::spec_contains(pending_withdrawals, delegator_address)
            ==> pool_u64::spec_shares(table::spec_get(inactive_shares,
            table::spec_get(pending_withdrawals, delegator_address)), delegator_address) != 0;


        // TODO(Teng): long verification time
        // Global property 9
        invariant forall delegator_address: address: table::spec_contains(pending_withdrawals, delegator_address) ==>
            (table::spec_get(pending_withdrawals, delegator_address).index
                <= observed_lockup_cycle.index);

        // TODO(Teng): Timeout
        // Global property 8 [TIMEOUT]
        // invariant forall i in 0..observed_lockup_cycle.index:
        //     table::spec_contains(inactive_shares, ObservedLockupCycle{index: i}) ==>
        //         table::spec_get(inactive_shares, ObservedLockupCycle{index: i}).total_coins != 0;

        // TODO(Teng): Timeout
        // Global property 4 [TIMEOUT]
        // invariant forall i in 0..observed_lockup_cycle.index + 1,
        // j in 0..observed_lockup_cycle.index + 1, delegator_address: address:
        //     (table::spec_contains(inactive_shares, ObservedLockupCycle{index: i}) &&
        //     table::spec_contains(inactive_shares, ObservedLockupCycle{index: j}) &&
        //     pool_u64::spec_shares(table::spec_get(inactive_shares,
        //         ObservedLockupCycle{index: i}), delegator_address) != 0 &&
        //     pool_u64::spec_shares(table::spec_get(inactive_shares,
        //         ObservedLockupCycle{index: j}), delegator_address) != 0) ==>
        //     i == j;

        // TODO(Teng): long verification time
        // Global property 7
        invariant table::spec_contains(inactive_shares, observed_lockup_cycle);
    }

    spec schema NullShareholderAddrRequirement {
        pool_address: address;
        requires global<stake::StakePool>(pool_address).operator_address != NULL_SHAREHOLDER;
        requires pool_address != NULL_SHAREHOLDER;
    }

    spec schema PoolDelegatorAddressNotEq {
        pool_address: address;
        delegator_address: address;
        requires pool_address != delegator_address;
    }

    // This invariant is used in the context of retriving stake pools using the resource address
    // Not set as a global invariant because it does not hold in stake::extract_owner_cap and stake::deposit_owner_cap
    spec schema GlobalPendingWithdrawlOwner {
        invariant forall addr: address: exists<DelegationPool>(addr)
            ==> (exists<stake::OwnerCapability>(addr) && global<stake::OwnerCapability>(addr).pool_address == addr);
    }

    spec schema TotalCoinInactiveLEInactiveStakePool {
        pool_address: address;
        // Global property 11
        //TODO(Teng): long verification time
        invariant global<DelegationPool>(pool_address).total_coins_inactive <=
            spec_get_stake(global<DelegationPool>(pool_address)).inactive.value;
        // The invariant below will timeout:
        // invariant forall addr: address: exists<DelegationPool>(addr)
        //     ==> global<DelegationPool>(addr).total_coins_inactive <= global<stake::StakePool>(addr).inactive.value;
    }

    spec schema WithdrawRequirements {
        pool: DelegationPool;
        // Use together with global property 2
        requires exists<DelegationPool>(pool.stake_pool_signer_cap.account);
        requires pool.total_coins_inactive <= spec_get_stake(pool).inactive.value;
        // TODO(Teng):
        // This does not hold during execution of synchronize_delegation_pool
        // when calling the update_total_coins for pending_inactive_shares_pool_mut(pool)
        // requires forall i in 0..pool.observed_lockup_cycle.index + 1:
        //     pool.total_coins_inactive >= table::spec_get(pool.inactive_shares, ObservedLockupCycle{index: i}).total_coins;
    }

    // Schema for global property 10
    spec schema RedeemInactive {
        pool_address: address;
        ensures forall i in 0..global<DelegationPool>(pool_address).observed_lockup_cycle.index:
            table::spec_contains(global<DelegationPool>(pool_address).inactive_shares, ObservedLockupCycle{index: i}) ==>
                table::spec_get(global<DelegationPool>(pool_address).inactive_shares, ObservedLockupCycle{index: i}).total_coins
                    <= table::spec_get(old(global<DelegationPool>(pool_address)).inactive_shares, ObservedLockupCycle{index: i}).total_coins;
    }

    spec fun spec_multiply_then_divide(x: u64, y: u64, z: u64): u64;

    spec multiply_then_divide {
        // TODO(Teng): proved but commented out because of long execution time
        pragma verify = false;
        aborts_if z == 0;
        ensures [concrete] result == (x * y) / z;
        ensures [abstract] result == spec_multiply_then_divide(x, y, z);
    }

    spec owner_cap_exists(addr: address): bool {
        pragma verify = true;
        aborts_if false;
    }

    spec get_owned_pool_address(owner: address): address {
        pragma verify = true;
        aborts_if !exists<DelegationPoolOwnership>(owner);
    }

    /// Return whether a delegation pool exists at supplied address `addr`.
    spec delegation_pool_exists(addr: address): bool {
        pragma verify = true;
        aborts_if false;
    }

    spec observed_lockup_cycle(pool_address: address): u64 {
        pragma verify = true;
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec operator_commission_percentage(pool_address: address): u64 {
        pragma verify = true;
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec shareholders_count_active_pool(pool_address: address): u64 {
        pragma verify = true;
        aborts_if !exists<DelegationPool>(pool_address);
    }

    spec get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) {
        pragma verify = true;
        aborts_if !exists<DelegationPool>(pool_address);
        aborts_if !stake::stake_pool_exists(pool_address);
    }

    spec get_pending_withdrawal(
    pool_address: address,
    delegator_address: address
    ): (bool, u64) {
        pragma verify = true;
        pragma aborts_if_is_partial;
        aborts_if !delegation_pool_exists(pool_address);
    }

    spec get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) {
        pragma verify = true;
    }

    spec fun spec_get_add_stake_fee(pool_address: address, amount: u64): u64;

    spec get_add_stake_fee(pool_address: address, amount: u64): u64 {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] result == spec_get_add_stake_fee(pool_address, amount);
    }

    spec can_withdraw_pending_inactive(pool_address: address): bool {
        pragma verify = true;
        pragma opaque;
        aborts_if !exists<stake::ValidatorSet>(@aptos_framework);
        ensures result == spec_can_withdraw_pending_inactive(pool_address);
    }

    spec fun spec_can_withdraw_pending_inactive(pool_address: address): bool {
        stake::spec_get_validator_state_inactive(pool_address) &&
            timestamp::now_seconds() >= stake::spec_get_lockup_secs(pool_address)
    }

    spec initialize_delegation_pool(
    owner: &signer,
    operator_commission_percentage: u64,
    delegation_pool_creation_seed: vector<u8>,
    ) {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        //Property 1 [OK]: asserts_if !features::delegation_pools_enabled()

        aborts_if !features::spec_is_enabled(features::DELEGATION_POOLS);
        //Property 1 [OK]: asserts_if exists<DelegationPoolOwnership>(owner) precondition
        //OK
        aborts_if exists<DelegationPoolOwnership>(signer::address_of(owner));
        //Request 3: Sasserts_if operator_commission_percentage > MAX_FEE
        //OK
        aborts_if operator_commission_percentage > MAX_FEE;
        //Request 4: exists<DelegationPoolOwnership>(owner) postcondition
        //OK
        ensures exists<DelegationPoolOwnership>(signer::address_of(owner));
        //Request 5: let pool_address = global<DelegationPoolOwnership>(owner).pool_address;
        //OK
        let post pool_address = global<DelegationPoolOwnership>(signer::address_of(owner)).pool_address;
        //Request 6: exists<DelegationPool>(pool_address)
        //OK
        ensures exists<DelegationPool>(pool_address);
        //Request 7: exists<StakePool>(pool_address)
        //OK
        ensures stake::stake_pool_exists(pool_address);
        //Request 8: table::contains(pool.inactive_shares, pool.OLC): shares pool of pending_inactive stake always exists (cannot be deleted unless it becomes inactive)
        //OK
        let post pool = global<DelegationPool>(pool_address);
        ensures table::spec_contains(pool.inactive_shares, pool.observed_lockup_cycle);
        //Request 9: total_coins(pool.active_shares) == active + pending_active on StakePool
        //OK
        let post stake_pool = global<stake::StakePool>(pool_address);
        ensures pool.active_shares.total_coins == coin::value(stake_pool.active) + coin::value(stake_pool.pending_active);
        //Request 10: total_coins(pool.inactive_shares[pool.OLC]) == pending_inactive
        //OK
        ensures table::spec_get(pool.inactive_shares,pool.observed_lockup_cycle).total_coins == coin::value(stake_pool.pending_inactive);
        //Request 11: total_coins_inactive == inactive on StakePool
        //OK
        ensures pool.total_coins_inactive == coin::value(stake_pool.inactive);

    }

    spec assert_owner_cap_exists(owner: address) {
        pragma verify = true;
        aborts_if !owner_cap_exists(owner);
    }

    spec assert_delegation_pool_exists(pool_address: address) {
        pragma verify = true;
        aborts_if !delegation_pool_exists(pool_address);
    }

    spec assert_min_active_balance(pool: &DelegationPool, delegator_address: address) {
        pragma verify = true;
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
        pragma verify = true;
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
        pragma verify = true;
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
        pragma verify = true;
        include AmountToSharesToRedeemAbortsIf {
            shares_pool: src_shares_pool,
        };
    }

    spec retrieve_stake_pool_owner(pool: &DelegationPool): signer {
        pragma verify = true;
        ensures signer::address_of(result) == pool.stake_pool_signer_cap.account;
    }

    spec set_operator(
        owner: &signer,
        new_operator: address
    ) {
        let pool_address = global<DelegationPoolOwnership>(signer::address_of(owner)).pool_address;
        include NullShareholderAddrRequirement;
        include TotalCoinInactiveLEInactiveStakePool;
        include WithdrawRequirements {
            pool: global<DelegationPool>(pool_address)
        };
        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        pragma verify = true;
    }

    spec set_delegated_voter(
        owner: &signer,
        new_voter: address
    ) {
        let pool_address = global<DelegationPoolOwnership>(signer::address_of(owner)).pool_address;
        include NullShareholderAddrRequirement;
        include TotalCoinInactiveLEInactiveStakePool;
        include WithdrawRequirements {
            pool: global<DelegationPool>(pool_address)
        };
        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        pragma verify = true;
    }

    spec add_stake(delegator: &signer, pool_address: address, amount: u64) {

        // TODO(Teng): long execution time
        pragma verify = false;
        pragma aborts_if_is_partial = true;

        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        include PoolDelegatorAddressNotEq;

        aborts_if amount > 0 && !exists<DelegationPool>(pool_address);

        let delegator_address = signer::address_of(delegator);

        let pre_pool = global<DelegationPool>(pool_address);
        let pre_stake_pool = spec_get_stake(pre_pool);
        let pre_stake_active_value = pre_stake_pool.active.value;
        let pre_stake_pending_active_value = pre_stake_pool.pending_active.value;

        let post pool = global<DelegationPool>(pool_address);
        let post stake_pool = spec_get_stake(pool);
        let post stake_active_value = stake_pool.active.value;
        let post stake_pending_active_value = stake_pool.pending_active.value;

        // TODO(Teng):
        // Property 1 [ISSUE]: aborts_if pool_u64::balance(pool.active_shares, delegator) < MIN_COINS_ON_SHARES_POOL
        // cannot use ghost var in the middle state to specify aborts_if conditions
        // add_stake_fee is hard to model in the spec

        // Property 2 [OK with long verification time]: ensures active + pending_active == old(active) + old(pending_active) + amount
        ensures pre_stake_active_value + pre_stake_pending_active_value + amount == stake_active_value + stake_pending_active_value;

        //Property 3 [OK: long verification time]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures amount > 0 ==> global<DelegationPool>(pool_address).active_shares.total_coins
            == coin::value(global<stake::StakePool>(pool_address).active) + coin::value(global<stake::StakePool>(pool_address).pending_active);

        // TODO(Teng): Timeout
        // Property 4 [Timeout]: pool_u64::shares(pool.active_shares, delegator) - pool_u64::shares(old(pool).active_shares, delegator) == pool_u64::amount_to_shares(pool.active_shares, amount - get_add_stake_fee(pool_address, amount))
        // let active_amount = amount - spec_get_add_stake_fee(pool_address, amount);
        // ensures amount > 0 ==> pool_u64::spec_shares(pool.active_shares, delegator_address)
        //     == pool_u64::spec_shares(add_stake_pool_share_after_sync, delegator_address)
        // + pool_u64::spec_amount_to_shares_with_total_coins(add_stake_pool_share_after_sync,
        //     active_amount, add_stake_pool_share_after_sync.total_coins);

        // TODO(Teng): long verification time
        // Property 5 [OK with long verification time]: pool_u64::shares(pool.active_shares, NULL_SHAREHOLDER) - pool_u64::shares(old(pool).active_shares, NULL_SHAREHOLDER) == pool_u64::amount_to_shares(pool.active_shares, get_add_stake_fee(pool_address, amount))
        ensures amount > 0 ==> pool_u64::spec_shares(pool.active_shares, NULL_SHAREHOLDER) ==
            pool_u64::spec_shares(add_stake_pool_share_before_2nd_buy_in, NULL_SHAREHOLDER)
                + pool_u64::spec_amount_to_shares_with_total_coins(add_stake_pool_share_before_2nd_buy_in,
            spec_get_add_stake_fee(pool_address, amount), add_stake_pool_share_before_2nd_buy_in.total_coins);

        //Property 6 [OK]:delegator-balance == old(delegator-balance) - amount
        // requires that pool_address != delegator_address and operat_address != delegator_address
        ensures (amount > 0 && global<stake::StakePool>(pool_address).operator_address != delegator_address) ==> coin::balance<AptosCoin>(delegator_address) == old(coin::balance<AptosCoin>(delegator_address)) - amount;

        //Property 7 [OK]: resource-account-balance == old(resource-account-balance)
        ensures global<AptosCoin>(pool_address) == old(global<AptosCoin>(pool_address));

    }

    spec unlock(delegator: &signer, pool_address: address, amount: u64) {

        //TODO(Teng): Timeout
        // redeem_active causes 200s extra verification time
        pragma verify = false;
        pragma aborts_if_is_partial;

        requires amount > 0;

        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        include PoolDelegatorAddressNotEq;
        aborts_if !exists<DelegationPool>(pool_address);

        let pre_pool = global<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);
        let post pool = global<DelegationPool>(pool_address);
        let pre_stake_pool = spec_get_stake(pre_pool);
        let post stake_pool = spec_get_stake(pool);

        //Property 1 [TIMEOUT]: pool_u64::shares_to_amount(
        //source_pool, // snapshot of shares pool before redeem_shares
        //pool_u64::shares(old(source_pool), delegator) -
        //pool_u64::shares(source_pool, delegator)
        //) == amount (its latest value)

        ensures pool_u64::spec_shares_to_amount_with_total_coins(ghost_source_pool,
            pool_u64::spec_shares(ghost_source_pool, delegator_address) -
                pool_u64::spec_shares(pool.active_shares, delegator_address),
            ghost_source_pool.total_coins)
            == ghost_amount;

        //Property 2 [TIMEOUT]: pool_u64::shares(destination_pool, delegator) -
        //pool_u64::shares(old(destination_pool), delegator) ==
        //pool_u64::amount_to_shares(
        //destination_pool, // snapshot of shares pool before buy_in
        //amount (its latest value)
        //)

        ensures pool_u64::spec_shares(table::spec_get(global<DelegationPool>(pool_address).inactive_shares, global<DelegationPool>(pool_address).observed_lockup_cycle), delegator_address)
            - pool_u64::spec_shares(ghost_dest_pool, delegator_address)
            == pool_u64::spec_amount_to_shares_with_total_coins(ghost_dest_pool, ghost_amount, ghost_dest_pool.total_coins);

        //Property 3 [TIMEOUT]: pool_u64::balance(pool.inactive_shares[pool.OLC], delegator) + pool_u64::balance(pool.active_shares, delegator) <= pool_u64::balance(old(inactive_shares[pool.OLC]), delegator) + pool_u64::balance(old(active_shares), delegator)
        let post inactive_shares_OLC_delegator = pool_u64::spec_balance(table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle), delegator_address);
        let post active_shares_OLC_delegator = pool_u64::spec_balance(pool.active_shares, delegator_address);
        ensures pre_inactive_shares_OLC_delegator + pre_active_shares_OLC_delegator <= inactive_shares_OLC_delegator + active_shares_OLC_delegator;

        //Property 4 [TIMEOUT]: total_coins(old(source_pool)) - total_coins(source_pool)
        // == total_coins(destination_pool) - total_coins(old(destination_pool))
        // == amount (its latest value)
        // Note that here old(...) means the state after synchronization
        ensures ghost_source_pool.total_coins
            - global<DelegationPool>(pool_address).active_shares.total_coins == ghost_amount;
        ensures table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins - ghost_dest_pool.total_coins == ghost_amount;


        //Property 5 [TIMEOUT]: abs(active - old(active)) == abs(pending_inactive - old(pending_inactive))
        ensures stake_pool.active.value - pre_stake_pool.pending_inactive.value ==
            pre_stake_pool.pending_inactive.value - stake_pool.pending_inactive.value;

        //Property 6 [TIMEOUT]: pending_active == old(pending_active): no pending_active stake can be displaced
        ensures pre_stake_pool.pending_active == stake_pool.pending_active;

        //Property 7 [TIMEOUT]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures global<DelegationPool>(pool_address).active_shares.total_coins == stake_pool.active.value + stake_pool.pending_active.value;

        //Property 8 [Timeout]
        // total_coins(pending_inactive_shares_pool(pool)) == pending_inactive on StakePool
        //ensures global<DelegationPool>(pool_address).total_coins_inactive == stake_pool.pending_inactive.value;

        //TODO(Teng): need to use specification functions for correctly specifying two aborts_if conditions
        //Property 9 [ISSUE]: aborts_if pool_u64::balance(destination_pool, delegator) < MIN_COINS_ON_SHARES_POOL

        //Property 10 [ISSUE]: pool_u64::balance(source_pool, delegator) >= MIN_COINS_ON_SHARES_POOL or == 0
    }

    spec reactivate_stake(delegator: &signer, pool_address: address, amount: u64) {
        //TODO(Teng): Timeout
        pragma verify = false;
        pragma aborts_if_is_partial;

        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        include PoolDelegatorAddressNotEq;
        aborts_if exists<DelegationPool>(pool_address);

        let pre_pool = global<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);
        let post pool = global<DelegationPool>(pool_address);
        let pre_stake_pool = spec_get_stake(pre_pool);
        let post stake_pool = spec_get_stake(pool);

        // which reactivate func is not much likely processed like unlock func.
        requires amount > 0;

        //Property 1 [TIMEOUT]: pool_u64::shares_to_amount(
        //source_pool, // snapshot of shares pool before redeem_shares
        //pool_u64::shares(old(source_pool), delegator) -
        //pool_u64::shares(source_pool, delegator)
        //) == amount (its latest value)
        // similar to property 2) of withdraw_internal

        ensures pool_u64::spec_shares_to_amount_with_total_coins(ghost_source_pool,
            pool_u64::spec_shares(ghost_source_pool, delegator_address) -
                pool_u64::spec_shares(table::spec_get(global<DelegationPool>(pool_address).inactive_shares, global<DelegationPool>(pool_address).observed_lockup_cycle), delegator_address),
            ghost_source_pool.total_coins)
            == ghost_amount;

        //Property 2 [TIMEOUT]: pool_u64::shares(destination_pool, delegator) -
        //pool_u64::shares(old(destination_pool), delegator) ==
        //pool_u64::amount_to_shares(
        //destination_pool, // snapshot of shares pool before buy_in
        //amount (its latest value)
        //)

        ensures pool_u64::spec_shares(global<DelegationPool>(pool_address).active_shares, delegator_address) - pool_u64::spec_shares(ghost_dest_pool, delegator_address)
            == pool_u64::spec_amount_to_shares_with_total_coins(ghost_dest_pool,
            ghost_amount, ghost_dest_pool.total_coins);


        //Property 3 [TIMEOUT]: pool_u64::balance(pool.inactive_shares[pool.OLC], delegator) + pool_u64::balance(pool.active_shares, delegator)
        // <= pool_u64::balance(old(inactive_shares[pool.OLC]), delegator) + pool_u64::balance(old(active_shares), delegator)
        let post inactive_shares_OLC_delegator = pool_u64::spec_balance(table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle), delegator_address);
        let post active_shares_OLC_delegator = pool_u64::spec_balance(pool.active_shares, delegator_address);
        ensures pre_inactive_shares_OLC_delegator + pre_active_shares_OLC_delegator <= inactive_shares_OLC_delegator + active_shares_OLC_delegator;


        //Property 4 [TIMEOUT]: total_coins(old(source_pool)) - total_coins(source_pool)
        // == total_coins(destination_pool) - total_coins(old(destination_pool))
        // == amount (its latest value)
        ensures ghost_source_pool.total_coins - table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == ghost_amount;
        ensures global<DelegationPool>(pool_address).active_shares.total_coins - ghost_dest_pool.total_coins == ghost_amount;

        //Property 5 [TIMEOUT]: abs(active - old(active)) == abs(pending_inactive - old(pending_inactive))
        ensures stake_pool.active.value - pre_stake_pool.pending_inactive.value ==
            pre_stake_pool.pending_inactive.value - stake_pool.pending_inactive.value;

        //Property 6 [TIMEOUT]: pending_active == old(pending_active): no pending_active stake can be displaced
        ensures pre_stake_pool.pending_active == stake_pool.pending_active;

        //Property 7 [TIMEOUT]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures global<DelegationPool>(pool_address).active_shares.total_coins == stake_pool.active.value + stake_pool.pending_active.value;

        // Property 8 [TIMEOUT]
        // total_coins(pending_inactive_shares_pool(pool)) == pending_inactive on StakePool
        //ensures global<DelegationPool>(pool_address).total_coins_inactive == stake_pool.pending_inactive.value;

        //TODO(Teng): need to use specification functions for correctly specifying two aborts_if conditions
        //Property 9 [ISSUE]: aborts_if pool_u64::balance(destination_pool, delegator) < MIN_COINS_ON_SHARES_POOL

        //Property 10 [ISSUE]: pool_u64::balance(source_pool, delegator) >= MIN_COINS_ON_SHARES_POOL or == 0
    }

    spec withdraw(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
        pragma aborts_if_is_partial;
        aborts_if !exists<DelegationPool>(pool_address);
        // Property 1 [OK]: aborts_if amount == 0;
        aborts_if amount == 0;
        let pool = global<DelegationPool>(pool_address);
        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        include PoolDelegatorAddressNotEq {
            delegator_address: signer::address_of(delegator)
        };
        aborts_if exists<DelegationPool>(pool_address);
    }

    spec withdraw_internal(pool: &mut DelegationPool, delegator_address: address, amount: u64) {

        pragma verify = true;
        pragma aborts_if_is_partial;

        let pool_address = pool.stake_pool_signer_cap.account;

        include PoolDelegatorAddressNotEq;
        include WithdrawRequirements;

        // Property 1 [OK]: !(withdrawal_exists && (withdrawing_inactive_stake ||
        // can_withdraw_pending_inactive(pool_address))) =>
        // 1) delegator-balance == old(delegator-balance)
        // 2) there are no state changes on
        // DelegationPool and StakePool: if the pending withdrawal does not exist or is not
        // inactive and cannot withdraw pending_inactive stake, then nothing happens
        let withdrawal_exists = spec_pending_withdrawal_exists_first(pool, delegator_address);
        let withdrawal_olc = spec_pending_withdrawal_exists_second(pool, delegator_address);
        let allow_withdraw = (withdrawal_exists &&
            (withdrawal_olc.index < pool.observed_lockup_cycle.index || spec_can_withdraw_pending_inactive(pool_address)));

        ensures !allow_withdraw
            ==> old(global<coin::CoinStore<AptosCoin>>(delegator_address)).coin.value == global<coin::CoinStore<AptosCoin>>(delegator_address).coin.value;
        ensures !allow_withdraw ==> old(pool) == pool;

        let pre_stake_pool = spec_get_stake(pool);
        let post stake_pool = spec_get_stake(pool);

        ensures withdrawal_exists ==> pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,
             withdrawal_olc), delegator_address) != 0;

        // TODO(Teng): Timeout
        // Property 2 [Timeout]:  pool_u64::shares_to_amount(
        // inactive_shares[withdrawal_olc], // snapshot of shares pool before redeem_shares
        // pool_u64::shares(old(inactive_shares[withdrawal_olc]), delegator) -
        // pool_u64::shares(inactive_shares[withdrawal_olc], delegator)
        // ) == amount (its latest value): delegator redeemed shares worth the amount withdrawn
        // Proved in redeem_inactive_shares

        // case 1:
        ensures (allow_withdraw && amount > 0 && pool_u64::spec_shares(old(table::spec_get(pool.inactive_shares, withdrawal_olc)), delegator_address) ==
            pool_u64::spec_shares(table::spec_get(pool.inactive_shares, withdrawal_olc), delegator_address)) ==>
            ghost_amount == 0;

        // case 2:
        // ensures (allow_withdraw && amount > 0 &&
        //     pool_u64::spec_shares(old(table::spec_get(pool.inactive_shares, withdrawal_olc)), delegator_address)
        //         != pool_u64::spec_shares(table::spec_get(pool.inactive_shares, withdrawal_olc), delegator_address))
        //     ==> pool_u64::spec_shares_to_amount_with_total_coins(
        //     table::spec_get(old(pool.inactive_shares), withdrawal_olc),
        //     pool_u64::spec_shares(old(table::spec_get(pool.inactive_shares, withdrawal_olc)), delegator_address)
        //         - pool_u64::spec_shares(table::spec_get(pool.inactive_shares, withdrawal_olc), delegator_address),
        //     table::spec_get(old(pool.inactive_shares), withdrawal_olc).total_coins
        // ) == ghost_amount;

        // Property 3 [OK]: delegator-balance == old(delegator-balance) + amount (its latest value): delegator
        // gained `amount` APT

        ensures (allow_withdraw && amount > 0) ==> global<coin::CoinStore<AptosCoin>>(delegator_address).coin.value
           == old(global<coin::CoinStore<AptosCoin>>(delegator_address)).coin.value + ghost_amount;

        // Property 4 [OK]: total_coins(pool.inactive_shares[withdrawal_olc]) -
        // total_coins(old(pool).inactive_shares[withdrawal_olc]) == amount (its latest value)

        ensures (allow_withdraw && amount > 0) ==>
            table::spec_get(old(pool).inactive_shares, withdrawal_olc).total_coins - table::spec_get(pool.inactive_shares, withdrawal_olc).total_coins == ghost_amount;

        // TODO(Teng):
        // Property 5 [ISSUE]: resource-account-balance == old(resource-account-balance): no stake is lost when
        // passing through the resource account
        // proved with dependency on Withdraw_Requirements, which fails in synchronize_delegation_pool
        // ensures old(global<coin::CoinStore<AptosCoin>>(pool_address)).coin.value == global<coin::CoinStore<AptosCoin>>(pool_address).coin.value;

        // Property 6 [ERROR(Not a valid property)]: total_coins(old(pool).inactive_shares[pool.OLC]) == old(pending_inactive) on StakePool:
        // This property actually only holds when withdrawal_olc.index < pool.observed_lockup_cycle.index
        // (requires before withdraw_internal, a pre condition)pending_inactive stake and
        // corresponding shares pool should be already synced, otherwise `pending_inactive -
        // amount` could fail if withdrawing pending_inactive stake
        // invariant table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive);

        // TODO(Teng): revisit these properties once Alexendru's PR is merged
        // Property 7 [ISSUE]: !withdrawing_inactive_stake =>

        // 7.a (OK)
        // a. pending_inactive == old(pending_inactive) - amount (its latest value): no excess
        // stake is inactivated on StakePool
        ensures (amount > 0 && withdrawal_exists &&
            (withdrawal_olc.index == pool.observed_lockup_cycle.index && spec_can_withdraw_pending_inactive(pool_address)))
            ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive)
            == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).pending_inactive) - ghost_amount;

        // 7.b (OK)
        // b. inactive == old(inactive): inactive stake remains unchanged
        ensures (amount > 0 && withdrawal_exists &&
            (withdrawal_olc.index == pool.observed_lockup_cycle.index && spec_can_withdraw_pending_inactive(pool_address)))
            ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive)
            == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).inactive);

        // 7.c
        // need a precondition old(total_coins_inactive) on delegation pool == old(inactive) on StakePool?
        // c. total_coins_inactive == old(total_coins_inactive): no excess stake is inactivated
        // on DelegationPool
        // requires pool.total_coins_inactive == coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive);
        // ensures (amount > 0 && withdrawal_exists &&
        //      (withdrawal_olc.index == pool.observed_lockup_cycle.index && spec_can_withdraw_pending_inactive(pool_address))) ==>
        //      pool.total_coins_inactive == old(pool).total_coins_inactive;

        // 7.d (Timeout)
        // need clarification on the meaning of withdrawing_inactive_stake
        // d. pool_u64::balance(inactive_shares[pool.OLC], delegator) >=
        // MIN_COINS_ON_SHARES_POOL or == 0
        // let balance = pool_u64::spec_shares_to_amount_with_total_coins(
        //     table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle),
        //     pool_u64::spec_shares(table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle), delegator_address),
        //     table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins);
        //
        // ensures (amount > 0 && withdrawal_exists &&
        //     (withdrawal_olc.index == pool.observed_lockup_cycle.index && spec_can_withdraw_pending_inactive(pool_address))) ==>
        //     (balance >= MIN_COINS_ON_SHARES_POOL || balance == 0);

        // TODO(Teng): revisit these properties once Alexendru's PR is merged
        // Property 8 [ISSUE] : withdrawing_inactive_stake =>
        // 8.a[OK] pending_inactive == old(pending_inactive) pending_inactive stake remains
        // unchanged
        // ensures (amount > 0 && withdrawal_exists &&
        //     (withdrawal_olc.index < pool.observed_lockup_cycle.index || !spec_can_withdraw_pending_inactive(pool_address)))
        //     ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive)
        //     == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).pending_inactive);

        // 8.a[OK]: if we do not withdraw pending_inactive, pending_inactive on the stakepool does not change after execution
        ensures withdrawal_olc.index < old(pool).observed_lockup_cycle.index ==> pre_stake_pool.pending_inactive.value == stake_pool.pending_inactive.value;

        // 8.b[Timeout]. inactive == old(inactive) - amount (its latest value)
        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address)
        // ensures (amount > 0 && withdrawal_exists &&
        //     (withdrawal_olc.index < pool.observed_lockup_cycle.index || !spec_can_withdraw_pending_inactive(pool_address)))
        //     ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive)
        //     == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).inactive) - ghost_amount;


        // 8.c[Timeout]. total_coins_inactive == old(total_coins_inactive) - amount (its latest value):
        // inactive stake known by DelegationPool is correctly updated and does not
        // bypass any later lockup-cycle detection (if directly assigning `inactive` to it)
        // ensures (amount > 0 && withdrawal_exists &&
        //     (withdrawal_olc.index < pool.observed_lockup_cycle.index || !spec_can_withdraw_pending_inactive(pool_address)))
        // ==> pool.total_coins_inactive == old(pool).total_coins_inactive - ghost_amount;

        // 8.d[Timeout]. total_coins_inactive == inactive on StakePool
        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address)
        //     ==> pool.total_coins_inactive == coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive);

    }

    spec fun spec_pending_withdrawal_exists_first(pool: DelegationPool, delegator_address: address): bool {
        table::spec_contains(pool.pending_withdrawals, delegator_address)
    }

    spec fun spec_pending_withdrawal_exists_second(pool: DelegationPool, delegator_address: address): ObservedLockupCycle {
        if (spec_pending_withdrawal_exists_first(pool, delegator_address)) {
            table::spec_get(pool.pending_withdrawals, delegator_address)
        } else {
            ObservedLockupCycle { index: 0 }
        }
    }

    spec pending_inactive_shares_pool_mut(pool: &mut DelegationPool): &mut pool_u64::Pool {
        pragma verify = true;
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
    }

    spec amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        pragma opaque;
        pragma verify = true;
        include AmountToSharesToRedeemAbortsIf;
        ensures result == spec_amount_to_shares_to_redeem(shares_pool, shareholder, coins_amount);
        ensures coins_amount == MAX_U64 ==> result == pool_u64::spec_shares(shares_pool, shareholder);
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
        pragma verify = true;
        pragma aborts_if_is_partial;
        let shares_pool = pool.active_shares;
        let shares_to_redeem = spec_amount_to_shares_to_redeem(pool.active_shares, shareholder, coins_amount);
        let redeemed_coins = pool_u64::spec_shares_to_amount_with_total_coins(shares_pool, shares_to_redeem, shares_pool.total_coins);
        aborts_if pool_u64::spec_shares(shares_pool, shareholder) < shares_to_redeem;
        aborts_if shares_pool.total_coins < redeemed_coins;
        aborts_if shares_pool.total_shares < shares_to_redeem;
        ensures pool_u64::spec_shares(shares_pool, shareholder) == shares_to_redeem ==> !pool_u64::spec_contains(pool.active_shares, shareholder);
        ensures coins_amount == MAX_U64 ==> !pool_u64::spec_contains(pool.active_shares, shareholder);
        ensures forall addr: address: addr != shareholder ==> pool_u64::spec_shares(pool.active_shares, addr) == pool_u64::spec_shares(old(pool).active_shares, addr);
    }

    spec redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: ObservedLockupCycle,
    ): u64 {
        pragma verify = true;
        pragma aborts_if_is_partial;
        //Property 1 [OK]: pool_u64::shares(old(pool).inactive_shares[lockup_cycle], shareholder) != 0 && pool_u64::shares(pool.inactive_shares[lockup_cycle], shareholder) == 0 => !table::contains(pending_withdrawals, delegator)
        ensures pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares, lockup_cycle), shareholder) != 0 && pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder) == 0 ==> !table::spec_contains(pool.pending_withdrawals, shareholder);

        //Property 2 [OK]: total_coins(old(pool).inactive_shares[lockup_cycle]) - redeemed_coins (result) == 0 => !table::contains(pool.inactive_shares, lockup_cycle):
        ensures (lockup_cycle.index < old(pool).observed_lockup_cycle.index
            && pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) != 0
            && table::spec_get(old(pool).inactive_shares,lockup_cycle).total_coins - result == 0)
            ==> !table::spec_contains(pool.inactive_shares, lockup_cycle);

        // TODO(Teng):
        // Rely on global property 5
        // Property 3 [OK with long execution time]: table::contains(old(pending_withdrawals), delegator) && !table::contains(pending_withdrawals, delegator) => old(pending_withdrawals)[delegator] == lockup_cycle:
        // ensures (table::spec_contains(old(pool).pending_withdrawals, shareholder) && !(table::spec_contains(pool.pending_withdrawals, shareholder))) ==>
        //    (table::spec_contains(old(pool).pending_withdrawals, shareholder) && table::spec_get(old(pool).pending_withdrawals, shareholder).index == lockup_cycle.index);

        // Property 2 of withdraw_internal can be expressed and proved here.
        ensures (pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares, lockup_cycle), shareholder) != 0
            && pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder) == 0) ==>
            pool_u64::spec_shares_to_amount_with_total_coins(
                table::spec_get(old(pool.inactive_shares), lockup_cycle),
                pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares, lockup_cycle), shareholder),
                table::spec_get(old(pool.inactive_shares), lockup_cycle).total_coins
            ) == result;

        ensures pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder) !=
            pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) ==>
            ghost_share_redeem == pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) -
                pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder);

        ensures pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder) !=
            pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) ==> (ghost_share_redeem == pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) -
            pool_u64::spec_shares(table::spec_get(pool.inactive_shares,lockup_cycle), shareholder)) && (
        result == pool_u64::spec_shares_to_amount_with_total_coins(table::spec_get(old(pool.inactive_shares), lockup_cycle),
            ghost_share_redeem, table::spec_get(old(pool.inactive_shares), lockup_cycle).total_coins));

        ensures result <= table::spec_get(old(pool.inactive_shares), lockup_cycle).total_coins;

        // Same as property 4 of withdraw_internal
        // ensures table::spec_get(old(pool.inactive_shares), lockup_cycle).total_coins - table::spec_get(pool.inactive_shares, lockup_cycle).total_coins == result;
    }

    spec calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        pragma verify = true;
        pragma opaque;
        let pool_address = get_pool_address(pool);
        aborts_if pool.total_coins_inactive > global<stake::StakePool>(pool_address).inactive.value;
        let pre_stake = global<stake::StakePool>(pool_address);
        ensures global<stake::StakePool>(pool_address) == old(global<stake::StakePool>(pool_address));
        ensures result_2 == global<stake::StakePool>(pool_address).active.value
            + global<stake::StakePool>(pool_address).pending_active.value;
        ensures result_1 == (spec_get_stake(pool).inactive.value > pool.total_coins_inactive);
        ensures result_1 <==> pool.total_coins_inactive < global<stake::StakePool>(pool_address).inactive.value;
        ensures result_1 ==> result_3 == pre_stake.inactive.value - pool.total_coins_inactive;
        ensures !result_1 <==> pool.total_coins_inactive == global<stake::StakePool>(pool_address).inactive.value;
        ensures !result_1 ==> result_3 == pre_stake.pending_inactive.value;
    }

    spec fun spec_get_stake(pool: DelegationPool): stake::StakePool {
        let pool_address = pool.stake_pool_signer_cap.account;
        global<stake::StakePool>(pool_address)
    }

    spec synchronize_delegation_pool(pool_address: address) {

        pragma verify = false;
        pragma aborts_if_is_strict = false;

        include PoolDelegatorAddressNotEq {
            delegator_address: global<stake::StakePool>(pool_address).operator_address
        };
        aborts_if !exists<DelegationPool>(pool_address);

        let pre_pool = global<DelegationPool>(pool_address);
        let pre_stake_pool = spec_get_stake(pre_pool);
        let pre_stake_active_value = pre_stake_pool.active.value;
        let pre_stake_pending_active_value = pre_stake_pool.pending_active.value;
        let pre_stake_pending_inactive_value = pre_stake_pool.pending_inactive.value;

        let post pool = global<DelegationPool>(pool_address);
        let post stake_pool = spec_get_stake(pool);//global<stake::StakePool>(pool_address);
        let post stake_pending_inactive_value = stake_pool.pending_inactive.value;

        let pre_inactive = pre_stake_pool.inactive.value;
        let lockup_cycle_ended = pre_inactive > pre_pool.total_coins_inactive;

        // Property 1 [OK]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures pool.active_shares.total_coins == pre_stake_active_value + pre_stake_pending_active_value;

        // Property 2 [OK]:. total_coins(pool.inactive_shares[pool.OLC]) == pending_inactive on StakePool
        ensures !lockup_cycle_ended ==> table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == pre_stake_pending_inactive_value;
        ensures lockup_cycle_ended ==> (table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == 0);

        // Property 3 [OK]: pool.total_coins_inactive == inactive on StakePool
        ensures pool.total_coins_inactive == stake_pool.inactive.value;

        // Property 4 [OK]: inactive > old(total_coins_inactive) IFF pool.OLC == old(pool).OLC + 1:
        ensures pre_inactive > pre_pool.total_coins_inactive <==> pre_pool.observed_lockup_cycle.index + 1 == pool.observed_lockup_cycle.index;

        // Property 5.1 [OK]: pool.OLC == old(pool).OLC + 1 => table::contains(pool.inactive_shares, pool.OLC)
        ensures (pre_pool.observed_lockup_cycle.index + 1 == pool.observed_lockup_cycle.index) ==> table::spec_contains(pool.inactive_shares, pool.observed_lockup_cycle);

        // Property 5.2 [OK]: pool.OLC == old(pool).OLC + 1 => total_coins(pool.inactive_shares[pool.OLC]) == 0
        ensures (pre_pool.observed_lockup_cycle.index + 1 == pool.observed_lockup_cycle.index) ==> table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == 0;
        //ensures lockup_cycle_ended ==> table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == 0;

        // Property 5.3 [OK]: pool.OLC == old(pool).OLC + 1 => total_coins(pool.inactive_shares[old(pool).OLC]) == inactive - old(total_coins_inactive)
        ensures (pre_pool.observed_lockup_cycle.index + 1 == pool.observed_lockup_cycle.index)
              ==> table::spec_get(pool.inactive_shares, pre_pool.observed_lockup_cycle).total_coins == pre_inactive - pre_pool.total_coins_inactive;

        // Property 6 [OK]: add_stake fees charged during previous epoch have been refunded
        ensures pre_stake_pending_active_value == 0 ==> !pool_u64::spec_contains(pool.active_shares, NULL_SHAREHOLDER);

        // Property 7 [OK]: pending_active != 0 => pool_u64::shares(pool.active_shares, NULL_SHAREHOLDER) == pool_u64::shares(old(pool).active_shares, NULL_SHAREHOLDER): any add_stake fees are not refunded unless the epoch when charged has ended
        ensures pre_stake_pending_active_value != 0 ==> pool_u64::spec_shares(pool.active_shares, NULL_SHAREHOLDER) == pool_u64::spec_shares(pre_pool.active_shares, NULL_SHAREHOLDER);

        // TODO(Teng): long execution time
        // Property 8 [OK]: let commission_active = delta total_coins(pool.active_shares) * pool.commission;
        // pool_u64::shares(pool.active_shares, operator) - pool_u64::shares(old(pool).active_shares, operator) ==
        // pool_u64::amount_to_shares(
        // pool.active_shares, // snapshot of shares pool before buy_in
        // commission_active
        // ) operator gained pool-rewards * pool-commission% additional to its existing stake + its rewards produced in the meantime as a regular delegator

        ensures pool_u64::spec_shares(sync_delegation_pool_snap_before_buy_in.active_shares, global<stake::StakePool>(pool_address).operator_address)
            + pool_u64::spec_amount_to_shares_with_total_coins(sync_delegation_pool_snap_before_buy_in.active_shares, sync_commission_active,
            sync_delegation_pool_snap_before_buy_in.active_shares.total_coins)
            == pool_u64::spec_shares(pool.active_shares, global<stake::StakePool>(pool_address).operator_address);

        ensures pool_u64::spec_shares(sync_inactive_snap_before_buy_in, global<stake::StakePool>(pool_address).operator_address)
            + pool_u64::spec_amount_to_shares_with_total_coins(sync_inactive_snap_before_buy_in, sync_commission_pending_inactive,
            sync_inactive_snap_before_buy_in.total_coins) ==
            pool_u64::spec_shares(table::spec_get(pool.inactive_shares, pre_pool.observed_lockup_cycle), global<stake::StakePool>(pool_address).operator_address);

        // Property 9 [OK]:
        // forall delegators: delegator != operator && != NULL_SHAREHOLDER => pool_u64::shares(pool.active_shares, delegator) == pool_u64::shares(old(pool).active_shares, delegator)
        ensures forall delegator: address: (delegator != global<stake::StakePool>(pool_address).operator_address && delegator != NULL_SHAREHOLDER) ==> (pool_u64::spec_shares(pool.active_shares, delegator) == pool_u64::spec_shares(pre_pool.active_shares, delegator));

        // Property 10 [OK]:
        // pending_inactive on stakepool does not change after synchronization
        ensures stake_pending_inactive_value == pre_stake_pending_inactive_value;

    }

}
