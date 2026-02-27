/// Lockup-Weighted Bonus Reward Layer - Auto-Renewal Model
///
/// ACTIVATION STEPS (Post-Deployment):
///
/// 1. Initialize Global Config (Aptos Framework):
///    aptos move run --function-id 0x1::weighted_staking_reward::initialize
///    - Sets base_share_bps = 10000 (100% base, 0% bonus - shadow mode)
///    - Initializes bucket configs with caller-specified durations and multipliers
///
/// 2. Enable Bonus Pool for Delegation Pool (callable by anyone):
///    aptos move run --function-id 0x1::weighted_staking_reward::try_initialize_bonus_pool
///    - Initializes BonusPoolState for the delegation pool (no-op if already initialized)
///    - Allows users to join lockup buckets
///
/// 3. Users Join Lockup Buckets:
///    aptos move run --function-id 0x1::delegation_pool::join_lockup_bucket \
///      --args address:<POOL_ADDRESS> u64:<BUCKET_ID> u128:<SHARES_AMOUNT>
///    - BUCKET_ID: 0=15d, 1=30d, 2=60d, 3=90d
///    - User starts earning at bucket's multiplier rate with auto-renewal
///
/// 4. Governance Activates Bonus Rewards (Gradual Rollout):
///    # Month 1: 10% bonus
///    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
///      --args u64:9000
///    # Month 2: 20% bonus
///    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
///      --args u64:8000
///    # Month 3: 25% bonus (target)
///    aptos move run --function-id 0x1::weighted_staking_reward::update_base_share \
///      --args u64:7500
///
/// 5. Optional - Adjust Bucket Configs:
///    aptos move run --function-id 0x1::weighted_staking_reward::update_bucket_configs \
///      --args 'vector<u64>:[1296000,2592000,5184000,7776000]' \
///             'vector<u64>:[10000,20000,40000,60000]'
///
/// ================================================================================================
///
/// This module implements an auto-renewing lockup-weighted bonus reward mechanism on top of the
/// existing delegation pool staking model. Total staking rewards are NOT increased; instead, they
/// are reallocated between base rewards (distributed pro-rata to all stakers) and bonus rewards
/// (distributed only to lockup participants, weighted by lockup duration).
///
/// Key Design Features:
/// - O(1) user operations via accumulator-based accounting
/// - O(K) distribution where K is constant number of buckets (4)
/// - No per-epoch user iteration required
/// - Uses delegation pool shares as stake unit
/// - Auto-renewing lockup cycles
/// - Bonus rewards paid directly to user wallet (not converted to shares)
///
/// Economic Model:
/// - R_total = total rewards per distribution round
/// - R_base = R_total * base_share_bps / 10000 (distributed via normal delegation pool)
/// - R_bonus = R_total - R_base (distributed across lockup buckets)
///
/// Lockup Buckets (Auto-Renewing):
/// - 4 buckets with fixed durations: 15d, 30d, 60d, 90d
/// - Reward multipliers: 1.0x, 2.0x, 4.0x, 6.0x (governance-tunable)
/// - Cycles auto-renew indefinitely from lock_start_secs
/// - Bonus allocation: R_i = R_bonus * (S[i] * m[i]) / sum(S[j] * m[j])
/// - Within-bucket distribution via accumulator: acc += R_i / S[i]
///
/// Early Exit & Claiming:
/// - Can claim complete cycle rewards anytime
/// - Exit before complete cycle: incomplete cycle rewards are BURNED (not redistributed)
/// - Bucket upgrades (shorter → longer): keep cycle progress, no penalty
/// - Bucket downgrades (longer → shorter): burn incomplete cycle, restart with fresh cycle
/// - Base rewards via delegation pool are unaffected
///
/// Security:
/// - Cannot unlock() below bucket commitment (prevents earning without staking)
/// - Must exit_bucket() before adding new stake to prevent cycle manipulation
///
/// Upgrade Strategy:
/// - Initial deployment with base_share_bps = 10000 (100% base, 0% bonus)
/// - Gradual rollout by decreasing base_share_bps over time
module aptos_framework::weighted_staking_reward {
    use std::error;
    use std::signer;

    use aptos_std::big_ordered_map::{Self, BigOrderedMap};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    friend aptos_framework::delegation_pool;
    friend aptos_framework::stake;

    // ===========================================================================================
    // Error codes
    // ===========================================================================================

