#[test_only]
module aptos_framework::weighted_staking_reward_integration_tests {
    use std::bls12381;
    use std::features;
    use std::signer;
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::delegation_pool as dp;
    use aptos_framework::reconfiguration;
    use aptos_framework::stake;
    use aptos_framework::timestamp;
    use aptos_framework::weighted_staking_reward;

    const EPOCH_DURATION: u64 = 60;
    const LOCKUP_CYCLE_SECONDS: u64 = 3600;
    const ONE_APT: u64 = 100000000;
    const BUCKET_0_DURATION_SECS: u64 = 1296000; // 15 days
    const BUCKET_1_DURATION_SECS: u64 = 2592000; // 30 days

    const DELEGATION_POOLS: u64 = 11;
    const MODULE_EVENT: u64 = 26;

    // Test helper functions

    fun initialize_for_test(aptos_framework: &signer) {
        account::create_account_for_test(signer::address_of(aptos_framework));
        stake::initialize_for_test_custom(
            aptos_framework,
            100 * ONE_APT,      // minimum_stake
            10000 * ONE_APT,    // maximum_stake
            LOCKUP_CYCLE_SECONDS,
            true,               // allow_validator_set_change
            1,                  // rewards_rate_numerator
            100,                // rewards_rate_denominator (1% rewards)
            1000000             // voting_power_increase_limit
        );
        reconfiguration::initialize_for_test(aptos_framework);
        features::change_feature_flags_for_testing(
            aptos_framework,
            vector[DELEGATION_POOLS, MODULE_EVENT],
            vector[]
        );

        // Initialize weighted staking reward config
        weighted_staking_reward::test_initialize(aptos_framework);
    }

    fun end_epoch() {
        stake::end_epoch();
        reconfiguration::reconfigure_for_test_custom();
    }

    fun fast_forward_seconds(seconds: u64) {
        timestamp::fast_forward_seconds(seconds);
    }

    // Generate BLS12-381 keys for validator
    fun generate_identity(): (bls12381::SecretKey, bls12381::PublicKey, bls12381::ProofOfPossession) {
        let (sk, pkpop) = bls12381::generate_keys();
        let pop = bls12381::generate_proof_of_possession(&sk);
        let pk = bls12381::public_key_with_pop_to_normal(&pkpop);
        (sk, pk, pop)
    }

    // Initialize validator with delegation pool and BLS keys
    fun setup_validator(validator: &signer, stake_amount: u64): address {
        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);

        // Initialize delegation pool
        dp::initialize_delegation_pool(validator, 0, vector::empty<u8>());
        let pool_address = dp::get_owned_pool_address(validator_address);

        // Register BLS keys
        let (_, pk, pop) = generate_identity();
        let pk_bytes = bls12381::public_key_to_bytes(&pk);
        let pop_bytes = bls12381::proof_of_possession_to_bytes(&pop);
        stake::rotate_consensus_key(validator, pool_address, pk_bytes, pop_bytes);

        // Enable lockup rewards
        dp::enable_lockup_rewards(validator, pool_address);

        // Add stake if specified
        if (stake_amount > 0) {
            stake::mint(validator, stake_amount);
            dp::add_stake(validator, pool_address, stake_amount);
        };

