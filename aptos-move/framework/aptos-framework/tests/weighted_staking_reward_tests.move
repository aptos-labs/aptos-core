#[test_only]
module aptos_framework::weighted_staking_reward_tests {
    use aptos_framework::account;
    use aptos_framework::stake;
    use aptos_framework::timestamp;
    use aptos_framework::weighted_staking_reward;

    // Test constants
    const LOCKUP_UNIT_SECS: u64 = 1209600; // 14 days in seconds (for stake module init)
    const ONE_APT: u64 = 100000000;

    // Bucket duration constants (matching weighted_staking_reward.move)
    const BUCKET_0_DURATION_SECS: u64 = 1296000; // 15 days
    const BUCKET_1_DURATION_SECS: u64 = 2592000; // 30 days
    const BUCKET_2_DURATION_SECS: u64 = 5184000; // 60 days
    const BUCKET_3_DURATION_SECS: u64 = 7776000; // 90 days

    // Helper function to set up aptos framework
    fun setup_test(aptos_framework: &signer) {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Initialize stake module so we can mint test coins
        stake::initialize_for_test_custom(
            aptos_framework,
            100 * ONE_APT,      // minimum_stake
            10000 * ONE_APT,    // maximum_stake
            LOCKUP_UNIT_SECS,
            true,               // allow_validator_set_change
            1,                  // rewards_rate_numerator
            100,                // rewards_rate_denominator
            1000000             // voting_power_increase_limit
        );

        weighted_staking_reward::test_initialize(aptos_framework);
    }

