/**
* Lockup-Weighted Bonus Reward Layer - Auto-Renewal Model
*
* ACTIVATION STEPS (Post-Deployment):
*
* 1. Initialize Global Config (Aptos Framework):
*    aptos move run --function-id 0x1::weighted_staking_reward::initialize
*    - Sets base_share_bps = 10000 (100% base, 0% bonus - shadow mode)
*    - Initializes 4 bucket configs with fixed durations
*
* 2. Enable Bonus Pool for Delegation Pool (Pool Owner):
*    aptos move run --function-id 0x1::delegation_pool::enable_lockup_rewards \
*      --args address:<POOL_ADDRESS>
*    - Initializes BonusPoolState for the delegation pool
*    - Allows users to join lockup buckets
*
* 3. Users Join Lockup Buckets:
*    aptos move run --function-id 0x1::delegation_pool::join_lockup_bucket \
*      --args address:<POOL_ADDRESS> u64:<BUCKET_ID> u128:<SHARES_AMOUNT>
*    - BUCKET_ID: 0=15d, 1=30d, 2=60d, 3=90d
*    - User starts earning at bucket's multiplier rate with auto-renewal
*
* 4. Governance Activates Bonus Rewards (Gradual Rollout):
*    # Month 1: 10% bonus
*    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
*      --args u64:9000
*    # Month 2: 20% bonus
*    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
*      --args u64:8000
*    # Month 3: 25% bonus (target)
*    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
*      --args u64:7500
*
* 5. Optional - Adjust Bucket Multipliers:
*    aptos move run --function-id 0x1::weighted_staking_reward::update_bucket_multipliers \
*      --args 'vector<u64>:[10000,20000,40000,60000]'
*
* ================================================================================================
*
* This module implements an auto-renewing lockup-weighted bonus reward mechanism on top of the
* existing delegation pool staking model. Total staking rewards are NOT increased; instead, they
* are reallocated between base rewards (distributed pro-rata to all stakers) and bonus rewards
* (distributed only to lockup participants, weighted by lockup duration).
*
* Key Design Features:
* - O(1) user operations via accumulator-based accounting
* - O(K) distribution where K is constant number of buckets (4)
* - No per-epoch user iteration required
* - Uses delegation pool shares as stake unit
* - Auto-renewing lockup cycles
* - Bonus rewards paid directly to user wallet (not converted to shares)
*
* Economic Model:
* - R_total = total rewards per distribution round
* - R_base = R_total * base_share_bps / 10000 (distributed via normal delegation pool)
* - R_bonus = R_total - R_base (distributed across lockup buckets)
*
* Lockup Buckets (Auto-Renewing):
* - 4 buckets with fixed durations: 15d, 30d, 60d, 90d
* - Reward multipliers: 1.0x, 2.0x, 4.0x, 6.0x (governance-tunable)
* - Cycles auto-renew indefinitely from lock_start_secs
* - Bonus allocation: R_i = R_bonus * (S[i] * m[i]) / sum(S[j] * m[j])
* - Within-bucket distribution via accumulator: acc += R_i / S[i]
*
* Early Exit & Claiming:
* - Can claim complete cycle rewards anytime
* - Exit before complete cycle: incomplete cycle rewards are BURNED (not redistributed)
* - Bucket upgrades (shorter → longer): keep cycle progress, no penalty
* - Bucket downgrades (longer → shorter): burn incomplete cycle, restart with fresh cycle
* - Base rewards via delegation pool are unaffected
*
* Security:
* - Cannot unlock() below bucket commitment (prevents earning without staking)
* - Must exit_bucket() before adding new stake to prevent cycle manipulation
*
* Upgrade Strategy:
* - Initial deployment with base_share_bps = 10000 (100% base, 0% bonus)
* - Gradual rollout by decreasing base_share_bps over time
*/
module aptos_framework::weighted_staking_reward {
    use std::error;
    use std::signer;

    use aptos_std::big_ordered_map::{Self, BigOrderedMap};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    // Friend declaration for delegation_pool to call sync_bonus_rewards
    friend aptos_framework::delegation_pool;

    // ===========================================================================================
    // Error codes
    // ===========================================================================================

    /// Bonus pool has not been initialized for this delegation pool
    const EBONUS_POOL_NOT_INITIALIZED: u64 = 1;

    /// Bonus pool already initialized for this delegation pool
    const EBONUS_POOL_ALREADY_INITIALIZED: u64 = 2;

    /// Invalid bucket ID
    const EINVALID_BUCKET_ID: u64 = 3;

    /// Position does not exist for this user
    const EPOSITION_NOT_FOUND: u64 = 4;

    /// Position already exists for this user
    const EPOSITION_ALREADY_EXISTS: u64 = 5;

    /// Cannot exit before maturity without penalty
    const EEARLY_EXIT: u64 = 6;

    /// Invalid configuration parameter
    const EINVALID_CONFIG: u64 = 7;

    /// Invalid base share bps (must be 0-10000)
    const EINVALID_BASE_SHARE_BPS: u64 = 8;

    /// Cannot extend to same or shorter bucket
    const EINVALID_LOCKUP_EXTENSION: u64 = 9;

    /// Shares amount must be greater than zero
    const EZERO_SHARES: u64 = 10;

    /// Global config not initialized
    const ECONFIG_NOT_INITIALIZED: u64 = 11;

    /// New bucket must have longer duration than current bucket
    const ENEW_BUCKET_NOT_LONGER: u64 = 12;

    /// Position has not matured yet
    const ENOT_MATURED: u64 = 13;

    /// Must be upgrading to a longer bucket
    const EMUST_UPGRADE: u64 = 14;

    /// Must be downgrading to a shorter bucket
    const EMUST_DOWNGRADE: u64 = 15;