    /// Bonus pool has not been initialized for this delegation pool
    const EBONUS_POOL_NOT_INITIALIZED: u64 = 1;
    /// Invalid bucket ID
    const EINVALID_BUCKET_ID: u64 = 3;
    /// Position does not exist for this user
    const EPOSITION_NOT_FOUND: u64 = 4;
    /// Position already exists for this user
    const EPOSITION_ALREADY_EXISTS: u64 = 5;
    /// Invalid configuration parameter
    const EINVALID_CONFIG: u64 = 7;
    /// Invalid base share bps (must be 0-10000)
    const EINVALID_BASE_SHARE_BPS: u64 = 8;
    /// Shares amount must be greater than zero
    const EZERO_SHARES: u64 = 10;
    /// Global config not initialized
    const ECONFIG_NOT_INITIALIZED: u64 = 11;
    /// Must be upgrading to a longer bucket
    const EMUST_UPGRADE: u64 = 14;
    /// Must be downgrading to a shorter bucket
    const EMUST_DOWNGRADE: u64 = 15;
    /// Overflow detected in weight calculation (shares * multiplier)
    const EOVERFLOW_WEIGHT_CALCULATION: u64 = 17;
    /// Overflow detected in reward calculation (shares * accumulator)
    const EOVERFLOW_REWARD_CALCULATION: u64 = 18;
    /// Insufficient bonus coins in bucket accounting
    const EINSUFFICIENT_BONUS_COINS: u64 = 19;
    /// Bucket durations must be strictly increasing
    const EDURATIONS_NOT_INCREASING: u64 = 20;
    /// Cannot reduce the number of buckets (existing positions reference bucket IDs by index)
    const ECANNOT_REDUCE_BUCKET_COUNT: u64 = 22;
    /// Bucket config vectors must be non-empty and equal length
    const EINVALID_BUCKET_CONFIG_LENGTH: u64 = 23;
    /// Bucket multipliers must be strictly increasing (longer duration → higher multiplier)
    const EMULTIPLIERS_NOT_INCREASING: u64 = 24;

    /// Maximum base share in basis points (100%)
    const MAX_BASE_SHARE_BPS: u64 = 10000;
    /// Default base share (100% to base, 0% to bonus) for initial deployment
    const DEFAULT_BASE_SHARE_BPS: u64 = 10000;
    /// Scaling factor for fixed-point accumulator arithmetic (10^18)
    const ACCUMULATOR_SCALE: u128 = 1000000000000000000;
    /// Maximum multiplier for a bucket (10x in basis points)
    const MAX_BUCKET_MULTIPLIER: u64 = 100000;
    /// Maximum value of u128
    const MAX_U128: u128 = 340282366920938463463374607431768211455;
    /// Scaling factor between raw delegation pool shares and normalized shares stored in positions.
    /// raw_pool_shares = normalized_shares * SHARES_SCALING_FACTOR
    const SHARES_SCALING_FACTOR: u128 = 10000000000000000;

    // ===========================================================================================
    // Data Structures
    // ===========================================================================================

    /// Global configuration for the bonus reward system (stored at @aptos_framework)
    struct BonusConfig has key {
        /// Base share in basis points (0-10000). base_rewards = total_rewards * base_share_bps / 10000
        base_share_bps: u64,
        /// Configuration for each lockup bucket
        buckets: vector<BucketConfig>
    }

    /// Configuration for a single lockup bucket
    struct BucketConfig has store, copy, drop {
        /// Duration of one lockup cycle in seconds
        duration_secs: u64,
        /// Multiplier in basis points (10000 = 1.0x, 20000 = 2.0x, etc.)
        multiplier_bps: u64
    }

    /// A user's position in the weighted bonus pool.
    /// Bucket params (multiplier, duration) are always read from BonusConfig at runtime,
    /// so governance updates take effect immediately on the next user operation.
    struct Position has store, copy, drop {
        /// Bucket ID — index into BonusConfig.buckets
        bucket_id: u64,
        /// Base delegation pool shares (principal).
        /// Passed as normalized shares = raw_pool_shares / SHARES_SCALING_FACTOR
        /// (≈ coin amount in octas at join time; safe scale for u128 accumulator math).
        shares: u128,
        /// Reward debt for accumulator math: debt = weighted_shares * acc_at_last_settlement
        debt: u128,
        /// Timestamp when current cycle started (reset on bucket change or after claim)
        lock_start_secs: u64
    }