    // Helper function to create test accounts
    fun create_test_account(aptos_framework: &signer, addr: address): signer {
        account::create_account_for_test(addr);
        account::create_signer_for_test(addr)
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize(aptos_framework: &signer) {
        setup_test(aptos_framework);

        // Verify initial config
        let base_share_bps = weighted_staking_reward::get_base_share_bps();
        assert!(base_share_bps == 10000, 0); // 100% base, 0% bonus

        // Verify bucket configs (now 4 buckets: 15/30/60/90 days with 1x/2x/4x/6x multipliers)
        let (duration0, multiplier0) = weighted_staking_reward::get_bucket_config(0);
        assert!(duration0 == 1296000, 1); // 15 days in seconds
        assert!(multiplier0 == 10000, 2); // 1.0x

        let (duration1, multiplier1) = weighted_staking_reward::get_bucket_config(1);
        assert!(duration1 == 2592000, 3); // 30 days
        assert!(multiplier1 == 20000, 4); // 2.0x

        let (duration2, multiplier2) = weighted_staking_reward::get_bucket_config(2);
        assert!(duration2 == 5184000, 5); // 60 days
        assert!(multiplier2 == 40000, 6); // 4.0x

        let (duration3, multiplier3) = weighted_staking_reward::get_bucket_config(3);
        assert!(duration3 == 7776000, 7); // 90 days
        assert!(multiplier3 == 60000, 8); // 6.0x
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_bonus_pool(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        assert!(weighted_staking_reward::is_bonus_pool_initialized(@0x100), 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x80002, location = aptos_framework::weighted_staking_reward)]
    fun test_initialize_bonus_pool_twice_fails(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);
        weighted_staking_reward::test_initialize_bonus_pool(&pool); // Should fail
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_update_base_share(aptos_framework: &signer) {
        setup_test(aptos_framework);

        // Update to 75% base, 25% bonus
        weighted_staking_reward::update_base_share(aptos_framework, 7500);

        let base_share_bps = weighted_staking_reward::get_base_share_bps();
        assert!(base_share_bps == 7500, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10008, location = aptos_framework::weighted_staking_reward)]
    fun test_update_base_share_invalid_fails(aptos_framework: &signer) {
        setup_test(aptos_framework);

        // Try to set to >100%
        weighted_staking_reward::update_base_share(aptos_framework, 10001); // Should fail
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_update_bucket_multiplier(aptos_framework: &signer) {
        setup_test(aptos_framework);

        // Update all 4 bucket multipliers
        let new_multipliers = vector[15000, 25000, 35000, 45000]; // 1.5x, 2.5x, 3.5x, 4.5x
        weighted_staking_reward::update_bucket_multipliers(aptos_framework, new_multipliers);

        // Verify bucket 0 was updated
        let (duration0, multiplier0) = weighted_staking_reward::get_bucket_config(0);
        assert!(duration0 == 1296000, 0); // Duration unchanged
        assert!(multiplier0 == 15000, 1); // Multiplier updated to 1.5x

        // Verify bucket 3 was updated
        let (duration3, multiplier3) = weighted_staking_reward::get_bucket_config(3);
        assert!(duration3 == 7776000, 2); // Duration unchanged
        assert!(multiplier3 == 45000, 3); // Multiplier updated to 4.5x
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_sync_bonus_rewards_with_100_percent_base(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        // With default config (100% base), all rewards go to base
        let (base_rewards, bonus_rewards) = weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        assert!(base_rewards == 1000, 0);
        assert!(bonus_rewards == 0, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_sync_bonus_rewards_with_75_percent_base(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        // Update to 75% base, 25% bonus
        weighted_staking_reward::update_base_share(aptos_framework, 7500);

        let (base_rewards, bonus_rewards) = weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // With no buckets having shares, bonus gets added back to base
        assert!(base_rewards == 1000, 0);
        assert!(bonus_rewards == 0, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_join_bucket_single_user(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        let user = create_test_account(aptos_framework, @0x200);
        let shares = 1000u128;

        // Join bucket 0 (14 days)
        weighted_staking_reward::join_bucket(&user, @0x100, 0, shares);

        // Verify position created
        let (bucket_id, pos_shares, lock_start_secs, pending, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(bucket_id == 0, 0);
        assert!(pos_shares == shares, 1);
        assert!(lock_start_secs == 0, 2); // timestamp starts at 0 in tests
        assert!(pending == 0, 3); // No rewards yet

        // Verify pool state updated (single shared pool now)
        let (total_weighted_shares, acc) = weighted_staking_reward::get_pool_state(@0x100);
        // total_weighted_shares = shares * multiplier_bps / 10000
        // For bucket 0: multiplier = 10000 (1.0x), so weighted = shares
        assert!(total_weighted_shares == shares, 4);
        assert!(acc == 0, 5); // No rewards distributed yet
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x80005, location = aptos_framework::weighted_staking_reward)]
    fun test_join_bucket_twice_fails(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        let user = create_test_account(aptos_framework, @0x200);

        weighted_staking_reward::join_bucket(&user, @0x100, 0, 1000);
        weighted_staking_reward::join_bucket(&user, @0x100, 1, 1000); // Should fail
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_single_user_earn_rewards(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        // Set to 50% base, 50% bonus
        weighted_staking_reward::update_base_share(aptos_framework, 5000);

        let user = create_test_account(aptos_framework, @0x200);
        let shares = 1000u128;

        // Join bucket 0
        weighted_staking_reward::join_bucket(&user, @0x100, 0, shares);

        // Distribute 1000 total rewards
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // Check pending bonus (should be 500, the bonus portion)
        let (_, _, _, pending, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(pending == 500, 0);

        // Fast forward past lockup period to allow claiming
        timestamp::fast_forward_seconds(LOCKUP_UNIT_SECS + 1);

        // Claim bonus
        let claimed = weighted_staking_reward::claim_bonus(&user, @0x100);
        assert!(claimed == 500, 1);

        // After claim, pending should be 0
        let (_, _, _, pending_after, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(pending_after == 0, 2);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_multiple_users_same_bucket(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 5000); // 50/50 split

        let user1 = create_test_account(aptos_framework, @0x200);
        let user2 = create_test_account(aptos_framework, @0x201);

        // User1: 1000 shares, User2: 3000 shares (1:3 ratio)
        weighted_staking_reward::join_bucket(&user1, @0x100, 0, 1000);
        weighted_staking_reward::join_bucket(&user2, @0x100, 0, 3000);

        // Distribute 1000 total rewards -> 500 bonus
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // User1 should get 1/4 of bonus (125), User2 should get 3/4 (375)
        let (_, _, _, pending1, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        let (_, _, _, pending2, _) = weighted_staking_reward::get_position(@0x100, @0x201);

        assert!(pending1 == 125, 0);
        assert!(pending2 == 375, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_multiple_buckets_weighted_distribution(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 0); // 100% bonus for easier math

        // Set simple multipliers: bucket0=1x (10000), bucket1=2x (20000)
        let multipliers = vector[10000, 20000, 40000, 60000];
        weighted_staking_reward::update_bucket_multipliers(aptos_framework, multipliers);

        let user1 = create_test_account(aptos_framework, @0x200);
        let user2 = create_test_account(aptos_framework, @0x201);

        // Both have 1000 shares
        weighted_staking_reward::join_bucket(&user1, @0x100, 0, 1000); // weight = 1000 * 10000 = 10,000,000
        weighted_staking_reward::join_bucket(&user2, @0x100, 1, 1000); // weight = 1000 * 20000 = 20,000,000
        // Total weight = 30,000,000
        // User1 gets 1/3, User2 gets 2/3

        // Distribute 900 rewards (all bonus)
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 900);

        let (_, _, _, pending1, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        let (_, _, _, pending2, _) = weighted_staking_reward::get_position(@0x100, @0x201);

        // User1: 900 * 1/3 = 300
        // User2: 900 * 2/3 = 600
        assert!(pending1 == 300, 0);
        assert!(pending2 == 600, 1);
    }

    // Note: extend_lockup tests removed as function was replaced with upgrade_bucket/downgrade_bucket
    // in the auto-renewal model. See weighted_staking_reward_integration_tests.move for
    // comprehensive tests of the new upgrade/downgrade functionality.

    #[test(aptos_framework = @aptos_framework)]
    fun test_exit_after_maturity(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 5000); // 50/50

        let user = create_test_account(aptos_framework, @0x200);

        weighted_staking_reward::join_bucket(&user, @0x100, 0, 1000);

        // Distribute rewards
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // Fast forward time past complete cycle (15 days for bucket 0)
        timestamp::fast_forward_seconds(BUCKET_0_DURATION_SECS);

        // Exit position (after complete cycle, at cycle boundary, no penalty)
        let (bonus_claimed, burned, _shares) = weighted_staking_reward::exit_bucket(&user, @0x100);

        assert!(bonus_claimed == 500, 0); // Gets the bonus rewards
        assert!(burned == 0, 1); // No penalty after complete cycle

        // Verify pool state updated
        let (total_weighted_shares, _) = weighted_staking_reward::get_pool_state(@0x100);
        assert!(total_weighted_shares == 0, 2); // All weighted shares removed from pool
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_early_exit_forfeits_bonus(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 5000); // 50/50

        let user = create_test_account(aptos_framework, @0x200);

        weighted_staking_reward::join_bucket(&user, @0x100, 0, 1000);

        // Distribute rewards
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // Exit before maturity
        let (bonus_claimed, _burned, _shares) = weighted_staking_reward::exit_bucket(&user, @0x100);

        assert!(bonus_claimed == 0, 0); // Forfeited due to early exit
        // assert!(penalty_applied, 1); // Penalty flag set
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_post_maturity_continuation(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 0); // 100% bonus

        let user = create_test_account(aptos_framework, @0x200);

        weighted_staking_reward::join_bucket(&user, @0x100, 0, 1000);

        // Distribute some rewards before first cycle completes
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        let (_, _, _, pending1, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(pending1 == 1000, 0);

        // Fast forward past first cycle (15 days for bucket 0)
        timestamp::fast_forward_seconds(BUCKET_0_DURATION_SECS);

        // Continue to earn rewards after first cycle (auto-renewal)
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 500);

        let (_, _, _, pending2, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(pending2 == 1500, 1); // Accumulated both cycles' rewards

        // Can still exit and claim all (at exact cycle boundary)
        let (bonus_claimed, burned, _shares) = weighted_staking_reward::exit_bucket(&user, @0x100);
        assert!(bonus_claimed == 1500, 2);
        assert!(burned == 0, 3); // No penalty at cycle boundary
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_empty_bucket_no_crash(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 5000); // 50% base, 50% bonus

        // Distribute rewards with no users in any bucket
        // Bonus gets added back to base since it can't be distributed
        let (base_rewards, bonus_rewards) = weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // With 50/50 split but no buckets having shares:
        // - Expected: 500 base + 500 bonus
        // - Actual: 500 base + 500 (returned) = 1000 base, 0 bonus
        assert!(base_rewards == 1000, 0); // All rewards go to base when no buckets have shares
        assert!(bonus_rewards == 0, 1); // No bonus distributed

        // Verify pool behaves normally - all rewards distributed as base
        // This ensures backward compatibility when no one participates in lockup
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_bonus_returns_to_base_when_all_exit(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 5000); // 50/50 split

        let user1 = create_test_account(aptos_framework, @0x200);

        // User joins bucket 0
        weighted_staking_reward::join_bucket(&user1, @0x100, 0, 1000);

        // Distribute rewards - should split 50/50
        let (base1, bonus1) = weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);
        assert!(base1 == 500, 0);
        assert!(bonus1 == 500, 1);

        // User exits (all buckets now empty)
        let (_claimed, _burned, _shares) = weighted_staking_reward::exit_bucket(&user1, @0x100);

        // Distribute more rewards - bonus should return to base since no buckets have shares
        let (base2, bonus2) = weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);
        assert!(base2 == 1000, 2); // All goes to base now
        assert!(bonus2 == 0, 3); // No bonus distributed

        // This verifies the pool automatically reverts to normal behavior
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_claim_bonus_twice(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 0); // 100% bonus

        let user = create_test_account(aptos_framework, @0x200);

        weighted_staking_reward::join_bucket(&user, @0x100, 0, 1000);
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        // Fast forward past lockup period to allow claiming
        timestamp::fast_forward_seconds(LOCKUP_UNIT_SECS + 1);

        // First claim
        let claimed1 = weighted_staking_reward::claim_bonus(&user, @0x100);
        assert!(claimed1 == 1000, 0);

        // Second claim immediately (should be 0)
        let claimed2 = weighted_staking_reward::claim_bonus(&user, @0x100);
        assert!(claimed2 == 0, 1);

        // Distribute more rewards
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 500);

        // Claim again
        let claimed3 = weighted_staking_reward::claim_bonus(&user, @0x100);
        assert!(claimed3 == 500, 2);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_join_after_other_users_earned_rewards(aptos_framework: &signer) {
        setup_test(aptos_framework);

        let pool = create_test_account(aptos_framework, @0x100);
        weighted_staking_reward::test_initialize_bonus_pool(&pool);

        weighted_staking_reward::update_base_share(aptos_framework, 0); // 100% bonus

        let user1 = create_test_account(aptos_framework, @0x200);
        let user2 = create_test_account(aptos_framework, @0x201);

        // User1 joins first
        weighted_staking_reward::join_bucket(&user1, @0x100, 0, 1000);

        // Distribute some rewards (user1 should get all)
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        let (_, _, _, pending1_before, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        assert!(pending1_before == 1000, 0);

        // User2 joins after rewards distributed
        weighted_staking_reward::join_bucket(&user2, @0x100, 0, 1000);

        // User2 should start with 0 pending (not get historical rewards)
        let (_, _, _, pending2_initial, _) = weighted_staking_reward::get_position(@0x100, @0x201);
        assert!(pending2_initial == 0, 1);

        // Distribute more rewards (should be split 50/50 now)
        weighted_staking_reward::test_sync_bonus_rewards(@0x100, 1000);

        let (_, _, _, pending1_after, _) = weighted_staking_reward::get_position(@0x100, @0x200);
        let (_, _, _, pending2_after, _) = weighted_staking_reward::get_position(@0x100, @0x201);

        assert!(pending1_after == 1500, 2); // 1000 old + 500 new
        assert!(pending2_after == 500, 3);  // 500 new only
    }

    // Note: test_early_exit_redistributes_to_all_buckets removed because in the auto-renewal model,
    // early exit BURNS rewards instead of redistributing them. See weighted_staking_reward_integration_tests.move
    // for comprehensive tests of the burning mechanism.
}
