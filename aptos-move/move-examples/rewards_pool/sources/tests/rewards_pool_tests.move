#[test_only]
module rewards_pool::rewards_pool_tests {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;
    use rewards_pool::rewards_pool::{Self, RewardsPool};
    use rewards_pool::test_helpers;
    use rewards_pool::epoch;
    use std::signer;
    use std::vector;

    #[test(claimer_1 = @0xdead, claimer_2 = @0xbeef, claimer_3 = @0xfeed)]
    fun test_e2e(claimer_1: &signer, claimer_2: &signer, claimer_3: &signer) {
        test_helpers::set_up();
        // Create a rewards pool with 1 native fungible asset and 1 coin.
        let asset_rewards_1 = test_helpers::create_fungible_asset_and_mint(claimer_1, b"test1", 1000);
        let asset_rewards_2 = test_helpers::create_fungible_asset_and_mint(claimer_1, b"test2", 2000);
        let asset_1 = fungible_asset::asset_metadata(&asset_rewards_1);
        let asset_2 = fungible_asset::asset_metadata(&asset_rewards_2);
        let rewards_pool = rewards_pool::create(vector[asset_1, asset_2]);

        // First epoch, claimers 1 and 2 split the rewards.
        increase_alocation(claimer_1, rewards_pool, 60);
        increase_alocation(claimer_2, rewards_pool, 40);
        epoch::fast_forward(1);
        add_rewards(rewards_pool, &mut asset_rewards_1, &mut asset_rewards_2, 100, 200);
        claim_and_verify_rewards(claimer_1, rewards_pool, vector[asset_1, asset_2], 101, vector[60, 120]);
        claim_and_verify_rewards(claimer_2, rewards_pool, vector[asset_1, asset_2], 101, vector[40, 80]);

        // Second epoch, there are 3 claimers, one of them has allocation decreased.
        increase_alocation(claimer_1, rewards_pool, 30);
        increase_alocation(claimer_2, rewards_pool, 50);
        increase_alocation(claimer_3, rewards_pool, 30);
        decrease_alocation(claimer_3, rewards_pool, 10);
        epoch::fast_forward(1);
        add_rewards(rewards_pool, &mut asset_rewards_1, &mut asset_rewards_2, 100, 200);
        claim_and_verify_rewards(claimer_1, rewards_pool, vector[asset_1, asset_2], 102, vector[30, 60]);
        claim_and_verify_rewards(claimer_2, rewards_pool, vector[asset_1, asset_2], 102, vector[50, 100]);
        claim_and_verify_rewards(claimer_3, rewards_pool, vector[asset_1, asset_2], 102, vector[20, 40]);

        test_helpers::clean_up(vector[asset_rewards_1, asset_rewards_2]);
    }

    fun add_rewards(
        pool: Object<RewardsPool>,
        rewards_1: &mut FungibleAsset,
        rewards_2: &mut FungibleAsset,
        amount_1: u64,
        amount_2: u64,
    ) {
        rewards_pool::add_rewards(
            pool,
            vector[fungible_asset::extract(rewards_1, amount_1), fungible_asset::extract(rewards_2, amount_2)],
            epoch::now() - 1,
        );
    }

    fun increase_alocation(claimer: &signer, pool: Object<RewardsPool>, amount: u64) {
        rewards_pool::increase_allocation(signer::address_of(claimer), pool, amount);
    }

    fun decrease_alocation(claimer: &signer, pool: Object<RewardsPool>, amount: u64) {
        rewards_pool::decrease_allocation(signer::address_of(claimer), pool, amount);
    }

    fun claim_and_verify_rewards(
        claimer: &signer,
        pool: Object<RewardsPool>,
        tokens: vector<Object<Metadata>>,
        epoch: u64,
        expected_amounts: vector<u64>,
    ) {
        let claimer_addr = signer::address_of(claimer);
        vector::zip(tokens, expected_amounts, |token, expected_amount| {
            let rewards = rewards_pool::claim_rewards(claimer_addr, pool, token, epoch);
            assert!(fungible_asset::amount(&rewards) == expected_amount, 0);
            primary_fungible_store::deposit(claimer_addr, rewards);
        });
    }
}