        pool_address
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator = @0x234)]
    public entry fun test_basic_auto_renewal_flow(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Delegator adds stake
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount);

        // Join validator set
        stake::join_validator_set(validator, pool_address);
        end_epoch(); // Activates validator

        // Set bonus rewards to 50% (default is 100% base, 0% bonus)
        weighted_staking_reward::update_base_share(aptos_framework, 5000);

        // Join bucket 0 (15 days) with all shares (1000 APT staked)
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator, pool_address, 0, all_shares);

        // Check initial position (no rewards yet, just joined)
        let (bucket_id, shares, lock_start_secs, pending_bonus, complete_cycles) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(bucket_id == 0, 1);
        assert!(shares == all_shares, 2);
        assert!(lock_start_secs > 0, 3);
        assert!(pending_bonus == 0, 4); // No rewards yet
        assert!(complete_cycles == 0, 5); // Just joined

        // Fast forward to middle of first cycle (7.5 days) and generate rewards
        fast_forward_seconds(BUCKET_0_DURATION_SECS / 2);
        end_epoch();
        dp::synchronize_delegation_pool(pool_address); // Sync rewards to bonus pool

        // Check position in middle of cycle
        let (_, _, _, pending_mid, cycles_mid) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(cycles_mid == 0, 6); // Still in first cycle
        assert!(pending_mid > 0, 7); // Has some rewards

        // Fast forward to complete first cycle (15 days total)
        fast_forward_seconds(BUCKET_0_DURATION_SECS / 2);
        end_epoch();
        dp::synchronize_delegation_pool(pool_address); // Sync rewards to bonus pool

        // Check position after first cycle
        let (_, _, _, pending_after_cycle, cycles_after) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(cycles_after == 1, 8); // Completed 1 cycle
        assert!(pending_after_cycle > pending_mid, 9); // More rewards

        // Claim rewards
        dp::claim_lockup_bonus(delegator, pool_address);

        // Check balance increased
        let balance = coin::balance<AptosCoin>(delegator_address);
        assert!(balance > 0, 10); // Received bonus to wallet

        // Exit bucket
        dp::exit_lockup_bucket(delegator, pool_address);

        // Verify no longer has position
        assert!(!weighted_staking_reward::has_position(pool_address, delegator_address), 11);
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator = @0x234)]
    public entry fun test_early_exit_burns_incomplete_cycle(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Delegator adds stake
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount);

        stake::join_validator_set(validator, pool_address);
        end_epoch(); // Activates validator

        // Set bonus rewards to 50%
        weighted_staking_reward::update_base_share(aptos_framework, 5000);

        // Join bucket
        // Join with all shares (100000000 staked)
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator, pool_address, 0, all_shares);

        // Generate initial rewards
        end_epoch();
        dp::synchronize_delegation_pool(pool_address); // Sync rewards to bonus pool

        // Fast forward to middle of cycle (7.5 days)
        fast_forward_seconds(BUCKET_0_DURATION_SECS / 2);
        end_epoch(); // Generate more rewards
        dp::synchronize_delegation_pool(pool_address); // Sync rewards to bonus pool

        // Check pending rewards before exit
        let (_, _, _, pending_before, _) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(pending_before > 0, 1); // Has earned some rewards

        // Exit early (in middle of cycle)
        let balance_before = coin::balance<AptosCoin>(delegator_address);
        dp::exit_lockup_bucket(delegator, pool_address);
        let balance_after = coin::balance<AptosCoin>(delegator_address);

        // Early exit should burn all rewards (none paid)
        assert!(balance_after == balance_before, 2); // No bonus paid
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator = @0x234)]
    public entry fun test_upgrade_bucket(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Delegator adds stake
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount);

        stake::join_validator_set(validator, pool_address);
        end_epoch();

        // Join bucket 0 (15 days) with all shares (1000 APT staked)
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator, pool_address, 0, all_shares);

        let (_, _, lock_start_original, _, _) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        // Fast forward to middle of cycle
        fast_forward_seconds(BUCKET_0_DURATION_SECS / 2);
        end_epoch();

        // Upgrade to bucket 1 (30 days) - should keep lock_start_secs
        dp::upgrade_lockup_bucket(delegator, pool_address, 1);

        let (new_bucket_id, _, lock_start_after, _, _) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(new_bucket_id == 1, 1); // Now in bucket 1
        assert!(lock_start_after == lock_start_original, 2); // Preserves original start time
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator = @0x234)]
    public entry fun test_downgrade_bucket_burns_incomplete(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Delegator adds stake
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount);

        stake::join_validator_set(validator, pool_address);
        end_epoch();

        // Join bucket 1 (30 days) with all shares
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator, pool_address, 1, all_shares);

        // Fast forward to middle of cycle
        fast_forward_seconds(BUCKET_1_DURATION_SECS / 2);
        end_epoch(); // Generate rewards

        let balance_before = coin::balance<AptosCoin>(delegator_address);

        // Downgrade to bucket 0 (15 days) - should burn incomplete cycle rewards
        dp::downgrade_lockup_bucket(delegator, pool_address, 0);

        let balance_after = coin::balance<AptosCoin>(delegator_address);

        // Downgrade should not pay rewards (burned)
        assert!(balance_after == balance_before, 1);

        // Verify now in bucket 0 with fresh lock_start_secs
        let (new_bucket_id, _, new_lock_start, _, new_cycles) =
            weighted_staking_reward::get_position(pool_address, delegator_address);

        assert!(new_bucket_id == 0, 2);
        assert!(new_cycles == 0, 3); // Fresh start, no complete cycles
        assert!(new_lock_start == timestamp::now_seconds(), 4); // Reset to now
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator1 = @0x234, delegator2 = @0x345)]
    public entry fun test_multiple_buckets_reward_distribution(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator1_address = signer::address_of(delegator1);
        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator1_address);
        account::create_account_for_test(delegator2_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Both delegators stake same amount
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator1, stake_amount);
        stake::mint(delegator2, stake_amount);
        dp::add_stake(delegator1, pool_address, stake_amount);
        dp::add_stake(delegator2, pool_address, stake_amount);

        stake::join_validator_set(validator, pool_address);
        end_epoch();

        // Set bonus rewards to 50% (so we can see difference)
        weighted_staking_reward::update_base_share(aptos_framework, 5000);

        // Delegator1 joins bucket 0 (1x multiplier) with all shares
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator1, pool_address, 0, all_shares);

        // Delegator2 joins bucket 2 (4x multiplier) with all shares
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator2, pool_address, 2, all_shares);

        // Fast forward and generate rewards
        fast_forward_seconds(BUCKET_0_DURATION_SECS); // Complete bucket 0's cycle
        end_epoch();

        // Claim rewards
        dp::claim_lockup_bonus(delegator1, pool_address);
        dp::claim_lockup_bonus(delegator2, pool_address);

        let balance1 = coin::balance<AptosCoin>(delegator1_address);
        let balance2 = coin::balance<AptosCoin>(delegator2_address);

        // Delegator2 (4x multiplier) should earn more than delegator1 (1x multiplier)
        assert!(balance2 > balance1, 1);
        // Roughly balance2 should be ~4x balance1 (weighted by multiplier)
        assert!(balance2 > balance1 * 3, 2); // At least 3x
    }

    #[test(aptos_framework = @0x1, validator = @0x123, delegator = @0x234)]
    #[expected_failure(abort_code = 0x30020, location = aptos_framework::delegation_pool)]
    public entry fun test_cannot_add_stake_with_bucket_position(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer
    ) {
        initialize_for_test(aptos_framework);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // Setup validator with initial stake
        let pool_address = setup_validator(validator, 1000 * ONE_APT);

        // Delegator adds stake
        let stake_amount = 1000 * ONE_APT;
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount);

        stake::join_validator_set(validator, pool_address);
        end_epoch();

        // Join bucket
        // Join with all shares (100000000 staked)
        let all_shares = (1000 * ONE_APT as u128);
        dp::join_lockup_bucket(delegator, pool_address, 0, all_shares);

        // Try to add more stake - should fail
        stake::mint(delegator, stake_amount);
        dp::add_stake(delegator, pool_address, stake_amount); // Should abort
    }
}
