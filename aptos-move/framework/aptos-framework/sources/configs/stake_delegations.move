module aptos_framework::stake_delegations {
    friend aptos_framework::genesis;

    use std::signer;
    use std::error;

    use aptos_std::event::{EventHandle, new_event_handle};
    use aptos_std::simple_map::{Self, SimpleMap};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::stake::{Self, OwnerCapability};
    use aptos_framework::system_addresses;

    const EINVALID_COMMISSION_PERCENT: u64 = 1;
    const EDELEGATION_NOT_FOUND: u64 = 2;
    const EVALIDATOR_INSUFFICIENT_STAKE: u64 = 3;
    const EVALIDATOR_INSUFFICIENT_LOCKUP: u64 = 4;
    const ETOO_MANY_DELEGATIONS: u64 = 5;
    const EINSUFFICIENT_WITHDRAWABLE_STAKE: u64 = 6;

    struct DelegationsPool has key {
        pool_address: address,
        owner_cap: OwnerCapability,
        commission_percent: u64,
        delegations: SimpleMap<address, u64>,
        total_delegations: u64,
        events: DelegationsPoolEvents,
    }

    struct DelegationsConfig has key {
        max_delegations_per_pool: u64,
    }

    struct DelegationsPoolEvents has store {
        initialize_delegations_pool_events: EventHandle<InitializeDelegationsPoolEvent>,
    }

    struct InitializeDelegationsPoolEvent has drop, store {
        pool_address: address,
        commission_percent: u64,
    }

    public(friend) fun initialize(aptos_framework: &signer, max_delegations_per_pool: u64) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, DelegationsConfig { max_delegations_per_pool });
    }

    public fun get_max_delegations_per_pool(): u64 acquires DelegationsConfig {
        borrow_global<DelegationsConfig>(@aptos_framework).max_delegations_per_pool
    }

    public fun set_max_delegations_per_pool(
        aptos_framework: &signer,
        new_max_delegations_per_pool: u64,
    ) acquires DelegationsConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<DelegationsConfig>(@aptos_framework).max_delegations_per_pool =new_max_delegations_per_pool;
    }

    public entry fun initialize_delegations_pool(
        validator: &signer,
        commission_percent: u64,
    ) {
        assert!(commission_percent < 100, error::invalid_argument(EINVALID_COMMISSION_PERCENT));

        let pool_address = signer::address_of(validator);
        let delegations = simple_map::create<address, u64>();
        let (active_stake, inactive_stake, pending_active_stake, pending_inactive_stake) = stake::get_stake(pool_address);
        let existing_stake = active_stake + inactive_stake + pending_active_stake + pending_inactive_stake;

        // Verify that the validator has a valid lockup. This is required since we cannot change the lockup after
        // creating a delegations pool.
        let (min_lockup_secs, _) = stake::get_required_lockup();
        assert!(
            stake::get_remaining_lockup_secs(pool_address) >= min_lockup_secs,
            error::invalid_argument(EVALIDATOR_INSUFFICIENT_LOCKUP),
        );

        // Treat validator's existing stake as a delegation. This is fine from a commission perspective as it'd go to
        // the validator either way.
        simple_map::add(&mut delegations, pool_address, existing_stake);

        let owner_cap = stake::extract_owner_cap(validator);
        move_to(validator, DelegationsPool {
            pool_address,
            owner_cap,
            commission_percent,
            delegations,
            total_delegations: 0,
            events: DelegationsPoolEvents {
                initialize_delegations_pool_events: new_event_handle<InitializeDelegationsPoolEvent>(validator)
            }
        });
    }

    public entry fun delegate(
        delegator: &signer,
        delegations_pool_address: address,
        amount: u64,
    ) acquires DelegationsConfig, DelegationsPool {
        delegate_coins(signer::address_of(delegator), delegations_pool_address, coin::withdraw<AptosCoin>(delegator, amount));
    }

    public fun delegate_coins(
        delegator_address: address,
        delegations_pool_address: address,
        coins: Coin<AptosCoin>,
    ) acquires DelegationsConfig, DelegationsPool {
        // Delegations can only be sent to validators that still have an active lockup.
        assert!(
            stake::get_remaining_lockup_secs(delegations_pool_address) > 0,
            error::invalid_argument(EVALIDATOR_INSUFFICIENT_LOCKUP),
        );

        let pool = borrow_global_mut<DelegationsPool>(delegations_pool_address);
        let amount = coin::value(&coins);
        stake::add_stake_with_cap(delegations_pool_address, &pool.owner_cap, coins);

        pool.total_delegations = pool.total_delegations + amount;
        // Separate update or insert to save on gas.
        if (simple_map::contains_key(&pool.delegations, &delegator_address)) {
            let delegation = simple_map::borrow_mut(&mut pool.delegations, &delegator_address);
            *delegation = *delegation + amount;
        } else {
            // Limit the number of delegations in a pool for performance and security reasons.
            assert!(
                simple_map::length(&pool.delegations) <= get_max_delegations_per_pool(),
                error::invalid_argument(ETOO_MANY_DELEGATIONS),
            );

            simple_map::add(&mut pool.delegations, delegator_address, amount);
        };
    }

    // Anyone can request unlock. This only works if the delegations pool's lockup has expired.
    public entry fun unlock(delegations_pool_address: address) acquires DelegationsPool {
        let pool = borrow_global_mut<DelegationsPool>(delegations_pool_address);
        let (active_stake, _, _, _) = stake::get_stake(delegations_pool_address);
        stake::unlock_with_cap(delegations_pool_address, active_stake, &pool.owner_cap);
    }

    // Withdrawal is separate as unlocking might take up to an epoch.
    public entry fun withdraw(delegator: &signer, delegations_pool_address: address) acquires DelegationsPool {
        // Ensure that there's inactive stake to withdraw from.
        // Unlock has to be called first which moves stake into an inactive state after one epoch.
        let (_, total_withdrawal_stake, _, _) = stake::get_stake(delegations_pool_address);
        assert!(total_withdrawal_stake > 0, error::invalid_argument(EINSUFFICIENT_WITHDRAWABLE_STAKE));

        let delegator_address = signer::address_of(delegator);
        let pool = borrow_global_mut<DelegationsPool>(delegations_pool_address);
        assert!(simple_map::contains_key(&pool.delegations, &delegator_address), error::invalid_argument(EDELEGATION_NOT_FOUND));

        let origin_stake = *simple_map::borrow(&mut pool.delegations, &delegator_address);
        simple_map::remove(&mut pool.delegations, &delegator_address);
        pool.total_delegations = pool.total_delegations - origin_stake;
        // This cannot overflow as long the square of maximum stake set by stake module doesn't execeed u64.max.
        let stake_with_rewards_amount = total_withdrawal_stake * origin_stake / pool.total_delegations;

        // Only request withdraw if there's any
        let stake_with_rewards = stake::withdraw_with_cap(delegations_pool_address, &pool.owner_cap, stake_with_rewards_amount);
        let rewards_amount = coin::value<AptosCoin>(&stake_with_rewards) - origin_stake;
        let commission_amount = rewards_amount * pool.commission_percent / 100;
        let commission = coin::extract(&mut stake_with_rewards, commission_amount);

        coin::deposit(delegator_address, stake_with_rewards);
        coin::deposit(delegations_pool_address, commission);
    }
}