    /// Bonus pool state for a delegation pool (stored at pool address).
    /// Single shared pool where all users participate with different weights.
    struct BonusPoolState has key {
        /// Total weighted shares across all positions:
        /// total_weighted = sum(position.shares * bucket.multiplier_bps / 10000)
        total_weighted_shares: u128,
        /// Global accumulator for bonus rewards per weighted share (scaled by ACCUMULATOR_SCALE)
        acc_bonus_per_weighted_share: u128,
        /// User positions (address -> Position); BigOrderedMap for gas efficiency
        positions: BigOrderedMap<address, Position>,
        /// Shared coin store for all bonus rewards extracted from stake pool at sync time
        bonus_coins: Coin<AptosCoin>
    }

    // ===========================================================================================
    // Events
    // ===========================================================================================

    #[event]
    /// Emitted when base share percentage is updated (also emitted on initialization,
    /// with old_base_share_bps == DEFAULT_BASE_SHARE_BPS)
    struct BaseShareUpdated has drop, store {
        old_base_share_bps: u64,
        new_base_share_bps: u64
    }

    #[event]
    /// Emitted when bucket configs are updated
    struct BucketConfigsUpdated has drop, store {
        durations_secs: vector<u64>,
        multipliers_bps: vector<u64>
    }

    #[event]
    /// Emitted when a bonus pool is initialized for a delegation pool
    struct BonusPoolInitialized has drop, store {
        pool_address: address
    }

    #[event]
    /// Emitted when bonus rewards are distributed
    struct BonusRewardsDistributed has drop, store {
        pool_address: address,
        total_rewards: u64,
        base_rewards: u64,
        bonus_rewards: u64
    }