    /// Cannot unlock stake while having bucket position (must exit bucket first)
    const ECANNOT_UNLOCK_WITH_BUCKET_POSITION: u64 = 16;

    /// Cannot add stake while having bucket position (must exit bucket first)
    const ECANNOT_ADD_STAKE_WITH_BUCKET_POSITION: u64 = 17;

    /// Overflow detected in weight calculation (shares * multiplier)
    const EOVERFLOW_WEIGHT_CALCULATION: u64 = 18;

    /// Overflow detected in reward calculation (shares * accumulator)
    const EOVERFLOW_REWARD_CALCULATION: u64 = 19;

    /// Insufficient bonus coins in bucket accounting
    const EINSUFFICIENT_BONUS_COINS: u64 = 20;

    // ===========================================================================================
    // Constants
    // ===========================================================================================

    /// Number of lockup buckets (15d, 30d, 60d, 90d)
    const NUM_BUCKETS: u64 = 4;

    /// Fixed bucket durations in seconds
    const BUCKET_0_DURATION_SECS: u64 = 1296000; // 15 days = 15 * 24 * 60 * 60
    const BUCKET_1_DURATION_SECS: u64 = 2592000; // 30 days = 30 * 24 * 60 * 60
    const BUCKET_2_DURATION_SECS: u64 = 5184000; // 60 days = 60 * 24 * 60 * 60
    const BUCKET_3_DURATION_SECS: u64 = 7776000; // 90 days = 90 * 24 * 60 * 60

    /// Fixed bucket multipliers in basis points (10000 = 1.0x)
    const BUCKET_0_MULTIPLIER_BPS: u64 = 10000; // 1.0x for 15 days
    const BUCKET_1_MULTIPLIER_BPS: u64 = 20000; // 2.0x for 30 days
    const BUCKET_2_MULTIPLIER_BPS: u64 = 40000; // 4.0x for 60 days
    const BUCKET_3_MULTIPLIER_BPS: u64 = 60000; // 6.0x for 90 days

    /// Maximum base share in basis points (100%)
    const MAX_BASE_SHARE_BPS: u64 = 10000;

    /// Default base share (100% to base, 0% to bonus) for initial deployment
    const DEFAULT_BASE_SHARE_BPS: u64 = 10000;

    /// Scaling factor for fixed-point accumulator arithmetic (10^18)
    const ACCUMULATOR_SCALE: u128 = 1000000000000000000;

    /// Maximum multiplier for a bucket (10x)
    const MAX_BUCKET_MULTIPLIER: u64 = 100000; // 10.0x in basis points (10000 = 1x)

    // ===========================================================================================
    // Data Structures
    // ===========================================================================================

    /// Global configuration for the bonus reward system (stored at @aptos_framework)
    struct BonusConfig has key {
        /// Base share in basis points (0-10000). base_rewards = total_rewards * base_share_bps / 10000
        base_share_bps: u64,
        /// Configuration for each bucket (7 buckets)
        buckets: vector<BucketConfig>
    }

    /// Configuration for a single lockup bucket
    struct BucketConfig has store, copy, drop {
        /// Multiplier in basis points (10000 = 1.0x, 20000 = 2.0x, etc.)
        /// Used for weighting: effective_weight = shares * multiplier
        multiplier_bps: u64
    }

    /// A user's position in the weighted bonus pool
    /// Auto-upgrades to current bucket params at cycle boundaries
    struct Position has store, copy, drop {
        /// Bucket ID (0-3) - user's choice of lockup preset
        bucket_id: u64,
        /// Base delegation pool shares (principal)
        shares: u128,
        /// Multiplier locked when joined/upgraded (can auto-upgrade at cycle boundary)
        multiplier_bps: u64,
        /// Duration locked when joined/upgraded (can auto-upgrade at cycle boundary)
        duration_secs: u64,
        /// Reward debt for accumulator math: debt = weighted_shares * acc_at_last_settlement
        debt: u128,
        /// Timestamp when current cycle started (reset on duration change)
        lock_start_secs: u64
    }

    /// Bonus pool state for a delegation pool (stored at pool address)
    /// Single shared pool where all users participate with different weights
    struct BonusPoolState has key {
        /// Pool address this bonus pool is associated with
        pool_address: address,
        /// Total weighted shares across all positions
        /// weighted_shares = sum(position.shares * position.multiplier_bps / 10000)
        total_weighted_shares: u128,
        /// Global accumulator for bonus rewards per weighted share (scaled by ACCUMULATOR_SCALE)
        acc_bonus_per_weighted_share: u128,
        /// Total bonus coins in the pool (numeric accounting)
        total_bonus_coins: u64,
        /// User positions (address -> Position)
        /// Using BigOrderedMap for gas efficiency
        positions: BigOrderedMap<address, Position>,
        /// Shared coin store for all bonus rewards
        /// Extracted from stake pool's active balance during sync
        bonus_coins: Coin<AptosCoin>
    }

    // ===========================================================================================
    // Events
    // ===========================================================================================

    #[event]
    /// Emitted when global bonus config is initialized
    struct ConfigInitialized has drop, store {
        base_share_bps: u64,
    }

    #[event]
    /// Emitted when base share percentage is updated
    struct BaseShareUpdated has drop, store {
        old_base_share_bps: u64,
        new_base_share_bps: u64,
    }

    #[event]
    /// Emitted when bucket multipliers are updated
    struct BucketMultipliersUpdated has drop, store {
        new_multipliers: vector<u64>,
    }

    #[event]
    /// Emitted when a bonus pool is initialized for a delegation pool
    struct BonusPoolInitialized has drop, store {
        pool_address: address,
    }

