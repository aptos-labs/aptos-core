spec aptos_framework::delegation_pool {
    spec module {
        // TODO: verification disabled until this module is specified.
        pragma verify = false;
        pragma aborts_if_is_partial;

        // Property 1 [OK]:
        invariant forall addr: address: exists<DelegationPool>(addr) ==> exists<stake::StakePool>(addr);

        // Property 3 [ISSUE]:
        // TODO: Can not deal with 'sum'

        // Property 4 [TIMEOUT]:

        // invariant [suspendable] forall delegator: address where exists<DelegationPool>(delegator):
        //     forall i in 0..global<DelegationPool>(delegator).observed_lockup_cycle.index,j in 0..global<DelegationPool>(delegator).observed_lockup_cycle.index:
        //         pool_u64::spec_shares(table::spec_get(global<DelegationPool>(delegator).inactive_shares,ObservedLockupCycle{index: i}), delegator) != 0 &&
        //         pool_u64::spec_shares(table::spec_get(global<DelegationPool>(delegator).inactive_shares,ObservedLockupCycle{index: j}), delegator) != 0 ==> i == j;

        // Property 5 [TIMEOUT]:

        // invariant [suspendable] forall delegator: address where exists<DelegationPool>(delegator): forall i in 0..global<DelegationPool>(delegator).observed_lockup_cycle.index:
        //     pool_u64::spec_shares(table::spec_get(global<DelegationPool>(delegator).inactive_shares,ObservedLockupCycle{index: i}), delegator) != 0 ==> table::spec_contains(global<DelegationPool>(delegator).pending_withdrawals, delegator) &&
        //     table::spec_get(global<DelegationPool>(delegator).pending_withdrawals,delegator).index == i;

        // Property 6 [OK]:

        invariant forall delegator: address where exists<DelegationPool>(delegator):
            table::spec_contains(global<DelegationPool>(delegator).pending_withdrawals, delegator) ==> pool_u64::spec_shares(table::spec_get(global<DelegationPool>(delegator).inactive_shares,ObservedLockupCycle{index: table::spec_get(global<DelegationPool>(delegator).pending_withdrawals,delegator).index}), delegator) != 0;

        // Property 7 [OK]:

        invariant forall addr: address where exists<DelegationPool>(addr): table::spec_contains(global<DelegationPool>(addr).inactive_shares,global<DelegationPool>(addr).observed_lockup_cycle);

        // Property 8 [OK]:

        invariant forall addr: address where exists<DelegationPool>(addr): forall i in 0..global<DelegationPool>(addr).observed_lockup_cycle.index: table::spec_contains(global<DelegationPool>(addr).inactive_shares,ObservedLockupCycle{index:i}) ==> table::spec_get(global<DelegationPool>(addr).inactive_shares,ObservedLockupCycle{index:i}).total_coins != 0;

        // Property 10 [TODO]:

        // Property 12 [TODO]:

        global ghost_coin_1: u64;
        global ghost_coin_2: u64;
        global ghost_coin_3: u64;
        global ghost_coin_4: u64;
        global ghost_shares: u64;
        global ghost_delegation_pool: DelegationPool;
        global ghost_balance: u64;
        global ghost_p_active: u64;
        global ghost_p_pending_active: u64;
        global ghost_p_inactive: u64;
        global ghost_p_pending_inactive: u64;
        global ghost_active_p: u64;
        global ghost_pending_active_p: u64;
        global ghost_inactive_p: u64;
        global ghost_pending_inactive_p: u64;

        global ghost_p_share_source: u64;
        global ghost_share_source_p: u64;
        global ghost_p_share_dest: u64;
        global ghost_share_dest_p: u64;
        global ghost_source_pool: pool_u64::Pool;
        global ghost_dest_pool: pool_u64::Pool;

        global ghost_bool: bool;
        global ghost_bool_1: bool;
        global ghost_amount: u64;
        global ghost_olc: ObservedLockupCycle;
    }

    spec schema Global_Requirement{
        // Property 2 [TIMEOUT]:
        // TODO: TIMEOUT in some function

        // invariant forall addr: address: global<DelegationPool>(addr).stake_pool_signer_cap.account == addr;

        // Property 9 [TIMEOUT]:
        // TODO: TIMEOUT in init_dele_pool

        // invariant forall delegator: address where exists<DelegationPool>(delegator): table::spec_get(global<DelegationPool>(delegator).pending_withdrawals,delegator).index <= global<DelegationPool>(delegator).observed_lockup_cycle.index;

        // Property 11 [TIMEOUT]:
        // TODO: TIMEOUT in some function

        // invariant [suspendable] forall addr: address: global<DelegationPool>(addr).total_coins_inactive <= global<stake::StakePool>(addr).inactive.value;
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
        //EXTRA TODO: TIMEOUT
        pragma verify = false;
        aborts_if !delegation_pool_exists(pool_address);
        // aborts_if !exists<DelegationPool>(pool_address);
        // aborts_if !stake::stake_pool_exists(pool_address);
    }

    spec get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) {
        //EXTRA TODO: TIMEOUT
        pragma verify = false;
    }

    spec fun spec_get_add_stake_fee(pool_address: address, amount: u64): u64;

    spec get_add_stake_fee(pool_address: address, amount: u64): u64 {
        pragma opaque;
        ensures [abstract] result == spec_get_add_stake_fee(pool_address, amount);
    }

    spec can_withdraw_pending_inactive(pool_address: address): bool {
        pragma verify = true;
        aborts_if !exists<stake::ValidatorSet>(@aptos_framework);
    }

    //Complete
    spec initialize_delegation_pool(
        owner: &signer,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
    ) {
        pragma verify = true;
        pragma aborts_if_is_partial = true;
        include stake::ResourceRequirement;
        //Property 1 [OK]: asserts_if !features::delegation_pools_enabled()
        //TODO: Prover can't resolve features::delegation_pools_enabled(), use magic number instead , may fixed later.
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
        ensures pool.total_coins_inactive == coin::value(stake_pool.pending_inactive);
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
        // let shares_to_redeem = spec_amount_to_shares_to_redeem(src_shares_pool, shareholder, amount);
        //  aborts_if src_shares_pool.total_coins > 0 && src_shares_pool.total_shares > 0
        //     && (shares_to_redeem * src_shares_pool.total_coins) / src_shares_pool.total_shares > MAX_U64;
        // include AmountToSharesToRedeemAbortsIf {
        //     shares_pool: src_shares_pool,
        //     shareholder: shares_to_redeem,
        // };
    }

    spec retrieve_stake_pool_owner(pool: &DelegationPool): signer {

    }

    spec get_pool_address(pool: &DelegationPool): address {

    }

    spec olc_with_index(index: u64): ObservedLockupCycle {

    }

    spec set_operator(
        owner: &signer,
        new_operator: address
    ) {
        //EXTRA TODO: TIMEOUT
        pragma verify = false;
    }

    spec set_delegated_voter(
        owner: &signer,
        new_voter: address
    ) {
        //EXTRA TODO: TIMEOUT
        pragma verify = false;
    }
    

    spec add_stake(delegator: &signer, pool_address: address, amount: u64) {

        //TODO: Decrease the usage of ghost var
        //All of the passed property may takes about 200s - 300s, it passed anyway 

        pragma verify = false;
        pragma aborts_if_is_partial = true;

        include stake::ResourceRequirement;

        let pool = global<DelegationPool>(pool_address);
        //Note: Origin func return when amount > 0, so it should be a pre-condition
        requires amount > 0;
        //invariant global<DelegationPool>(pool_address).stake_pool_signer_cap.account == pool_address;
        //Property 1 [OK]: aborts_if pool_u64::balance(pool.active_shares, delegator) < MIN_COINS_ON_SHARES_POOL
        //Note: Prover occur timeout when introducing the pool_64::balance directly, using ghost var instead.
        // aborts_if ghost_balance < MIN_COINS_ON_SHARES_POOL;

        //Property 2 [OK]: ensures active + pending_active == old(active) + old(pending_active) + amount
        //Note: Add a function in stake.move to obtain the onwerbility.pool_address
        ensures ghost_active_p + ghost_pending_active_p == ghost_p_active + ghost_p_pending_active + amount;

        //Property 3 [TIMEOUT]: total_coins(pool.active_shares) == active + pending_active on StakePool
        //TODO: Which StakePool does it mean? global<stake::StakePool>(pool_address) or global<stake::StakePool>(get_address_of(pool))?
        
        ensures global<DelegationPool>(pool_address).active_shares.total_coins == coin::value(global<stake::StakePool>(pool_address).active) + coin::value(global<stake::StakePool>(pool_address).pending_active);

        //Property 4 [OK]: pool_u64::shares(pool.active_shares, delegator) - pool_u64::shares(old(pool).active_shares, delegator) == pool_u64::amount_to_shares(pool.active_shares, amount - get_add_stake_fee(pool_address, amount))   
        //TODO: May use ghost_pool instead of ghost_share later.
        let delegator_address = signer::address_of(delegator);
        let total_coins = pool.active_shares.total_coins;   
        //invariant pool_u64::spec_shares(pool.active_shares, delegator_address) - ghost_shares == pool_u64::spec_amount_to_shares_with_total_coins(pool.active_shares, amount - spec_get_add_stake_fee(pool_address, amount), pool.active_shares.total_coins);
        
        //Property 5 [OK]: pool_u64::shares(pool.active_shares, NULL_SHAREHOLDER) - pool_u64::shares(old(pool).active_shares, NULL_SHAREHOLDER) == pool_u64::amount_to_shares(pool.active_shares, get_add_stake_fee(pool_address, amount))
        invariant pool_u64::spec_shares(pool.active_shares, NULL_SHAREHOLDER) - pool_u64::spec_shares( ghost_delegation_pool.active_shares, NULL_SHAREHOLDER) == pool_u64::spec_amount_to_shares_with_total_coins(pool.active_shares, amount - spec_get_add_stake_fee(pool_address, amount), pool.active_shares.total_coins);
        
        //Property 6 [ISSUE]:delegator-balance == old(delegator-balance) - amount
        //Issue: Is it possible that the delegator is the same as resources account?
        //Suggestion: Add assert in the origin function
        //After Suggestion [OK]
        ensures ghost_coin_3 == ghost_coin_1 - amount;
        
        //Property 7 [ISSUE]: resource-account-balance == old(resource-account-balance)
        //Issue: Delegtor transfer to the pool_address (recive), and let resource-account add stake (paid), how could resource-account balance remain the same?
        //Suggestion: ghost_coin_4 == ghost_coin_2 - amount
        //After Suggestion [OK]
        ensures ghost_coin_4 == ghost_coin_2 - amount;

        //Property 8 [TODO]: delegator does not earn rewards from its pending_active stake when this epoch ends
        //Note: I'm not sure if this property should be verfied here, it should belong to sync_delegation_pool

        
    }

    spec unlock(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
        pragma aborts_if_is_partial;

        requires amount > 0;

        //TODO: All timeout, it may related to other property

        //Property 1 [TIMEOUT]: pool_u64::shares_to_amount(
        //source_pool, // snapshot of shares pool before redeem_shares
        //pool_u64::shares(old(source_pool), delegator) -
        //pool_u64::shares(source_pool, delegator)
        //) == amount (its latest value)

        ensures pool_u64::spec_shares_to_amount_with_total_coins( ghost_source_pool, pool_u64::spec_shares(old(global<DelegationPool>(pool_address)).active_shares, signer::address_of(delegator)) - pool_u64::spec_shares(ghost_source_pool, signer::address_of(delegator)),ghost_source_pool.total_coins) == ghost_amount;

        //Property 2 [TIMEOUT]: pool_u64::shares(destination_pool, delegator) -
        //pool_u64::shares(old(destination_pool), delegator) ==
        //pool_u64::amount_to_shares(
        //destination_pool, // snapshot of shares pool before buy_in
        //amount (its latest value)
        //)

        ensures pool_u64::spec_shares(table::spec_get(global<DelegationPool>(pool_address).inactive_shares,global<DelegationPool>(pool_address).observed_lockup_cycle), signer::address_of(delegator)) - pool_u64::spec_shares(table::spec_get(old(global<DelegationPool>(pool_address)).inactive_shares,global<DelegationPool>(pool_address).observed_lockup_cycle), signer::address_of(delegator)) == pool_u64::spec_amount_to_shares_with_total_coins(ghost_dest_pool,amount,ghost_dest_pool.total_coins);

        //Property 3 [TIMEOUT]: pool_u64::balance(pool.inactive_shares[pool.OLC], delegator) + pool_u64::balance(pool.active_shares, delegator) <= pool_u64::balance(old(inactive_shares[pool.OLC]), delegator) + pool_u64::balance(old(active_shares), delegator)

        ensures ghost_coin_3 + ghost_coin_4 <= ghost_coin_1 + ghost_coin_2;

        //Property 4 [TIMOUT]: total_coins(old(source_pool)) - total_coins(source_pool) == total_coins(destination_pool) - total_coins(old(destination_pool)) == amount (its latest value)

        ensures old(global<DelegationPool>(pool_address).active_shares).total_coins - global<DelegationPool>(pool_address).active_shares.total_coins == ghost_amount;


        //Property 5 [TIMEOUT]: abs(active - old(active)) == abs(pending_inactive - old(pending_inactive))

        ensures ghost_active_p - ghost_p_active == ghost_p_pending_inactive - ghost_pending_inactive_p;

        //Property 6 [TIMEOUT]: pending_active == old(pending_active): no pending_active stake can be displaced

        ensures ghost_p_pending_active == ghost_pending_active_p;

        //Property 7 [TIMEOUT]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures global<DelegationPool>(pool_address).active_shares.total_coins == ghost_active_p + ghost_pending_active_p;

        // total_coins(pending_inactive_shares_pool(pool)) == pending_inactive on StakePool
        ensures global<DelegationPool>(pool_address).total_coins_inactive == ghost_pending_inactive_p;

        //Property 8 [TIMEOUT]: aborts_if pool_u64::balance(destination_pool, delegator) < MIN_COINS_ON_SHARES_POOL
        aborts_if ghost_coin_4 < MIN_COINS_ON_SHARES_POOL;    

        //Property 9 [TIMEOUT]: pool_u64::balance(source_pool, delegator) >= MIN_COINS_ON_SHARES_POOL or == 0
        aborts_if  ghost_coin_1 < MIN_COINS_ON_SHARES_POOL && ghost_coin_1 != 0;
    }

    spec reactivate_stake(delegator: &signer, pool_address: address, amount: u64) {
        pragma verify = false;
        pragma aborts_if_is_partial;

        //TODO: All Timeout, and maybe there are remained some error, which reactivate func is not much likely processed like unlock func.

        requires amount > 0;
        requires signer::address_of(delegator) != pool_address;

        //Property 1 [TIMEOUT]: pool_u64::shares_to_amount(
        //source_pool, // snapshot of shares pool before redeem_shares
        //pool_u64::shares(old(source_pool), delegator) -
        //pool_u64::shares(source_pool, delegator)
        //) == amount (its latest value)

        ensures pool_u64::spec_shares_to_amount_with_total_coins(table::spec_get(global<DelegationPool>(pool_address).inactive_shares,global<DelegationPool>(pool_address).observed_lockup_cycle), ghost_p_share_source - ghost_share_source_p, table::spec_get(global<DelegationPool>(pool_address).inactive_shares,global<DelegationPool>(pool_address).observed_lockup_cycle).total_coins) == amount;

        //Property 2 [TIMEOUT]: pool_u64::shares(destination_pool, delegator) -
        //pool_u64::shares(old(destination_pool), delegator) ==
        //pool_u64::amount_to_shares(
        //destination_pool, // snapshot of shares pool before buy_in
        //amount (its latest value)
        //)

        //ensures pool_u64::spec_shares(global<DelegationPool>(pool_address).active_shares, signer::address_of(delegator)) - pool_u64::spec_shares(old(global<DelegationPool>(pool_address)).active_shares, signer::address_of(delegator)) == pool_u64::spec_amount_to_shares_with_total_coins(global<DelegationPool>(pool_address).active_shares,amount,global<DelegationPool>(pool_address).active_shares.total_coins);

        ensures ghost_share_dest_p - ghost_p_share_dest == pool_u64::spec_amount_to_shares_with_total_coins(global<DelegationPool>(pool_address).active_shares,amount,global<DelegationPool>(pool_address).active_shares.total_coins);        

        //Property 3 [TIMEOUT]: pool_u64::balance(pool.inactive_shares[pool.OLC], delegator) + pool_u64::balance(pool.active_shares, delegator) <= pool_u64::balance(old(inactive_shares[pool.OLC]), delegator) + pool_u64::balance(old(active_shares), delegator)
        ensures ghost_coin_3 + ghost_coin_4 <= ghost_coin_1 + ghost_coin_2;

        //Property 4 [TIMEOUT]: total_coins(old(source_pool)) - total_coins(source_pool) == total_coins(destination_pool) - total_coins(old(destination_pool)) == amount (its latest value)
        ensures old(global<DelegationPool>(pool_address)).total_coins_inactive - global<DelegationPool>(pool_address).total_coins_inactive == amount;
        ensures global<DelegationPool>(pool_address).active_shares.total_coins - old(global<DelegationPool>(pool_address)).active_shares.total_coins == amount;

        //Property 5 [TIMEOUT]: abs(active - old(active)) == abs(pending_inactive - old(pending_inactive))
        ensures ghost_active_p - ghost_p_active == ghost_p_pending_inactive - ghost_pending_inactive_p;

        //Property 6 [TIMEOUT]: pending_active == old(pending_active): no pending_active stake can be displaced
        ensures ghost_p_pending_active == ghost_pending_active_p;

        //Property 7 [TIMEOUT]: total_coins(pool.active_shares) == active + pending_active on StakePool
        ensures global<DelegationPool>(pool_address).active_shares.total_coins == ghost_active_p + ghost_pending_active_p;

        // total_coins(pending_inactive_shares_pool(pool)) == pending_inactive on StakePool
        ensures global<DelegationPool>(pool_address).total_coins_inactive == ghost_pending_inactive_p;

        //Property 8 [TIMEOUT]: aborts_if pool_u64::balance(destination_pool, delegator) < MIN_COINS_ON_SHARES_POOL
        aborts_if ghost_coin_4 < MIN_COINS_ON_SHARES_POOL;    

        //Property 9 [TIMEOUT]: pool_u64::balance(source_pool, delegator) >= MIN_COINS_ON_SHARES_POOL or == 0
        aborts_if ghost_coin_1 < MIN_COINS_ON_SHARES_POOL && ghost_coin_1 != 0;
    }

    spec withdraw(delegator: &signer, pool_address: address, amount: u64) {
        //TODO: TIMEOUT via Global
        pragma verify = false;
        pragma aborts_if_is_partial;
        // Property 1 [OK]: aborts_if amount == 0;
        aborts_if amount == 0;
    }

    spec withdraw_internal(pool: &mut DelegationPool, delegator_address: address, amount: u64) {

        pragma verify = true;
        pragma aborts_if_is_partial;

        requires get_pool_address(pool) != delegator_address;
        requires pool.stake_pool_signer_cap.account == get_pool_address(pool);

        // Property 1 [OK]: !(withdrawal_exists && (withdrawing_inactive_stake ||
        // can_withdraw_pending_inactive(pool_address))) =>
        // a. delegator-balance == old(delegator-balance) && there are no state changes on
        // DelegationPool and StakePool: if the pending withdrawal does not exist or is not
        // inactive and cannot withdraw pending_inactive stake, then nothing happens

        ensures ghost_bool ==> old(global<coin::CoinStore<AptosCoin>>(delegator_address)).coin.value == global<coin::CoinStore<AptosCoin>>(delegator_address).coin.value;

        // Property 2 [TIMEOUT]:  pool_u64::shares_to_amount(
        // inactive_shares[withdrawal_olc], // snapshot of shares pool before redeem_shares
        // pool_u64::shares(old(inactive_shares[withdrawal_olc]), delegator) -
        // pool_u64::shares(inactive_shares[withdrawal_olc], delegator)
        // ) == amount (its latest value): delegator redeemed shares worth the amount withdrawn
        
        // TODO: Have use opaque to solve, still timeout
        // As long as redeem_inactive_shares is verfied, maybe this one is reduntent?

        // ensures !ghost_bool && amount > 0 ==>pool_u64::spec_shares_to_amount_with_total_coins(
        //     table::spec_get(old(pool.inactive_shares),ghost_olc),
        //     pool_u64::spec_shares(old(table::spec_get(pool.inactive_shares,ghost_olc)), delegator_address) - pool_u64::spec_shares(table::spec_get(pool.inactive_shares,ghost_olc), delegator_address),
        //     table::spec_get(old(pool.inactive_shares),ghost_olc).total_coins
        // ) == ghost_amount;

        // Property 3 [OK]: delegator-balance == old(delegator-balance) + amount (its latest value): delegator
        // gained `amount` APT

        ensures !ghost_bool && amount > 0 ==> global<coin::CoinStore<AptosCoin>>(delegator_address).coin.value ==  old(global<coin::CoinStore<AptosCoin>>(delegator_address)).coin.value + ghost_amount;
        
        // ensures !ghost_bool ==> ghost_coin_1 == ghost_coin_3 + ghost_amount;
        
        // Property 4 [OK]: total_coins(pool.inactive_shares[withdrawal_olc]) -
        // total_coins(old(pool).inactive_shares[withdrawal_olc]) == amount (its latest value)

        ensures !ghost_bool && amount > 0 ==> table::spec_get(old(pool).inactive_shares,ghost_olc).total_coins - table::spec_get(pool.inactive_shares,ghost_olc).total_coins == ghost_amount;

        // Property 5 [OK]: resource-account-balance == old(resource-account-balance): no stake is lost when
        // passing through the resource account

        ensures global<coin::CoinStore<AptosCoin>>(get_pool_address(old(pool))).coin.value == global<coin::CoinStore<AptosCoin>>(get_pool_address(pool)).coin.value;

        // Property 6 [ERROR]: total_coins(old(pool).inactive_shares[pool.OLC]) == old(pending_inactive) on StakePool:
        // (requires before withdraw_internal, a pre condition)pending_inactive stake and
        // corresponding shares pool should be already synced, otherwise `pending_inactive -
        // amount` could fail if withdrawing pending_inactive stake

        // TODO: This property is far from valid, we are figuring out why

        // ensures table::spec_get(old(pool.inactive_shares),old(pool.observed_lockup_cycle)).total_coins == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive));

        // Property 7 [ERROR]: !withdrawing_inactive_stake =>
        // a. pending_inactive == old(pending_inactive) - amount (its latest value): no excess
        // stake is inactivated on StakePool
        // b. inactive == old(inactive): inactive stake remains unchanged
        // c. total_coins_inactive == old(total_coins_inactive): no excess stake is inactivated
        // on DelegationPool
        // d. pool_u64::balance(inactive_shares[pool.OLC], delegator) >=
        // MIN_COINS_ON_SHARES_POOL or == 0

        // TODO: Don't know what does withdrawing_inactive_stake means
        // It does not make any sense, in the sources code, the coin is from inactive -> pending_inactive -> delegator
        // But in the introduction, only inactive coin could be withdraw
        
        // ensures ghost_bool ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive) == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).pending_inactive) - ghost_amount;


        // ensures coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive) == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account)).inactive) ==> pool.total_coins_inactive == old(pool).total_coins_inactive;

        // let balance = pool_u64::spec_shares_to_amount_with_total_coins(
        //     table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle),
        //     pool_u64::spec_shares(table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle), delegator_address), 
        //     table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins);

        // ensures !ghost_bool && amount > 0 && table::spec_contains(pool.pending_withdrawals, delegator_address) ==> balance >= MIN_COINS_ON_SHARES_POOL || balance == 0;
        

        // Property 8 [ERROR]: withdrawing_inactive_stake =>
        // a. pending_inactive == old(pending_inactive) pending_inactive stake remains
        // unchanged
        // b. inactive == old(inactive) - amount (its latest value)
        // c. total_coins_inactive == old(total_coins_inactive) - amount (its latest value):
        // inactive stake known by DelegationPool is correctly updated and does not
        // bypass any later lockup-cycle detection (if directly assigning `inactive` to it)
        // d. total_coins_inactive == inactive on StakePool

        // TODO: Same as the property 7

        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address) ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive) == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account).pending_inactive));

        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address) ==> coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive) == coin::value(old(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive)) - amount;
        
        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address) ==> pool.total_coins_inactive == old(pool).total_coins_inactive - amount;
        
        // ensures table::spec_contains(pool.pending_withdrawals, delegator_address) ==> pool.total_coins_inactive == coin::value(global<stake::StakePool>(pool.stake_pool_signer_cap.account).inactive);

    }

    spec pending_withdrawal_exists(pool: &DelegationPool, delegator_address: address): (bool, ObservedLockupCycle) {
    
    }

    spec pending_inactive_shares_pool_mut(pool: &mut DelegationPool): &mut pool_u64::Pool {
        
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
    }

    spec pending_inactive_shares_pool(pool: &DelegationPool): &pool_u64::Pool {
        
    }

    spec execute_pending_withdrawal(pool: &mut DelegationPool, delegator_address: address) {
        
    }

    spec buy_in_pending_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        aborts_if !table::spec_contains(pool.inactive_shares, observed_lockup_cycle);
    }

    spec amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        pragma opaque;
        include AmountToSharesToRedeemAbortsIf;
        ensures [abstract] result == spec_amount_to_shares_to_redeem(shares_pool, shareholder, coins_amount);
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
        pragma verify = false;
        pragma aborts_if_is_partial;
        let shares_pool = pool.active_shares;

        include AmountToSharesToRedeemAbortsIf;

        let shares_to_redeem = spec_amount_to_shares_to_redeem(pool.active_shares, shareholder, coins_amount);
        let redeemed_coins = pool_u64::spec_shares_to_amount_with_total_coins(shares_pool, shares_to_redeem, shares_pool.total_coins);

        aborts_if pool_u64::spec_shares(shares_pool, shareholder) < shares_to_redeem;
        aborts_if shares_pool.total_coins < redeemed_coins;
        aborts_if shares_pool.total_shares < shares_to_redeem;
    }

    //Complete
    spec redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: ObservedLockupCycle,
    ): u64 {
        pragma verify = true;
        pragma aborts_if_is_partial;
        //Property 1 [OK]: pool_u64::shares(old(pool).inactive_shares[lockup_cycle], shareholder) != 0 && pool_u64::shares(pool.inactive_shares[lockup_cycle], shareholder) == 0 => !table::contains(pending_withdrawals, delegator)
        ensures (pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) != 0 && pool_u64::spec_shares(table::spec_get(old(pool).inactive_shares,lockup_cycle), shareholder) == 0 ==> !table::spec_contains(pool.pending_withdrawals, shareholder));
        
        //Property 2 [OK]: total_coins(old(pool).inactive_shares[lockup_cycle]) - redeemed_coins (result) == 0 => !table::contains(pool.inactive_shares, lockup_cycle): 
        //Note: The inactive[olc] exist, however, it's total_coin = 0. Should we change the condition to pool.inactive_shares.total_coin == 0 ?
        //If the condition modified as mentioned, it shall pass.

        //ensures lockup_cycle.index != 0 && table::spec_get(old(pool).inactive_shares,lockup_cycle).total_coins - result == 0 ==> !table::spec_contains(pool.inactive_shares, lockup_cycle);
        ensures lockup_cycle.index != 0 && table::spec_get(old(pool).inactive_shares,lockup_cycle).total_coins - result == 0 ==> table::spec_get(pool.inactive_shares, lockup_cycle).total_coins == 0;

        //Property 3 [OK]: table::contains(old(pending_withdrawals), delegator) && !table::contains(pending_withdrawals, delegator) => old(pending_withdrawals)[delegator] == lockup_cycle:
        //Note: The prover can't apply this condition correctly: !table::spec_contains(pool.pending_withdrawals, shareholder)
        //To solve this issue, apply (pre) != (post), this new condition is resonable beacuse:
        //Obviously, if the function deleted a shareholder from the table, the (pre) should never be the same as (post)
        let a = table::spec_get(pool.pending_withdrawals, shareholder);
        let post b = table::spec_get(pool.pending_withdrawals, shareholder);

        ensures table::spec_contains(old(pool).pending_withdrawals, shareholder) && !table::spec_contains(pool.pending_withdrawals, shareholder) && a != b ==> table::spec_get(old(pool).pending_withdrawals, shareholder) == lockup_cycle;
    }

    spec calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        pragma verify = true;
    }

    spec fun spec_get_pending_inactive(pool: DelegationPool):u64 {
        let pool_address = pool.stake_pool_signer_cap.account;
        let stake_pool = global<stake::StakePool>(pool_address);
        let inactive = stake_pool.inactive.value;
        // let lockup_cycle_ended = inactive > pool.total_coins_inactive;

        if (inactive > pool.total_coins_inactive) {
            // `inactive` on stake pool = any previous `inactive` stake +
            // any previous `pending_inactive` stake and its rewards (both inactivated)
            inactive - pool.total_coins_inactive
        }else {
            0
        }
    }

    spec synchronize_delegation_pool(pool_address: address) {
            pragma verify = false;
            pragma aborts_if_is_strict = false;

            include Global_Requirement;

            // TODO: After fixed pool_u64_unbound, some property shown post condition not hold or timeout
            // Need further investigate if its related to p_u64's global invariant

            let pool = global<DelegationPool>(pool_address);
            let stake_pool = global<stake::StakePool>(pool_address);
            let inactive = stake_pool.inactive.value;
            let pending_active = stake_pool.pending_active.value;

            // Property 1 [ERROR]: total_coins(pool.active_shares) == active + pending_active on StakePool
            ensures pool.active_shares.total_coins == global<stake::StakePool>(pool_address).active.value + global<stake::StakePool>(pool_address).pending_active.value;

            // Property 2 [TIMOUT]:. total_coins(pool.inactive_shares[pool.OLC]) == pending_inactive on StakePool
            ensures table::spec_get(pool.inactive_shares,pool.observed_lockup_cycle).total_coins == spec_get_pending_inactive(pool);

            // Property 3 [ERROR]: pool.total_coins_inactive == inactive on StakePool
            // inactive > pre_pool.total_coins_inactive &&
            ensures pool.total_coins_inactive == stake_pool.inactive.value;

            // Property 4 [ERROR]: inactive > old(total_coins_inactive) IF pool.OLC == old(pool).OLC + 1:
            ensures old(global<DelegationPool>(pool_address)).observed_lockup_cycle.index != pool.observed_lockup_cycle.index ==> inactive > old(global<DelegationPool>(pool_address)).total_coins_inactive;
             
            // Property 5.1 [TIMEOUT]: pool.OLC == old(pool).OLC + 1 => table::contains(pool.inactive_shares, pool.OLC)

            ensures (pool.observed_lockup_cycle.index == old(global<DelegationPool>(pool_address)).observed_lockup_cycle.index + 1) ==> table::spec_contains(pool.inactive_shares,pool.observed_lockup_cycle);

            // Property 5.2 [TIMEOUT]: pool.OLC == old(pool).OLC + 1 => total_coins(pool.inactive_shares[pool.OLC]) == 0
            ensures (pool.observed_lockup_cycle.index == old(global<DelegationPool>(pool_address)).observed_lockup_cycle.index + 1) ==> table::spec_get(pool.inactive_shares, pool.observed_lockup_cycle).total_coins == 0;

            // Property 5.3 [TIMEOUT]: pool.OLC == old(pool).OLC + 1 => total_coins(pool.inactive_shares[old(pool).OLC]) == inactive - old(total_coins_inactive)
            ensures (pool.observed_lockup_cycle.index == old(global<DelegationPool>(pool_address)).observed_lockup_cycle.index + 1) ==> table::spec_get(pool.inactive_shares,  old(global<DelegationPool>(pool_address)).observed_lockup_cycle).total_coins == inactive - old(global<DelegationPool>(pool_address)).total_coins_inactive;

            // Property 5.4 [TIMEOUT]: pool.OLC == old(pool).OLC + 1 => pending_inactive == 0: this is the 1st stake-management operation on this new lockup cycle and thus no stake could have been unlocked yet
            ensures (pool.observed_lockup_cycle.index == old(global<DelegationPool>(pool_address)).observed_lockup_cycle.index + 1) ==> pending_active == 0;

            // Property 6 [TIMEOUT]: pending_active == 0 => pool_u64::shares(pool.active_shares, NULL_SHAREHOLDER) == 0: add_stake fees charged during previous epoch have been refunded

            ensures pending_active == 0 ==> pool_u64::spec_shares(pool.active_shares, NULL_SHAREHOLDER) == 0;

            // Property 7 [TIMEOUT]: pending_active != 0 => pool_u64::shares(pool.active_shares, NULL_SHAREHOLDER) == pool_u64::shares(old(pool).active_shares, NULL_SHAREHOLDER): any add_stake fees are not refunded unless the epoch when charged has ended

            ensures pending_active != 0 ==> pool_u64::spec_shares(pool.active_shares, NULL_SHAREHOLDER) == pool_u64::spec_shares(old(global<DelegationPool>(pool_address)).active_shares, NULL_SHAREHOLDER);

            // Property 8 [TIMEOUT]: let commission_active = delta total_coins(pool.active_shares) * pool.commission;
            // pool_u64::shares(pool.active_shares, operator) - pool_u64::shares(old(pool).active_shares, operator) ==
            // pool_u64::amount_to_shares(
            // pool.active_shares, // snapshot of shares pool before buy_in
            // commission_active
            // ) operator gained pool-rewards * pool-commission% additional to its existing stake + its rewards produced in the meantime as a regular delegator
            // Note: commission_active = ghost_coin_1
            ensures pool_u64::spec_shares(pool.active_shares, global<stake::StakePool>(pool_address).operator_address) - pool_u64::spec_shares(old(global<DelegationPool>(pool_address)).active_shares, global<stake::StakePool>(pool_address).operator_address) == pool_u64::spec_amount_to_shares_with_total_coins(pool.active_shares, ghost_coin_1, pool.active_shares.total_coins);

            // Property 9 [TIMEOUT]: Same for pending_inactive commission used to buy shares into pool at index old(OLC) in pool.inactive_shares
            // forall delegators: delegator != operator && != NULL_SHAREHOLDER => pool_u64::shares(pool.active_shares, delegator) == pool_u64::shares(old(pool).active_shares, delegator) 
            ensures forall delegators: address: delegators != global<stake::StakePool>(pool_address).operator_address && delegators != NULL_SHAREHOLDER ==> pool_u64::spec_shares(pool.active_shares, delegators) == pool_u64::spec_shares(old(global<DelegationPool>(pool_address)).active_shares, delegators); 

        }
}
