module rewards_pool::rewards_pool {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleStore, Metadata};
    use aptos_framework::object::{Self, Object, ExtendRef};
    use aptos_std::pool_u64_unbound::{Self as pool_u64, Pool};
    use aptos_std::smart_table::{Self, SmartTable};

    use rewards_pool::epoch;

    use std::signer;
    use std::vector;

    /// Rewards can only be claimed for epochs that have ended.
    const EREWARDS_CANNOT_BE_CLAIMED_FOR_CURRENT_EPOCH: u64 = 1;
    /// The rewards pool does not support the given reward token type.
    const EREWARD_TOKEN_NOT_SUPPORTED: u64 = 2;
    /// The caller doesn't have any pending rewards.
    const ENO_REWARDS_TO_CLAIM: u64 = 3;

    struct EpochRewards has store {
        /// Total rewards for all reward tokens supported to be distributed for each epoch.
        amount: SmartTable<u64, u64>,
        /// As fungible assets are stored in separate store objects, we need to keep track of them.
        store: Object<FungibleStore>,
        /// We need to keep the fungible store's extend ref to be able to transfer rewards from it during claiming.
        store_extend_ref: ExtendRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct RewardsPool has key {
        /// Track allocation per address per epoch. This is used to calculate the rewards for each address.
        epoch_allocations: SmartTable<u64, Pool>,
        epoch_rewards: SmartTable<Object<Metadata>, EpochRewards>,
        reward_tokens: vector<Object<Metadata>>,
    }

    public entry fun create_entry(reward_tokens: vector<Object<Metadata>>) {
        create(reward_tokens);
    }

    /// Create a new rewards pool with the given reward tokens (fungible assets only)
    public fun create(reward_tokens: vector<Object<Metadata>>): Object<RewardsPool> {
        // The owner of the object doesn't matter as there are no owner-based permissions.
        let rewards_pool_constructor_ref = &object::create_object(@rewards_pool);
        let rewards_pool_signer = &object::generate_signer(rewards_pool_constructor_ref);
        let rewards_pool_addr = signer::address_of(rewards_pool_signer);
        let epoch_rewards = smart_table::new();
        vector::for_each(reward_tokens, |reward_token| {
            let reward_token: Object<Metadata> = reward_token;
            let store_constructor_ref = &object::create_object(rewards_pool_addr);
            let store = fungible_asset::create_store(store_constructor_ref, reward_token);
            smart_table::add(&mut epoch_rewards, reward_token, EpochRewards {
                amount: smart_table::new(),
                store,
                store_extend_ref: object::generate_extend_ref(store_constructor_ref),
            });
        });
        move_to(rewards_pool_signer, RewardsPool {
            epoch_allocations: smart_table::new(),
            epoch_rewards,
            reward_tokens,
        });
        object::object_from_constructor_ref(rewards_pool_constructor_ref)
    }

    #[view]
    /// Return the amounts of claimable rewards for a given claimer, rewards pool, and epoch.
    /// The return value is a vector of reward tokens and a vector of amounts.
    public fun claimable_rewards(
        claimer: address,
        rewards_pool: Object<RewardsPool>,
        epoch: u64,
    ): (vector<Object<Metadata>>, vector<u64>) acquires RewardsPool {
        assert!(epoch < epoch::now(), EREWARDS_CANNOT_BE_CLAIMED_FOR_CURRENT_EPOCH);
        let rewards_tokens = safe_rewards_pool_data(&rewards_pool).reward_tokens;
        let non_empty_reward_tokens = vector[];
        let reward_per_tokens = vector[];
        vector::for_each(rewards_tokens, |reward_token| {
            let reward = rewards(claimer, rewards_pool, reward_token, epoch);
            if (reward > 0) {
                vector::push_back(&mut non_empty_reward_tokens, reward_token);
                vector::push_back(&mut reward_per_tokens, reward);
            };
        });
        (non_empty_reward_tokens, reward_per_tokens)
    }

    public fun claim_rewards(
        claimer: address,
        rewards_pool: Object<RewardsPool>,
        reward_token: Object<Metadata>,
        epoch: u64,
    ): FungibleAsset acquires RewardsPool {
        assert!(epoch < epoch::now(), EREWARDS_CANNOT_BE_CLAIMED_FOR_CURRENT_EPOCH);
        let reward = rewards(claimer, rewards_pool, reward_token, epoch);
        assert!(reward > 0, ENO_REWARDS_TO_CLAIM);
        let reward_store = smart_table::borrow(&safe_rewards_pool_data(&rewards_pool).epoch_rewards, reward_token);
        let store_signer = &object::generate_signer_for_extending(&reward_store.store_extend_ref);
        fungible_asset::withdraw(store_signer, reward_store.store, reward)
    }

    /// Add rewards to the specified rewards pool.
    public fun add_rewards(
        rewards_pool: Object<RewardsPool>,
        fungible_assets: vector<FungibleAsset>,
        epoch: u64,
    ) acquires RewardsPool {
        vector::for_each(fungible_assets, |fa| {
            let reward_token = fungible_asset::metadata_from_asset(&fa);
            // This aborts if the reward token is not supported.
            let reward_store = smart_table::borrow_mut(
                &mut unchecked_mut_rewards_pool_data(&rewards_pool).epoch_rewards,
                reward_token,
            );
            let amount = fungible_asset::amount(&fa);
            let current_amount = smart_table::borrow_mut_with_default(&mut reward_store.amount, epoch, 0);
            *current_amount = *current_amount + amount;
            fungible_asset::deposit(reward_store.store, fa);
        });

        // TODO: Emit event
    }

    /// This should only be called by system modules to increase the shares of a claimer.
    public(friend) fun increase_allocation(
        claimer: address,
        rewards_pool: Object<RewardsPool>,
        amount: u64,
    ) acquires RewardsPool {
        pool_u64::buy_in(unchecked_mut_epoch_allocation(&rewards_pool), claimer, amount);
    }

    /// This should only be called by system modules to decrease the shares of a claimer.
    public(friend) fun decrease_allocation(
        claimer: address,
        rewards_pool: Object<RewardsPool>,
        amount: u64,
    ) acquires RewardsPool {
        let epoch_allocations = &mut unchecked_mut_rewards_pool_data(&rewards_pool).epoch_allocations;
        let current_epoch = epoch::now();
        let pool = smart_table::borrow_mut(epoch_allocations, current_epoch);
        pool_u64::redeem_shares(pool, claimer, (amount as u128));

        // Delete the epoch allocation pool if there are no remaining claimers.
        if (pool_u64::total_coins(pool) == 0) {
            let pool = smart_table::remove(epoch_allocations, current_epoch);
            pool_u64::destroy_empty(pool);
        };
    }

    fun rewards(
        claimer: address,
        rewards_pool: Object<RewardsPool>,
        reward_token: Object<Metadata>,
        epoch: u64,
    ): u64 acquires RewardsPool {
        let rewards_pool_data = safe_rewards_pool_data(&rewards_pool);
        if (!smart_table::contains(&rewards_pool_data.epoch_allocations, epoch)) {
            return 0
        };
        assert!(
            smart_table::contains(&rewards_pool_data.epoch_rewards, reward_token),
            EREWARD_TOKEN_NOT_SUPPORTED,
        );
        let all_epoch_rewards = smart_table::borrow(&rewards_pool_data.epoch_rewards, reward_token);
        let epoch_rewards = *smart_table::borrow_with_default(&all_epoch_rewards.amount, epoch, &0);
        let epoch_allocations = smart_table::borrow(&rewards_pool_data.epoch_allocations, epoch);
        let allocation = pool_u64::shares(epoch_allocations, claimer);
        pool_u64::shares_to_amount_with_total_coins(epoch_allocations, allocation, epoch_rewards)
    }

    inline fun safe_rewards_pool_data(
        rewards_pool: &Object<RewardsPool>,
    ): &RewardsPool acquires RewardsPool {
        borrow_global<RewardsPool>(object::object_address(rewards_pool))
    }

    inline fun unchecked_mut_epoch_allocation(rewards_pool: &Object<RewardsPool>): &mut Pool acquires RewardsPool {
        let epoch_allocations = &mut unchecked_mut_rewards_pool_data(rewards_pool).epoch_allocations;
        let current_epoch = epoch::now();
        if (!smart_table::contains(epoch_allocations, current_epoch)) {
            smart_table::add(epoch_allocations, current_epoch, pool_u64::create());
        };
        smart_table::borrow_mut(epoch_allocations, current_epoch)
    }

    inline fun unchecked_mut_rewards_pool_data(
        rewards_pool: &Object<RewardsPool>,
    ): &mut RewardsPool acquires RewardsPool {
        borrow_global_mut<RewardsPool>(object::object_address(rewards_pool))
    }

    #[test_only]
    friend rewards_pool::rewards_pool_tests;
}