    #[event]
    /// Emitted when a user joins a lockup bucket
    struct JoinedBucket has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        shares: u128,
        lock_start_secs: u64
    }

    #[event]
    /// Emitted when a user upgrades to a longer bucket
    struct UpgradedBucket has drop, store {
        pool_address: address,
        user: address,
        old_bucket_id: u64,
        new_bucket_id: u64,
        lock_start_secs: u64
    }

    #[event]
    /// Emitted when a user claims bonus rewards
    struct BonusClaimed has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        amount: u64,
        complete_cycles: u64
    }

    #[event]
    /// Emitted when a user exits a bucket
    struct ExitedBucket has drop, store {
        pool_address: address,
        user: address,
        bucket_id: u64,
        shares: u128,
        bonus_claimed: u64,
        bonus_burned: u64,
        complete_cycles: u64,
        partial_cycle_progress: u64
    }

    // ===========================================================================================
    // Initialization
    // ===========================================================================================

    /// Initialize global bonus config (governance only).
    /// Caller specifies bucket durations and multipliers directly — no hardcoded defaults.
    /// durations_secs: strictly increasing list of lockup durations (one per bucket)
    /// multipliers_bps: reward multiplier for each bucket in basis points (10000 = 1.0x)
    /// Initial base_share_bps = 10000 (100% base, 0% bonus — shadow mode until activated)
    public entry fun initialize(
        aptos_framework: &signer, durations_secs: vector<u64>, multipliers_bps: vector<u64>
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<BonusConfig>(@aptos_framework),
            error::already_exists(ECONFIG_NOT_INITIALIZED)
        );

        let n = durations_secs.length();
        assert!(n > 0, error::invalid_argument(EINVALID_BUCKET_CONFIG_LENGTH));
        assert!(
            multipliers_bps.length() == n,
            error::invalid_argument(EINVALID_BUCKET_CONFIG_LENGTH)
        );

        for (i in 1..n) {
            assert!(
                durations_secs[i] > durations_secs[i - 1],
                error::invalid_argument(EDURATIONS_NOT_INCREASING)
            );
            assert!(
                multipliers_bps[i] > multipliers_bps[i - 1],
                error::invalid_argument(EMULTIPLIERS_NOT_INCREASING)
            );
        };

        for (i in 0..n) {
            assert!(
                multipliers_bps[i] > 0 && multipliers_bps[i] <= MAX_BUCKET_MULTIPLIER,
                error::invalid_argument(EINVALID_CONFIG)
            );
        };

        let buckets = vector[];
        for (i in 0..n) {
            buckets.push_back(
                BucketConfig {
                    duration_secs: durations_secs[i],
                    multiplier_bps: multipliers_bps[i]
                }
            );
        };

        move_to(
            aptos_framework,
            BonusConfig { base_share_bps: DEFAULT_BASE_SHARE_BPS, buckets }
        );

        // Emit BaseShareUpdated to signal initialization; old == new == DEFAULT_BASE_SHARE_BPS
        event::emit(
            BaseShareUpdated {
                old_base_share_bps: DEFAULT_BASE_SHARE_BPS,
                new_base_share_bps: DEFAULT_BASE_SHARE_BPS
            }
        );
    }

    /// Initialize bonus pool for a delegation pool. No-op if already initialized
    /// or if global BonusConfig doesn't exist. Callable by anyone.
    public entry fun try_initialize_bonus_pool(account: &signer) {
        if (!exists<BonusConfig>(@aptos_framework)) { return };
        let pool_address = signer::address_of(account);
        if (exists<BonusPoolState>(pool_address)) { return };

        move_to(
            account,
            BonusPoolState {
                total_weighted_shares: 0,
                acc_bonus_per_weighted_share: 0,
                positions: big_ordered_map::new(),
                bonus_coins: coin::zero<AptosCoin>()
            }
        );

        event::emit(BonusPoolInitialized { pool_address });
    }

    // ===========================================================================================
    // Governance Configuration
    // ===========================================================================================

    /// Update base share percentage (governance only). base_share_bps: 0-10000.
    public entry fun update_base_share(
        aptos_framework: &signer, new_base_share_bps: u64
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            new_base_share_bps <= MAX_BASE_SHARE_BPS,
            error::invalid_argument(EINVALID_BASE_SHARE_BPS)
        );

        let config = borrow_config_mut();
        let old_base_share_bps = config.base_share_bps;
        config.base_share_bps = new_base_share_bps;

        event::emit(BaseShareUpdated { old_base_share_bps, new_base_share_bps });
    }

    /// Update durations and multipliers for all buckets (governance only).
    /// Can update existing buckets or add new ones — cannot reduce the bucket count
    /// because existing positions reference bucket IDs by index.
    /// durations_secs must be strictly increasing across all buckets.
    public entry fun update_bucket_configs(
        aptos_framework: &signer, durations_secs: vector<u64>, multipliers_bps: vector<u64>
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        let new_n = durations_secs.length();
        assert!(new_n > 0, error::invalid_argument(EINVALID_BUCKET_CONFIG_LENGTH));
        assert!(
            multipliers_bps.length() == new_n,
            error::invalid_argument(EINVALID_BUCKET_CONFIG_LENGTH)
        );

        let config = borrow_config_mut();
        let old_n = config.buckets.length();
        assert!(new_n >= old_n, error::invalid_argument(ECANNOT_REDUCE_BUCKET_COUNT));

        for (i in 1..new_n) {
            assert!(
                durations_secs[i] > durations_secs[i - 1],
                error::invalid_argument(EDURATIONS_NOT_INCREASING)
            );
            assert!(
                multipliers_bps[i] > multipliers_bps[i - 1],
                error::invalid_argument(EMULTIPLIERS_NOT_INCREASING)
            );
        };

        for (i in 0..new_n) {
            assert!(
                multipliers_bps[i] > 0 && multipliers_bps[i] <= MAX_BUCKET_MULTIPLIER,
                error::invalid_argument(EINVALID_CONFIG)
            );
        };

        for (i in 0..old_n) {
            config.buckets[i].duration_secs = durations_secs[i];
            config.buckets[i].multiplier_bps = multipliers_bps[i];
        };

        for (i in old_n..new_n) {
            config.buckets.push_back(
                BucketConfig {
                    duration_secs: durations_secs[i],
                    multiplier_bps: multipliers_bps[i]
                }
            );
        };

        event::emit(BucketConfigsUpdated { durations_secs, multipliers_bps });
    }

    // ===========================================================================================
    // Private Helpers
    // ===========================================================================================

    inline fun borrow_config(): &BonusConfig {
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        &BonusConfig[@aptos_framework]
    }

    inline fun borrow_config_mut(): &mut BonusConfig {
        assert!(
            exists<BonusConfig>(@aptos_framework),
            error::not_found(ECONFIG_NOT_INITIALIZED)
        );
        &mut BonusConfig[@aptos_framework]
    }

    /// Get a copy of the bucket config for bucket_id, asserting the ID is valid.
    inline fun get_bucket(bucket_id: u64): BucketConfig {
        let config = borrow_config();
        assert!(
            bucket_id < config.buckets.length(),
            error::invalid_argument(EINVALID_BUCKET_ID)
        );
        config.buckets[bucket_id]
    }

    /// Returns (complete_cycles, partial_secs) for the current time relative to lock_start.
    inline fun calculate_cycles(
        lock_start_secs: u64, current_time: u64, cycle_duration: u64
    ): (u64, u64) {
        let elapsed = current_time - lock_start_secs;
        (elapsed / cycle_duration, elapsed % cycle_duration)
    }

    /// Compute weighted shares with overflow protection.
    inline fun weighted_shares(shares: u128, multiplier_bps: u64): u128 {
        assert!(
            shares <= MAX_U128 / (multiplier_bps as u128),
            error::invalid_state(EOVERFLOW_WEIGHT_CALCULATION)
        );
        shares * (multiplier_bps as u128) / 10000
    }

    /// Get weighted shares for a position, reading multiplier from current BonusConfig.
    inline fun get_weighted_shares(position: &Position): u128 {
        let bucket = get_bucket(position.bucket_id);
        weighted_shares(position.shares, bucket.multiplier_bps)
    }

    /// Update total_weighted_shares after a multiplier change (signed-safe arithmetic).
    inline fun update_total_weighted(
        pool: &mut BonusPoolState, old_weighted: u128, new_weighted: u128
    ) {
        if (new_weighted >= old_weighted) {
            pool.total_weighted_shares += new_weighted - old_weighted;
        } else {
            pool.total_weighted_shares -= old_weighted - new_weighted;
        }
    }

    /// Compute weighted_shares * acc / SCALE with overflow protection.
    inline fun earned_at(weighted_shares: u128, acc: u128): u128 {
        if (acc > 0) {
            assert!(
                weighted_shares <= MAX_U128 / acc,
                error::invalid_state(EOVERFLOW_REWARD_CALCULATION)
            );
        };
        weighted_shares * acc / ACCUMULATOR_SCALE
    }

    /// Pending bonus = earned_at(weighted_shares, acc) - debt.
    inline fun calculate_pending_bonus_internal(
        bonus_pool: &BonusPoolState, position: &Position
    ): u64 {
        let ws = get_weighted_shares(position);
        let earned_total = earned_at(ws, bonus_pool.acc_bonus_per_weighted_share);
        if (earned_total > position.debt) {
            ((earned_total - position.debt) as u64)
        } else { 0 }
    }

    // ===========================================================================================
    // View Functions
    // ===========================================================================================

    #[view]
    public fun get_shares_scaling_factor(): u128 {
        SHARES_SCALING_FACTOR
    }

    #[view]
    public fun get_base_share_bps(): u64 {
        borrow_config().base_share_bps
    }

    #[view]
    /// Get bucket configuration: (duration_secs, multiplier_bps)
    public fun get_bucket_config(bucket_id: u64): (u64, u64) {
        let bucket = get_bucket(bucket_id);
        (bucket.duration_secs, bucket.multiplier_bps)
    }

    #[view]
    /// Get user's locked shares. Returns 0 if pool or position does not exist.
    public fun get_total_bucket_shares(
        pool_address: address, user: address
    ): u128 {
        if (!exists<BonusPoolState>(pool_address)) {
            return 0
        };
        let bonus_pool = &BonusPoolState[pool_address];
        if (!bonus_pool.positions.contains(&user)) {
            return 0
        };
        bonus_pool.positions.borrow(&user).shares * SHARES_SCALING_FACTOR
    }

    #[view]
    /// Get user's position info: (bucket_id, shares, lock_start_secs, pending_bonus, complete_cycles)
    public fun get_position(pool_address: address, user: address): (u64, u128, u64, u64, u64) {
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
        let now = timestamp::now_seconds();
        let bucket = get_bucket(position.bucket_id);
        let (complete_cycles, _) =
            calculate_cycles(position.lock_start_secs, now, bucket.duration_secs);

        (
            position.bucket_id,
            position.shares * SHARES_SCALING_FACTOR,
            position.lock_start_secs,
            pending_bonus,
            complete_cycles
        )
    }

    #[view]
    /// Get pool state: (total_weighted_shares, acc_bonus_per_weighted_share)
    public fun get_pool_state(pool_address: address): (u128, u128) {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        let bonus_pool = &BonusPoolState[pool_address];
        (bonus_pool.total_weighted_shares, bonus_pool.acc_bonus_per_weighted_share)
    }

    // ===========================================================================================
    // Core Reward Distribution
    // ===========================================================================================

    /// Split reward coins at mint time, depositing the bonus portion into BonusPoolState.
    /// Called by stake.move when distributing rewards, so bonus coins never enter StakePool.active.
    /// Returns the base portion of coins to be merged into active stake.
    /// Returns all coins as base if: BonusPoolState/BonusConfig missing, or no bucket participants.
    public(friend) fun split_and_deposit_rewards(
        pool_address: address, reward_coins: Coin<AptosCoin>
    ): Coin<AptosCoin> {
        if (!exists<BonusPoolState>(pool_address)
            || !exists<BonusConfig>(@aptos_framework)) {
            return reward_coins
        };

        if (BonusPoolState[pool_address].total_weighted_shares == 0) {
            return reward_coins
        };

        let total = coin::value(&reward_coins);
        let base_amount =
            (
                ((total as u128) * (BonusConfig[@aptos_framework].base_share_bps as u128)
                    / (MAX_BASE_SHARE_BPS as u128)) as u64
            );
        let bonus_amount = total - base_amount;

        if (bonus_amount == 0) {
            return reward_coins
        };

        let bonus_coins = coin::extract(&mut reward_coins, bonus_amount);
        let bonus_pool = &mut BonusPoolState[pool_address];
        coin::merge(&mut bonus_pool.bonus_coins, bonus_coins);
        bonus_pool.acc_bonus_per_weighted_share +=(bonus_amount as u128)
            * ACCUMULATOR_SCALE / bonus_pool.total_weighted_shares;

        event::emit(
            BonusRewardsDistributed {
                pool_address,
                total_rewards: total,
                base_rewards: base_amount,
                bonus_rewards: bonus_amount
            }
        );

        reward_coins
    }

    /// Extract bonus payment from pool for a user. Called by delegation_pool when paying out rewards.
    public(friend) fun extract_bonus_payment(
        pool_address: address, amount: u64
    ): Coin<AptosCoin> {
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            coin::value(&bonus_pool.bonus_coins) >= amount,
            error::invalid_state(EINSUFFICIENT_BONUS_COINS)
        );
        coin::extract(&mut bonus_pool.bonus_coins, amount)
    }

    // ===========================================================================================
    // User Position Management
    // ===========================================================================================

    /// Join a lockup bucket with delegation shares (auto-renewing cycles start immediately).
    /// raw_shares: raw delegation pool shares (will be normalized internally by dividing SHARES_SCALING_FACTOR)
    public fun join_bucket(
        user: &signer,
        pool_address: address,
        bucket_id: u64,
        raw_shares: u128
    ) {
        let user_addr = signer::address_of(user);
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        let normalized = raw_shares / SHARES_SCALING_FACTOR;
        assert!(normalized > 0, error::invalid_argument(EZERO_SHARES));

        let bucket = get_bucket(bucket_id);
        let ws = weighted_shares(normalized, bucket.multiplier_bps);
        let now = timestamp::now_seconds();

        let bonus_pool = &mut BonusPoolState[pool_address];
        assert!(
            !bonus_pool.positions.contains(&user_addr),
            error::already_exists(EPOSITION_ALREADY_EXISTS)
        );

        let debt = earned_at(ws, bonus_pool.acc_bonus_per_weighted_share);
        bonus_pool.positions.add(
            user_addr,
            Position { bucket_id, shares: normalized, debt, lock_start_secs: now }
        );
        bonus_pool.total_weighted_shares += ws;

        event::emit(
            JoinedBucket {
                pool_address,
                user: user_addr,
                bucket_id,
                shares: normalized,
                lock_start_secs: now
            }
        );
    }

    /// Upgrade to a longer bucket: settles complete cycles first, then switches bucket.
    /// Complete cycle rewards are returned for immediate payment to the caller.
    /// Partial cycle progress (lock_start advanced to last cycle boundary) and
    /// remaining pending rewards are preserved across the multiplier change — no burning.
    /// Returns claimed amount (complete cycles settled before the upgrade).
    public fun upgrade_bucket(
        user: &signer, pool_address: address, new_bucket_id: u64
    ): u64 {
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

        let old_pos = bonus_pool.positions.remove(&user_addr);
        let old_bucket = get_bucket(old_pos.bucket_id);
        let new_bucket = get_bucket(new_bucket_id);
        assert!(
            new_bucket.duration_secs > old_bucket.duration_secs,
            error::invalid_argument(EMUST_UPGRADE)
        );

        // Settle complete cycles before switching bucket so the user isn't penalized
        // for upgrading. lock_start advances to the last cycle boundary; partial cycle
        // progress carries over into the new bucket.
        let now = timestamp::now_seconds();
        let elapsed = now - old_pos.lock_start_secs;
        let complete_cycles = elapsed / old_bucket.duration_secs;
        let total_pending = calculate_pending_bonus_internal(bonus_pool, &old_pos);

        let (claimable, new_lock_start) =
            if (complete_cycles == 0 || elapsed == 0 || total_pending == 0) {
                (0u64, old_pos.lock_start_secs)
            } else {
                let complete_secs =
                    (complete_cycles as u128) * (old_bucket.duration_secs as u128);
                let c = ((total_pending as u128) * complete_secs / (elapsed as u128)) as u64;
                (c, old_pos.lock_start_secs
                    + complete_cycles * old_bucket.duration_secs)
            };

        let remaining_pending = (total_pending - claimable) as u128;

        let old_weighted = weighted_shares(old_pos.shares, old_bucket.multiplier_bps);
        let new_weighted = weighted_shares(old_pos.shares, new_bucket.multiplier_bps);

        // Preserve remaining_pending across the multiplier change.
        // new_weighted >= old_weighted holds when longer duration implies higher multiplier
        // (the intended bucket configuration), ensuring no underflow here.
        let new_debt =
            earned_at(new_weighted, bonus_pool.acc_bonus_per_weighted_share)
                - remaining_pending;

        update_total_weighted(bonus_pool, old_weighted, new_weighted);
        bonus_pool.positions.add(
            user_addr,
            Position {
                bucket_id: new_bucket_id,
                shares: old_pos.shares,
                debt: new_debt,
                lock_start_secs: new_lock_start
            }
        );

        if (claimable > 0) {
            event::emit(
                BonusClaimed {
                    pool_address,
                    user: user_addr,
                    bucket_id: old_pos.bucket_id,
                    amount: claimable,
                    complete_cycles
                }
            );
        };

        event::emit(
            UpgradedBucket {
                pool_address,
                user: user_addr,
                old_bucket_id: old_pos.bucket_id,
                new_bucket_id,
                lock_start_secs: new_lock_start
            }
        );

        claimable
    }

    /// Downgrade to a shorter bucket: pays complete-cycle rewards, burns partial-cycle remainder,
    /// then re-joins the new (shorter) bucket with a fresh cycle.
    /// Returns: (claimed_rewards, burned_rewards)
    public fun downgrade_bucket(
        user: &signer, pool_address: address, new_bucket_id: u64
    ): (u64, u64) {
        let user_addr = signer::address_of(user);
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );

        let shares = {
            let bonus_pool = &BonusPoolState[pool_address];
            assert!(
                bonus_pool.positions.contains(&user_addr),
                error::not_found(EPOSITION_NOT_FOUND)
            );
            let pos = bonus_pool.positions.borrow(&user_addr);
            let old_bucket = get_bucket(pos.bucket_id);
            let new_bucket = get_bucket(new_bucket_id);
            assert!(
                new_bucket.duration_secs < old_bucket.duration_secs,
                error::invalid_argument(EMUST_DOWNGRADE)
            );
            pos.shares
        };

        let (claimed, burned, _) =
            exit_bucket(user, pool_address, shares * SHARES_SCALING_FACTOR);
        join_bucket(
            user,
            pool_address,
            new_bucket_id,
            shares * SHARES_SCALING_FACTOR
        );
        (claimed, burned)
    }

    /// Claim accumulated bonus rewards for all complete cycles.
    /// Incomplete cycle rewards remain pending until the cycle completes.
    /// Returns amount of bonus rewards claimed.
    public fun claim_bonus(user: &signer, pool_address: address): u64 {
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

        let pos = bonus_pool.positions.remove(&user_addr);
        let now = timestamp::now_seconds();
        let bucket = get_bucket(pos.bucket_id);
        let elapsed = now - pos.lock_start_secs;
        let complete_cycles = elapsed / bucket.duration_secs;
        let total_pending = calculate_pending_bonus_internal(bonus_pool, &pos);

        // Proportional formula: claimable = total_pending * (complete_cycles * duration) / elapsed
        // Assumption: rewards arrive at a constant rate, so time proportion = reward proportion.
        let claimable =
            if (complete_cycles == 0 || elapsed == 0 || total_pending == 0) { 0u64 }
            else {
                let complete_secs = (complete_cycles as u128)
                    * (bucket.duration_secs as u128);
                ((total_pending as u128) * complete_secs / (elapsed as u128)) as u64
            };

        if (claimable == 0) {
            bonus_pool.positions.add(user_addr, pos);
            return 0
        };

        // Advance debt by claimed; reset lock_start to cycle boundary to prevent double-claiming.
        pos.debt += claimable as u128;
        pos.lock_start_secs = pos.lock_start_secs
            + complete_cycles * bucket.duration_secs;

        bonus_pool.positions.add(user_addr, pos);

        event::emit(
            BonusClaimed {
                pool_address,
                user: user_addr,
                bucket_id: pos.bucket_id,
                amount: claimable,
                complete_cycles
            }
        );

        claimable
    }

    /// Fully exit position (convenience wrapper).
    /// Returns (bonus_claimed, bonus_burned, exit_shares).
    public fun exit_bucket_all(user: &signer, pool_address: address): (u64, u64, u128) {
        let user_addr = signer::address_of(user);
        assert!(
            exists<BonusPoolState>(pool_address),
            error::not_found(EBONUS_POOL_NOT_INITIALIZED)
        );
        let shares = BonusPoolState[pool_address].positions.borrow(&user_addr).shares;
        exit_bucket(user, pool_address, shares * SHARES_SCALING_FACTOR)
    }

    /// Exit position (fully or partially).
    /// Complete cycle rewards for the exited portion: claimed.
    /// Incomplete cycle rewards for the exited portion: BURNED (not redistributed).
    /// Pass exit_shares_raw == position raw shares (pos.shares * SHARES_SCALING_FACTOR) for a full exit.
    /// Returns (bonus_claimed, bonus_burned, exit_shares_raw)
    public fun exit_bucket(
        user: &signer, pool_address: address, exit_shares_raw: u128
    ): (u64, u64, u128) {
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

        let pos = bonus_pool.positions.remove(&user_addr);
        let exit_shares = (exit_shares_raw + SHARES_SCALING_FACTOR - 1)
            / SHARES_SCALING_FACTOR;
        assert!(
            exit_shares > 0 && exit_shares <= pos.shares,
            error::invalid_argument(EZERO_SHARES)
        );

        let now = timestamp::now_seconds();
        let bucket = get_bucket(pos.bucket_id);
        let elapsed = now - pos.lock_start_secs;
        let (complete_cycles, partial_cycle_progress) =
            calculate_cycles(pos.lock_start_secs, now, bucket.duration_secs);

        let total_pending = calculate_pending_bonus_internal(bonus_pool, &pos);
        let exit_pending = ((total_pending as u128) * exit_shares / pos.shares) as u64;

        // Same proportional formula as claim_bonus, applied to the exited portion.
        let (claimed, burned) =
            if (complete_cycles == 0 || elapsed == 0) {
                (0u64, exit_pending)
            } else {
                let complete_secs = (complete_cycles as u128)
                    * (bucket.duration_secs as u128);
                let claimable = ((exit_pending as u128) * complete_secs
                    / (elapsed as u128)) as u64;
                (claimable, exit_pending - claimable)
            };

        let exit_weighted = weighted_shares(exit_shares, bucket.multiplier_bps);
        bonus_pool.total_weighted_shares -= exit_weighted;

        if (exit_shares < pos.shares) {
            let exit_debt = pos.debt * exit_shares / pos.shares;
            pos.shares -= exit_shares;
            pos.debt -= exit_debt;
            bonus_pool.positions.add(user_addr, pos);
        };
        // Full exit: position already removed, do not re-insert.

        event::emit(
            ExitedBucket {
                pool_address,
                user: user_addr,
                bucket_id: pos.bucket_id,
                shares: exit_shares,
                bonus_claimed: claimed,
                bonus_burned: burned,
                complete_cycles,
                partial_cycle_progress
            }
        );

        (claimed, burned, exit_shares * SHARES_SCALING_FACTOR)
    }

    // ===========================================================================================
    // Test-only Functions
    // ===========================================================================================

    #[test_only]
    public fun test_initialize(aptos_framework: &signer) {
        initialize(
            aptos_framework,
            vector[1296000, 2592000, 5184000, 7776000], // 15d, 30d, 60d, 90d
            vector[10000, 20000, 40000, 60000] // 1x, 2x, 4x, 6x
        );
    }

    #[test_only]
    public fun test_initialize_bonus_pool(account: &signer) {
        try_initialize_bonus_pool(account);
    }

    #[test_only]
    public fun is_bonus_pool_initialized(pool_address: address): bool {
        exists<BonusPoolState>(pool_address)
    }

    #[test_only]
    public fun test_split_and_deposit_rewards(
        pool_address: address, reward_coins: Coin<AptosCoin>
    ): Coin<AptosCoin> {
        split_and_deposit_rewards(pool_address, reward_coins)
    }
}