    #[event]
    /// Emitted when bonus rewards are distributed
    struct BonusRewardsDistributed has drop, store {
        pool_address: address,
        total_rewards: u64,
        base_rewards: u64,
        bonus_rewards: u64,
        bonus_returned_to_base: u64,
    }

    #[event]
    /// Emitted when a user joins a lockup bucket
    struct JoinedBucket has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        shares: u128,
        lock_start_secs: u64,
    }

    #[event]
    /// Emitted when a user upgrades to a longer bucket
    struct UpgradedBucket has drop, store {
        pool_address: address,
        user: address,
        old_bucket_id: u64,
        new_bucket_id: u64,
        lock_start_secs: u64,  // Preserved from old position
    }

    #[event]
    /// Emitted when a user downgrades to a shorter bucket
    struct DowngradedBucket has drop, store {
        pool_address: address,
        user: address,
        old_bucket_id: u64,
        new_bucket_id: u64,
        bonus_claimed: u64,      // Complete cycle rewards
        bonus_burned: u64,       // Incomplete cycle rewards burned
        new_lock_start_secs: u64 // Fresh start for new bucket
    }

    #[event]
    /// Emitted when a user claims bonus rewards
    struct BonusClaimed has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        amount: u64,
        complete_cycles: u64,    // Number of complete cycles claimed
    }

    #[event]
    /// Emitted when a user exits a bucket
    struct ExitedBucket has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        shares: u128,
        bonus_claimed: u64,      // Complete cycle rewards
        bonus_burned: u64,       // Incomplete cycle rewards (burned, not redistributed)
        complete_cycles: u64,    // Number of complete cycles
        partial_cycle_progress: u64, // Progress in incomplete cycle (seconds)
    }

    #[event]
    /// Emitted when position auto-upgrades to current bucket params
    struct BucketParamsUpgraded has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        old_multiplier: u128,    // For tracking purposes
        new_multiplier: u64,
        new_duration: u64,
    }

    // ===========================================================================================
    // Initialization Functions
    // ===========================================================================================

    /// Initialize global bonus config with default values (governance only)
    /// Initial config: 100% base, 0% bonus, with 4 buckets defined but inactive
    public entry fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<BonusConfig>(@aptos_framework),
            error::already_exists(ECONFIG_NOT_INITIALIZED)
        );

        // Initialize with 4 buckets with fixed durations and multipliers
        let buckets = vector[
            BucketConfig { multiplier_bps: BUCKET_0_MULTIPLIER_BPS }, // 15d, 1.0x
            BucketConfig { multiplier_bps: BUCKET_1_MULTIPLIER_BPS }, // 30d, 2.0x
            BucketConfig { multiplier_bps: BUCKET_2_MULTIPLIER_BPS }, // 60d, 4.0x
            BucketConfig { multiplier_bps: BUCKET_3_MULTIPLIER_BPS }  // 90d, 6.0x
        ];

        move_to(
            aptos_framework,
            BonusConfig { base_share_bps: DEFAULT_BASE_SHARE_BPS, buckets }
        );

        event::emit(ConfigInitialized {
            base_share_bps: DEFAULT_BASE_SHARE_BPS,
        });
    }

    /// Initialize bonus pool for a delegation pool (called by pool owner or governance)
    /// This enables bonus reward tracking for the pool
    public entry fun initialize_bonus_pool(account: &signer) {
        let pool_address = signer::address_of(account);
        assert!(
            !exists<BonusPoolState>(pool_address),
            error::already_exists(EBONUS_POOL_ALREADY_INITIALIZED)
        );
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );

        // Initialize single shared pool state
        move_to(
            account,
            BonusPoolState {
                pool_address,
                total_weighted_shares: 0,
                acc_bonus_per_weighted_share: 0,
                total_bonus_coins: 0,
                positions: big_ordered_map::new(),
                bonus_coins: coin::zero<AptosCoin>()
            }
        );

        event::emit(BonusPoolInitialized {
            pool_address,
        });
    }

    // ===========================================================================================
    // Governance Configuration Functions
    // ===========================================================================================

    /// Update base share percentage (governance only)
    /// base_share_bps: 0-10000 (0% to 100%)
    public entry fun update_base_share(
        aptos_framework: &signer, new_base_share_bps: u64
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            new_base_share_bps <= MAX_BASE_SHARE_BPS,
            error::invalid_argument(EINVALID_BASE_SHARE_BPS)
        );

        let config = &mut BonusConfig[@aptos_framework];
        let old_base_share_bps = config.base_share_bps;
        config.base_share_bps = new_base_share_bps;

        event::emit(BaseShareUpdated {
            old_base_share_bps,
            new_base_share_bps,
        });
    }

    /// Update multiplier for a specific bucket (governance only)
    /// Update all bucket multipliers at once
    /// Multipliers must be provided for all 4 buckets
    public entry fun update_bucket_multipliers(
        aptos_framework: &signer, new_multipliers_bps: vector<u64>
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            new_multipliers_bps.length() == NUM_BUCKETS,
            error::invalid_argument(EINVALID_CONFIG)
        );

        // Validate all multipliers
        for (i in 0..NUM_BUCKETS) {
            let multiplier = new_multipliers_bps[i];
            assert!(
                multiplier > 0 && multiplier <= MAX_BUCKET_MULTIPLIER,
                error::invalid_argument(EINVALID_CONFIG)
            );
        };

        // Update all multipliers
        let config = &mut BonusConfig[@aptos_framework];
        for (i in 0..NUM_BUCKETS) {
            config.buckets[i].multiplier_bps = new_multipliers_bps[i];
        };

        event::emit(BucketMultipliersUpdated {
            new_multipliers: new_multipliers_bps,
        });
    }

    // ===========================================================================================
    // Helper Functions for Bucket Configuration
    // ===========================================================================================

    /// Get bucket duration in seconds
    inline fun get_bucket_duration_secs(bucket_id: u64): u64 {
        if (bucket_id == 0) {
            BUCKET_0_DURATION_SECS
        } else if (bucket_id == 1) {
            BUCKET_1_DURATION_SECS
        } else if (bucket_id == 2) {
            BUCKET_2_DURATION_SECS
        } else if (bucket_id == 3) {
            BUCKET_3_DURATION_SECS
        } else {
            abort error::invalid_argument(EINVALID_BUCKET_ID)
        }
    }

    /// Get bucket multiplier in basis points (reads from config)
    inline fun get_bucket_multiplier_bps(bucket_id: u64): u64 acquires BonusConfig {
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        assert!(bucket_id < NUM_BUCKETS, error::invalid_argument(EINVALID_BUCKET_ID));

        let config = borrow_global<BonusConfig>(@aptos_framework);
        config.buckets[bucket_id].multiplier_bps
    }

    /// Calculate complete cycles since lock start
    inline fun calculate_complete_cycles(
        lock_start_secs: u64,
        current_time: u64,
        cycle_duration: u64
    ): u64 {
        let elapsed = current_time - lock_start_secs;
        elapsed / cycle_duration
    }

    /// Calculate progress in current incomplete cycle (0 to cycle_duration-1)
    inline fun calculate_current_cycle_progress(
        lock_start_secs: u64,
        current_time: u64,
        cycle_duration: u64
    ): u64 {
        let elapsed = current_time - lock_start_secs;
        elapsed % cycle_duration
    }

    // ===========================================================================================
    // View Functions
    // ===========================================================================================

    #[view]
    /// Get current base share percentage
    public fun get_base_share_bps(): u64 {
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        BonusConfig[@aptos_framework].base_share_bps
    }

    #[view]
    /// Get bucket configuration (returns: duration_secs, multiplier_bps)
    public fun get_bucket_config(bucket_id: u64): (u64, u64) acquires BonusConfig {
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        assert!(bucket_id < NUM_BUCKETS, error::invalid_argument(EINVALID_BUCKET_ID));

        let duration_secs = get_bucket_duration_secs(bucket_id);
        let multiplier_bps = get_bucket_multiplier_bps(bucket_id);
        (duration_secs, multiplier_bps)
    }

    #[view]
    /// Check if bonus pool is initialized for a delegation pool
    public fun is_bonus_pool_initialized(pool_address: address): bool {
        exists<BonusPoolState>(pool_address)
    }

    #[view]
    /// Check if user has a position in any bucket
    public fun has_position(pool_address: address, user: address): bool acquires BonusPoolState {
        if (!exists<BonusPoolState>(pool_address)) {
            return false
        };

        let bonus_pool = &BonusPoolState[pool_address];
        bonus_pool.positions.contains(&user)
    }

    #[view]
    /// Get user's total bucket shares (needed for unlock security check)
    public fun get_total_bucket_shares(
        pool_address: address,
        user: address
    ): u128 acquires BonusPoolState {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &BonusPoolState[pool_address];
        if (!bonus_pool.positions.contains(&user)) {
            return 0
        };

        let position = bonus_pool.positions.borrow(&user);
        position.shares
    }

    #[view]
    /// Get user's position info (returns: bucket_id, shares, lock_start_secs, pending_bonus, complete_cycles)
    public fun get_position(pool_address: address, user: address): (u64, u128, u64, u64, u64) acquires BonusPoolState {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &BonusPoolState[pool_address];
        assert!(
            bonus_pool.positions.contains(&user),
            error::not_found(EPOSITION_NOT_FOUND)
        );

        let position = *bonus_pool.positions.borrow(&user);
        let pending_bonus = calculate_pending_bonus_internal(bonus_pool, &position);

        // Calculate complete cycles using position's locked duration
        let now = timestamp::now_seconds();
        let cycle_duration = position.duration_secs;  // Use locked duration
        let complete_cycles = calculate_complete_cycles(
            position.lock_start_secs,
            now,
            cycle_duration
        );

        (position.bucket_id, position.shares, position.lock_start_secs, pending_bonus, complete_cycles)
    }

    #[view]
    /// Get pool state (returns: total_weighted_shares, acc_bonus_per_weighted_share)
    /// Note: With single shared pool, all users share the same accumulator
    public fun get_pool_state(pool_address: address): (u128, u128) {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &BonusPoolState[pool_address];
        (bonus_pool.total_weighted_shares, bonus_pool.acc_bonus_per_weighted_share)
    }

    // ===========================================================================================
    // Core Reward Distribution Logic
    // ===========================================================================================

    /// Sync bonus rewards - called during delegation pool sync (friend function)
    /// Takes total_rewards, splits according to base_share_bps, and distributes bonus across buckets
    /// Returns (base_rewards, bonus_rewards_distributed)
    public(friend) fun sync_bonus_rewards(
        pool_address: address, total_rewards: u64
    ): (u64, u64) {
        if (!exists<BonusPoolState>(pool_address)
            || !exists<BonusConfig>(@aptos_framework)) {
            // If bonus pool not initialized, all rewards go to base
            return (total_rewards, 0)
        };

        let config = &BonusConfig[@aptos_framework];

        // Split rewards: base vs bonus
        let base_rewards =
            ((total_rewards as u128) * (config.base_share_bps as u128)
                / (MAX_BASE_SHARE_BPS as u128) as u64);
        let bonus_rewards = total_rewards - base_rewards;

        if (bonus_rewards == 0) {
            return (base_rewards, 0)
        };

        // Note: Bonus coins will be extracted and deposited via deposit_bonus_coins()
        // This function only calculates the split
        let actually_distributed = bonus_rewards;

        // If no users have staked (total_weighted_shares == 0), add undistributed back to base
        let bonus_returned_to_base = 0u64;
        let bonus_pool = &BonusPoolState[pool_address];
        if (bonus_pool.total_weighted_shares == 0) {
            base_rewards += bonus_rewards;
            bonus_returned_to_base = bonus_rewards;
            actually_distributed = 0;
        };

        event::emit(BonusRewardsDistributed {
            pool_address,
            total_rewards,
            base_rewards,
            bonus_rewards: actually_distributed,
            bonus_returned_to_base,
        });

        (base_rewards, actually_distributed)
    }

    /// Deposit bonus coins into the pool and distribute across buckets (friend function)
    /// Called by delegation_pool after extracting bonus coins from stake pool
    public(friend) fun deposit_bonus_coins(
        pool_address: address, bonus_coins: Coin<AptosCoin>
    ) acquires BonusPoolState {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );

        distribute_bonus_rewards(pool_address, bonus_coins);
    }

    /// Extract bonus payment from pool for a user (friend function)
    /// Called by delegation_pool when paying out bonus rewards
    /// Extracts coins from shared bonus pool
    public(friend) fun extract_bonus_payment(
        pool_address: address, amount: u64
    ): Coin<AptosCoin> acquires BonusPoolState {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &mut BonusPoolState[pool_address];

        // Validate we have enough coins
        assert!(
            bonus_pool.total_bonus_coins >= amount,
            error::invalid_state(EINSUFFICIENT_BONUS_COINS)
        );

        // Update accounting
        bonus_pool.total_bonus_coins -= amount;

        // Extract coins from shared pool
        coin::extract(&mut bonus_pool.bonus_coins, amount)
    }

    /// Internal: Distribute bonus rewards across all buckets weighted by (shares * multiplier)
    /// Returns: amount actually distributed (may be 0 if no buckets have shares)
    /// Distribute bonus rewards to all users in the shared pool
    /// Much simpler than bucket approach - single O(1) accumulator update!
    fun distribute_bonus_rewards(
        pool_address: address, bonus_coins: Coin<AptosCoin>
    ): u64 acquires BonusPoolState {
        let bonus_pool = &mut BonusPoolState[pool_address];
        let bonus_rewards = coin::value(&bonus_coins);

        // Merge bonus coins into the shared pool
        coin::merge(&mut bonus_pool.bonus_coins, bonus_coins);

        // If no one is staked, return 0 (undistributed)
        if (bonus_pool.total_weighted_shares == 0) {
            return 0
        };

        // Update global accumulator - this is it! O(1) distribution to all users
        // acc += bonus_rewards / total_weighted_shares
        // Each user will get: weighted_shares * acc - debt
        let acc_increment = (bonus_rewards as u128) * ACCUMULATOR_SCALE
            / bonus_pool.total_weighted_shares;
        bonus_pool.acc_bonus_per_weighted_share += acc_increment;

        // Update coin accounting
        bonus_pool.total_bonus_coins += bonus_rewards;

        bonus_rewards // Return full amount as distributed
    }

    // ===========================================================================================
    // User Position Management
    // ===========================================================================================

    /// Join a lockup bucket with delegation shares (auto-renewing cycles start immediately)
    /// user: the delegator
    /// pool_address: delegation pool address
    /// bucket_id: which bucket to join (0-3 for 15d/30d/60d/90d)
    /// shares: amount of delegation pool shares to lock
    public fun join_bucket(
        user: &signer,
        pool_address: address,
        bucket_id: u64,
        shares: u128
    ) acquires BonusPoolState, BonusConfig {
        let user_addr = signer::address_of(user);

        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        assert!(bucket_id < NUM_BUCKETS, error::invalid_argument(EINVALID_BUCKET_ID));
        assert!(shares > 0, error::invalid_argument(EZERO_SHARES));

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            !bonus_pool.positions.contains(&user_addr),
            error::already_exists(EPOSITION_ALREADY_EXISTS)
        );

        // Lock current bucket parameters (can auto-upgrade at cycle boundaries)
        let multiplier_bps = get_bucket_multiplier_bps(bucket_id);
        let duration_secs = get_bucket_duration_secs(bucket_id);

        // Calculate weighted shares
        let weighted_shares = calculate_weighted_shares(shares, bucket_id);

        // Set lock start time to now (first cycle begins immediately)
        let now = timestamp::now_seconds();

        // Get current global accumulator value
        let current_acc = bonus_pool.acc_bonus_per_weighted_share;

        // Create position - locks current multiplier and duration
        let position = Position {
            bucket_id,
            shares,
            multiplier_bps,
            duration_secs,
            debt: weighted_shares * current_acc / ACCUMULATOR_SCALE,
            lock_start_secs: now
        };

        // Add position and update global weighted shares
        bonus_pool.positions.add(user_addr, position);
        bonus_pool.total_weighted_shares += weighted_shares;

        event::emit(JoinedBucket {
            pool_address,
            user: user_addr,
            bucket_id,
            shares,
            lock_start_secs: now,
        });
    }

    /// Upgrade to a longer bucket (keeps lock_start_secs, no penalty)
    /// User commits to longer lockup, preserving cycle progress
    /// No rewards are claimed or burned during upgrade
    public fun upgrade_bucket(
        user: &signer, pool_address: address, new_bucket_id: u64
    ) acquires BonusPoolState, BonusConfig {
        let user_addr = signer::address_of(user);

        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        assert!(new_bucket_id < NUM_BUCKETS, error::invalid_argument(EINVALID_BUCKET_ID));

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            bonus_pool.positions.contains(&user_addr),
            error::not_found(EPOSITION_NOT_FOUND)
        );

        let old_position = bonus_pool.positions.remove(&user_addr);

        // Verify new bucket is longer
        let old_duration = old_position.duration_secs;
        let new_duration = get_bucket_duration_secs(new_bucket_id);
        assert!(
            new_duration > old_duration,
            error::invalid_argument(EMUST_UPGRADE)
        );

        // Get new bucket's multiplier
        let new_multiplier = get_bucket_multiplier_bps(new_bucket_id);

        // Calculate old and new weighted shares
        let old_weighted = get_weighted_shares(&old_position);
        let new_weighted = (old_position.shares * (new_multiplier as u128)) / 10000;

        // Create new position, preserving lock_start_secs
        let new_position = Position {
            bucket_id: new_bucket_id,
            shares: old_position.shares,
            multiplier_bps: new_multiplier,
            duration_secs: new_duration,
            debt: new_weighted * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE,
            lock_start_secs: old_position.lock_start_secs  // Keep original start time
        };

        // Update global weighted shares
        if (new_weighted >= old_weighted) {
            bonus_pool.total_weighted_shares += (new_weighted - old_weighted);
        } else {
            bonus_pool.total_weighted_shares -= (old_weighted - new_weighted);
        };

        bonus_pool.positions.add(user_addr, new_position);

        event::emit(UpgradedBucket {
            pool_address,
            user: user_addr,
            old_bucket_id: old_position.bucket_id,
            new_bucket_id,
            lock_start_secs: old_position.lock_start_secs,
        });
    }

    /// Downgrade to a shorter bucket (burns incomplete cycle, restarts with new lock_start_secs)
    /// User reduces commitment, so incomplete cycle rewards are burned
    /// Returns: (complete_cycle_rewards, burned_rewards)
    public fun downgrade_bucket(
        user: &signer, pool_address: address, new_bucket_id: u64
    ): (u64, u64) acquires BonusPoolState, BonusConfig {
        let user_addr = signer::address_of(user);

        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        assert!(new_bucket_id < NUM_BUCKETS, error::invalid_argument(EINVALID_BUCKET_ID));

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            bonus_pool.positions.contains(&user_addr),
            error::not_found(EPOSITION_NOT_FOUND)
        );

        let old_position = bonus_pool.positions.remove(&user_addr);

        // Verify new bucket is shorter
        let old_duration = old_position.duration_secs;
        let new_duration = get_bucket_duration_secs(new_bucket_id);
        assert!(
            new_duration < old_duration,
            error::invalid_argument(EMUST_DOWNGRADE)
        );

        // Calculate complete vs incomplete cycle rewards
        let now = timestamp::now_seconds();
        let total_pending = calculate_pending_bonus_internal(bonus_pool, &old_position);

        let complete_cycles = calculate_complete_cycles(
            old_position.lock_start_secs,
            now,
            old_duration
        );

        // For simplicity: if any incomplete cycle, burn ALL pending rewards
        // More sophisticated: calculate proportional burn based on cycle progress
        let (complete_rewards, burned_rewards) = if (calculate_current_cycle_progress(
            old_position.lock_start_secs,
            now,
            old_duration
        ) == 0) {
            // Exactly at cycle boundary - no burn
            (total_pending, 0)
        } else {
            // Has incomplete cycle - burn all pending (conservative approach)
            // TODO: Could implement proportional burn based on complete_cycles / total_time
            (0, total_pending)
        };

        // Get new bucket's multiplier
        let new_multiplier = get_bucket_multiplier_bps(new_bucket_id);

        // Calculate old and new weighted shares
        let old_weighted = get_weighted_shares(&old_position);
        let new_weighted = (old_position.shares * (new_multiplier as u128)) / 10000;

        // Create new position with fresh lock_start_secs
        let new_position = Position {
            bucket_id: new_bucket_id,
            shares: old_position.shares,
            multiplier_bps: new_multiplier,
            duration_secs: new_duration,
            debt: new_weighted * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE,
            lock_start_secs: now  // Fresh start for new bucket
        };

        // Update global weighted shares
        if (new_weighted >= old_weighted) {
            bonus_pool.total_weighted_shares += (new_weighted - old_weighted);
        } else {
            bonus_pool.total_weighted_shares -= (old_weighted - new_weighted);
        };

        bonus_pool.positions.add(user_addr, new_position);

        event::emit(DowngradedBucket {
            pool_address,
            user: user_addr,
            old_bucket_id: old_position.bucket_id,
            new_bucket_id,
            bonus_claimed: complete_rewards,
            bonus_burned: burned_rewards,
            new_lock_start_secs: now,
        });

        (complete_rewards, burned_rewards)
    }

    /// Claim accumulated bonus rewards for all complete cycles
    /// User can claim anytime, but only complete cycle rewards are paid
    /// Incomplete cycle rewards remain pending until cycle completes
    /// Returns amount of bonus rewards claimed
    public fun claim_bonus(user: &signer, pool_address: address): u64 acquires BonusPoolState, BonusConfig {
        let user_addr = signer::address_of(user);

        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            bonus_pool.positions.contains(&user_addr),
            error::not_found(EPOSITION_NOT_FOUND)
        );

        // Remove position temporarily for auto-upgrade
        let pos = bonus_pool.positions.remove(&user_addr);

        // === Inline auto-upgrade logic to avoid borrow conflicts ===
        let current_multiplier = get_bucket_multiplier_bps(pos.bucket_id);
        let current_duration = get_bucket_duration_secs(pos.bucket_id);
        let params_changed =
            current_multiplier != pos.multiplier_bps ||
            current_duration != pos.duration_secs;

        if (params_changed) {
            let now = timestamp::now_seconds();
            let elapsed = now - pos.lock_start_secs;
            let complete_cycles = elapsed / pos.duration_secs;
            let at_boundary = (elapsed % pos.duration_secs) == 0;

            if (complete_cycles > 0 && at_boundary) {
                // Auto-upgrade!
                let old_weighted = get_weighted_shares(&pos);
                pos.multiplier_bps = current_multiplier;
                pos.duration_secs = current_duration;
                let new_weighted = get_weighted_shares(&pos);

                // Update global total_weighted_shares
                if (new_weighted >= old_weighted) {
                    bonus_pool.total_weighted_shares += (new_weighted - old_weighted);
                } else {
                    bonus_pool.total_weighted_shares -= (old_weighted - new_weighted);
                };

                // Reset debt and lock_start_secs
                pos.debt = new_weighted * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE;
                pos.lock_start_secs = now;

                // Emit event
                event::emit(BucketParamsUpgraded {
                    pool_address: bonus_pool.pool_address,
                    user: user_addr,
                    bucket_id: pos.bucket_id,
                    old_multiplier: old_weighted,
                    new_multiplier: current_multiplier,
                    new_duration: current_duration,
                });
            };
        };
        // === End inline auto-upgrade logic ===

        // Calculate complete cycles using position's locked duration
        let now = timestamp::now_seconds();
        let complete_cycles = calculate_complete_cycles(
            pos.lock_start_secs,
            now,
            pos.duration_secs  // Use locked duration
        );

        // For auto-renewal: user can always claim all accumulated rewards
        // The "cycle" concept only matters for exit penalty, not for claiming
        // So we simplify: claim all pending rewards
        let pending = calculate_pending_bonus_internal(bonus_pool, &pos);

        if (pending == 0) {
            // Add position back and return
            bonus_pool.positions.add(user_addr, pos);
            return 0
        };

        // Update debt to current accumulator value (using dynamic weighted_shares)
        let weighted_shares = get_weighted_shares(&pos);
        pos.debt =
            weighted_shares * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE;

        // Add position back
        bonus_pool.positions.add(user_addr, pos);

        event::emit(BonusClaimed {
            pool_address,
            user: user_addr,
            bucket_id: pos.bucket_id,
            amount: pending,
            complete_cycles,
        });

        pending
    }

    /// Exit position and withdraw from bucket
    /// Complete cycle rewards: claimed
    /// Incomplete cycle rewards: BURNED (not redistributed)
    /// Returns (bonus_claimed, bonus_burned, shares)
    public fun exit_bucket(user: &signer, pool_address: address): (u64, u64, u128) acquires BonusPoolState, BonusConfig {
        let user_addr = signer::address_of(user);

        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            bonus_pool.positions.contains(&user_addr),
            error::not_found(EPOSITION_NOT_FOUND)
        );

        // Remove position temporarily for auto-upgrade
        let pos = bonus_pool.positions.remove(&user_addr);

        // === Inline auto-upgrade logic to avoid borrow conflicts ===
        let current_multiplier = get_bucket_multiplier_bps(pos.bucket_id);
        let current_duration = get_bucket_duration_secs(pos.bucket_id);
        let params_changed =
            current_multiplier != pos.multiplier_bps ||
            current_duration != pos.duration_secs;

        if (params_changed) {
            let now = timestamp::now_seconds();
            let elapsed = now - pos.lock_start_secs;
            let complete_cycles = elapsed / pos.duration_secs;
            let at_boundary = (elapsed % pos.duration_secs) == 0;

            if (complete_cycles > 0 && at_boundary) {
                // Auto-upgrade!
                let old_weighted = get_weighted_shares(&pos);
                pos.multiplier_bps = current_multiplier;
                pos.duration_secs = current_duration;
                let new_weighted = get_weighted_shares(&pos);

                // Update global total_weighted_shares
                if (new_weighted >= old_weighted) {
                    bonus_pool.total_weighted_shares += (new_weighted - old_weighted);
                } else {
                    bonus_pool.total_weighted_shares -= (old_weighted - new_weighted);
                };

                // Reset debt and lock_start_secs
                pos.debt = new_weighted * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE;
                pos.lock_start_secs = now;

                // Emit event
                event::emit(BucketParamsUpgraded {
                    pool_address: bonus_pool.pool_address,
                    user: user_addr,
                    bucket_id: pos.bucket_id,
                    old_multiplier: old_weighted,
                    new_multiplier: current_multiplier,
                    new_duration: current_duration,
                });
            };
        };
        // === End inline auto-upgrade logic ===

        // Calculate cycle progress using position's locked duration
        let now = timestamp::now_seconds();
        let cycle_duration = pos.duration_secs;  // Use locked duration
        let complete_cycles = calculate_complete_cycles(
            pos.lock_start_secs,
            now,
            cycle_duration
        );
        let partial_cycle_progress = calculate_current_cycle_progress(
            pos.lock_start_secs,
            now,
            cycle_duration
        );

        // Calculate total pending bonus using global accumulator
        let total_pending = calculate_pending_bonus_internal(bonus_pool, &pos);

        // Determine claimed vs burned
        let (claimed, burned) = if (complete_cycles >= 1 && partial_cycle_progress == 0) {
            // Completed at least one full cycle and exactly at cycle boundary - no penalty
            (total_pending, 0)
        } else {
            // Either: (1) haven't completed any cycles, or (2) mid-cycle exit
            // Conservative approach: burn all pending rewards
            // This includes the edge case where user exits immediately (complete_cycles == 0)
            (0, total_pending)
        };

        // Update global weighted shares (remove user's weighted shares, calculated dynamically)
        let weighted_shares = get_weighted_shares(&pos);
        bonus_pool.total_weighted_shares -= weighted_shares;

        // Note: burned rewards are NOT redistributed - they simply remain
        // unclaimed in the accumulator, effectively burning them from circulation

        event::emit(ExitedBucket {
            pool_address,
            user: user_addr,
            bucket_id: pos.bucket_id,
            shares: pos.shares,
            bonus_claimed: claimed,
            bonus_burned: burned,
            complete_cycles,
            partial_cycle_progress,
        });

        (claimed, burned, pos.shares)
    }

    // ===========================================================================================
    // Helper Functions
    // ===========================================================================================

    /// Calculate weighted shares from position's locked multiplier
    /// Uses position.multiplier_bps (not dynamic lookup) for consistency
    inline fun get_weighted_shares(position: &Position): u128 {
        position.shares * (position.multiplier_bps as u128) / 10000
    }

    /// Calculate weighted shares from bucket_id and shares (for join/exit)
    /// Note: Inline functions inherit acquires from callers, no need to specify
    inline fun calculate_weighted_shares(shares: u128, bucket_id: u64): u128 {
        let multiplier_bps = get_bucket_multiplier_bps(bucket_id);
        // OVERFLOW PROTECTION: Check multiplication won't overflow
        assert!(
            shares <= 340282366920938463463374607431768211455u128 / (multiplier_bps as u128),
            error::invalid_state(EOVERFLOW_WEIGHT_CALCULATION)
        );
        shares * (multiplier_bps as u128) / 10000
    }

    /// Auto-upgrade position to current bucket params if at cycle boundary
    /// This ensures users benefit from governance improvements automatically
    fun auto_update_bucket_params_if_needed(
        user_addr: address,
        position: &mut Position,
        bonus_pool: &mut BonusPoolState
    ) acquires BonusConfig {
        // Get current bucket params
        let current_multiplier = get_bucket_multiplier_bps(position.bucket_id);
        let current_duration = get_bucket_duration_secs(position.bucket_id);

        // Check if params changed
        let params_changed =
            current_multiplier != position.multiplier_bps ||
            current_duration != position.duration_secs;

        if (!params_changed) {
            return  // No changes, skip
        };

        // Check if at cycle boundary (using CURRENT duration)
        let now = timestamp::now_seconds();
        let elapsed = now - position.lock_start_secs;
        let complete_cycles = elapsed / position.duration_secs;
        let at_boundary = (elapsed % position.duration_secs) == 0;

        if (complete_cycles == 0 || !at_boundary) {
            return  // Not at cycle boundary yet, wait
        };

        // === Auto-upgrade! ===

        // 1. Calculate old weighted shares
        let old_weighted = get_weighted_shares(position);

        // 2. Update to current params
        position.multiplier_bps = current_multiplier;
        position.duration_secs = current_duration;

        // 3. Calculate new weighted shares
        let new_weighted = get_weighted_shares(position);

        // 4. Update global total_weighted_shares
        if (new_weighted >= old_weighted) {
            bonus_pool.total_weighted_shares += (new_weighted - old_weighted);
        } else {
            bonus_pool.total_weighted_shares -= (old_weighted - new_weighted);
        };

        // 5. Reset debt (since weighted_shares changed)
        position.debt = new_weighted * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE;

        // 6. Reset lock_start_secs (new cycle starts now)
        position.lock_start_secs = now;

        // Event for tracking
        event::emit(BucketParamsUpgraded {
            pool_address: bonus_pool.pool_address,
            user: user_addr,
            bucket_id: position.bucket_id,
            old_multiplier: old_weighted, // Using as proxy
            new_multiplier: current_multiplier,
            new_duration: current_duration,
        });
    }

    /// Calculate pending bonus for a position using global accumulator
    /// Much simpler with single shared pool!
    inline fun calculate_pending_bonus_internal(
        bonus_pool: &BonusPoolState, position: &Position
    ): u64 {
        // Calculate weighted_shares dynamically (no storage cost!)
        let weighted_shares = get_weighted_shares(position);

        // pending = weighted_shares * (acc_now - acc_at_last) = weighted_shares * acc_now / SCALE - debt

        // OVERFLOW PROTECTION: Check multiplication won't overflow u128
        // weighted_shares <= u128::MAX / acc_bonus_per_weighted_share
        if (bonus_pool.acc_bonus_per_weighted_share > 0) {
            assert!(
                weighted_shares <= 340282366920938463463374607431768211455u128 / bonus_pool.acc_bonus_per_weighted_share,
                error::invalid_state(EOVERFLOW_REWARD_CALCULATION)
            );
        };

        let earned_total =
            weighted_shares * bonus_pool.acc_bonus_per_weighted_share / ACCUMULATOR_SCALE;
        if (earned_total > position.debt) {
            ((earned_total - position.debt) as u64)
        } else { 0 }
    }

    // ===========================================================================================
    // Test-only Functions
    // ===========================================================================================

    #[test_only]
    public fun test_initialize(aptos_framework: &signer) {
        initialize(aptos_framework);
    }

    #[test_only]
    public fun test_initialize_bonus_pool(account: &signer) {
        initialize_bonus_pool(account);
    }

    #[test_only]
    use aptos_framework::stake;

    #[test_only]
    public fun test_sync_bonus_rewards(
        pool_address: address, total_rewards: u64
    ): (u64, u64) acquires BonusPoolState, BonusConfig {
        let (base_rewards, bonus_rewards) = sync_bonus_rewards(pool_address, total_rewards);

        // For testing: mint and deposit bonus coins
        if (bonus_rewards > 0 && exists<BonusPoolState>(pool_address)) {
            let bonus_coins = stake::mint_coins_for_test(bonus_rewards);
            deposit_bonus_coins(pool_address, bonus_coins);
        };

        (base_rewards, bonus_rewards)
    }

    #[test_only]
    /// Test helper to deposit bonus coins (for unit tests)
    /// The test must mint the coins and call this to deposit them
    public fun test_deposit_bonus_coins(pool_address: address, bonus_coins: Coin<AptosCoin>) {
        deposit_bonus_coins(pool_address, bonus_coins);
    }
}
