module rewards::rewards {
    use std::signer;
    use std::vector;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::aptos_account;

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::coin::Coin;

    /// Caller is not authorised to perform the action.
    const ENOT_AUTHORISED: u64 = 1;
    /// No rewards to claim.
    const ENO_REWARDS_TO_CLAIM: u64 = 2;

    struct RewardStore has key {
        admin: address,
        rewards: SmartTable<address, Coin<AptosCoin>>,
    }

    fun init_module(rewards_signer: &signer) {
        move_to(rewards_signer, RewardStore {
            admin: @rewards,
            rewards: smart_table::new(),
        });
    }

    #[view]
    /// Returns the pending rewards for the caller.
    public fun pending_rewards(user: address): u64 acquires RewardStore {
        let rewards_store = borrow_global<RewardStore>(@rewards);
        if (!smart_table::contains(&rewards_store.rewards, user)) {
            return 0
        };

        let rewards = smart_table::borrow(&rewards_store.rewards, user);
        coin::value(rewards)
    }

    /// Allow admin to upload rewards for multiple recipients.
    public entry fun add_rewards(admin: &signer, recipients: vector<address>, amounts: vector<u64>) acquires RewardStore {
        assert_is_admin(signer::address_of(admin));
        let rewards_store = borrow_global_mut<RewardStore>(@rewards);
        vector::zip(recipients, amounts, |recipient, amount| {
            // Extract rewards from the admin's account.
            let reward = coin::withdraw<AptosCoin>(admin, amount);
            // Add to current rewards (can be 0).
            if (!smart_table::contains(&rewards_store.rewards, recipient)) {
                smart_table::add(&mut rewards_store.rewards, recipient, coin::zero());
            };
            let current_rewards = smart_table::borrow_mut(&mut rewards_store.rewards, recipient);
            coin::merge(current_rewards, reward);
        });
    }

    public entry fun cancel_rewards(admin: &signer, recipients: vector<address>) acquires RewardStore {
        let admin_addr = signer::address_of(admin);
        assert_is_admin(admin_addr);
        let rewards_store = borrow_global_mut<RewardStore>(@rewards);
        vector::for_each(recipients, |recipient| {
            let rewards = smart_table::remove(&mut rewards_store.rewards, recipient);
            aptos_account::deposit_coins(admin_addr, rewards);
        });
    }

    /// Claim rewards for the caller. This errors out if there are no rewards to claim.
    public entry fun claim_reward(user: &signer) acquires RewardStore {
        let rewards_store = borrow_global_mut<RewardStore>(@rewards);
        let user_address = signer::address_of(user);
        assert!(smart_table::contains(&rewards_store.rewards, user_address), ENO_REWARDS_TO_CLAIM);
        let rewards = smart_table::remove(&mut rewards_store.rewards, user_address);
        aptos_account::deposit_coins(user_address, rewards);
    }

    /// Transfer the admin role to a new address.
    public entry fun transfer_admin_role(admin: &signer, new_admin: address) acquires RewardStore {
        assert_is_admin(signer::address_of(admin));
        let rewards_store = borrow_global_mut<RewardStore>(@rewards);
        rewards_store.admin = new_admin;
    }

    fun assert_is_admin(admin: address) acquires RewardStore {
        let rewards_store = borrow_global<RewardStore>(@rewards);
        assert!(rewards_store.admin == admin, ENOT_AUTHORISED);
    }

    #[test_only]
    public fun init_for_test(admin: &signer) {
        move_to(admin, RewardStore {
            admin: signer::address_of(admin),
            rewards: smart_table::new(),
        });
    }
}
