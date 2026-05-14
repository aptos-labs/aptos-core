/// Manages configuration and validation for higher transaction limits based on
/// staking.
///
/// Users can request multipliers to transaction limits (e..g, execution limit
/// or IO limit) if they prove they control a significant stake in a stake pool
/// that is currently in the active validator set:
///   - as a stake pool owner,
///   - as a delegated voter,
///   - as a delegation pool delegator.
/// For example, one can request 2.5x on execution limits and 5x on IO limits.
///
/// Multipliers are expressed as percent of the base limit where 100 is 1x,
/// 250 is 2.5x.
///
/// The on-chain config stores a vector of tiers. Each tier maps multiplier to
/// the required minimum stake threshold. A smallest multiplier that is greater
/// than or equal to the requested multiplier is chosen.
module aptos_framework::transaction_limits {
    use aptos_framework::system_addresses;
    use aptos_framework::aptos_governance;
    use aptos_framework::delegation_pool;
    use aptos_framework::stake;
    use std::error;

    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    /// No stake pool exists at the specified address.
    const ESTAKE_POOL_NOT_FOUND: u64 = 1;
    /// Fee payer is not the owner of the specified stake pool.
    const ENOT_STAKE_POOL_OWNER: u64 = 2;
    /// Fee payer is not the delegated voter of the specified stake pool.
    const ENOT_DELEGATED_VOTER: u64 = 3;
    /// No delegation pool exists at the specified address.
    const EDELEGATION_POOL_NOT_FOUND: u64 = 4;
    /// Committed stake is insufficient for the requested multiplier tier.
    const EINSUFFICIENT_STAKE: u64 = 5;
    /// Config tiers are not monotonically ordered.
    const ETHRESHOLDS_NOT_MONOTONIC: u64 = 6;
    /// Multiplier must be > 100 (> 1x).
    const EINVALID_MULTIPLIER: u64 = 7;
    /// Requested multiplier is not available in any configured tier.
    const EMULTIPLIER_NOT_AVAILABLE: u64 = 8;
    /// Stake pool is not in the current-epoch validator set.
    const EPOOL_NOT_IN_VALIDATOR_SET: u64 = 9;
    /// Min-stakes and multipliers vectors have different lengths.
    const EVECTOR_LENGTH_MISMATCH: u64 = 10;

    /// Every multiplier must be greater than this minimum (1x).
    ///
    /// INVARIANT: must match Rust version checked by VM.
    const MIN_MULTIPLIER_PERCENT: u64 = 100;
    /// Every multiplier must be less than or equal to this maximum (100x).
    ///
    /// INVARIANT: must match Rust version checked by VM.
    const MAX_MULTIPLIER_PERCENT: u64 = 10000;

    /// A single tier: the minimum committed stake required and the multiplier
    /// it unlocks.
    struct TxnLimitTier has copy, drop, store {
        min_stake: u64,
        // Multiplier as percent of the base limit (100 = 1x, 200 = 2x, 250 = 2.5x).
        multiplier_percent: u64
    }

    /// On-chain configuration for higher transaction limits. Stores a vector
    /// of tiers for each dimension (e.g., execution, IO). Tiers are ordered
    /// monotonically by both minimum stakes and multipliers.
    enum TxnLimitsConfig has key {
        V1 {
            execution_tiers: vector<TxnLimitTier>,
            io_tiers: vector<TxnLimitTier>
        }
    }

    /// Multipliers requested by the user.
    ///
    /// INVARIANT: must match Rust enum for BCS serialization.
    enum RequestedMultipliers has copy, drop, store {
        V1 {
            /// Execution-gas multiplier (100 is 1x).
            execution_multiplier_percent: u64,
            /// IO-gas multiplier (100 is 1x).
            io_multiplier_percent: u64
        }
    }

    /// Request for higher transaction limits, passed to the prologue. Carries
    /// the proof that the sender has enough stake.
    ///
    /// INVARIANT: must match Rust enum for BCS serialization.
    enum UserTxnLimitsRequest has copy, drop {
        StakePoolOwner {
            multipliers: RequestedMultipliers
        },
        DelegatedVoter {
            pool_address: address,
            multipliers: RequestedMultipliers
        },
        DelegationPoolDelegator {
            pool_address: address,
            multipliers: RequestedMultipliers
        }
    }

    /// Creates a new tier. Aborts if multiplier is not in (100, 10000].
    public fun new_tier(min_stake: u64, multiplier_percent: u64): TxnLimitTier {
        assert!(
            multiplier_percent > MIN_MULTIPLIER_PERCENT
                && multiplier_percent <= MAX_MULTIPLIER_PERCENT,
            error::invalid_argument(EINVALID_MULTIPLIER)
        );
        TxnLimitTier { min_stake, multiplier_percent }
    }

    /// Aborts if:
    ///   - Minimum stake tiers are not monotonically increasing.
    ///   - Multiplier tiers are not strictly monotonically increasing.
    fun validate_tiers(tiers: &vector<TxnLimitTier>) {
        let i = 1;
        let len = tiers.length();

        while (i < len) {
            let prev = &tiers[i - 1];
            let curr = &tiers[i];
            assert!(
                curr.min_stake >= prev.min_stake
                    && curr.multiplier_percent > prev.multiplier_percent,
                error::invalid_argument(ETHRESHOLDS_NOT_MONOTONIC)
            );
            i += 1;
        };
    }

    /// Builds a vector of tiers from inputs.
    ///
    /// Aborts if:
    ///   - Minimum stakes and multipliers vectors have different lengths.
    ///   - Minimum stakes and multipliers vectors are not monotonically
    ///     increasing.
    ///   - Multiplier is not valid (1x or below).
    fun new_tiers(min_stakes: vector<u64>, multipliers_percent: vector<u64>)
        : vector<TxnLimitTier> {
        let len = min_stakes.length();
        assert!(
            len == multipliers_percent.length(),
            error::invalid_argument(EVECTOR_LENGTH_MISMATCH)
        );

        let tiers = vector[];
        let i = 0;
        while (i < len) {
            tiers.push_back(new_tier(min_stakes[i], multipliers_percent[i]));
            i += 1;
        };
        validate_tiers(&tiers);

        tiers
    }

    /// Finds the smallest tier whose multiplier is greater than or equal to
    /// the requested multiplier. Returns minimum stake correspondng to this
    /// tier.
    ///
    /// Aborts if no tier can cover the request.
    ///
    /// Implemnetation note: Tier count is small in practice, so using linear
    /// search here, which is cheaper and currently faster than a binary search
    /// in Move bytecode.
    fun find_min_stake_required(
        tiers: &vector<TxnLimitTier>, multiplier_percent: u64
    ): u64 {
        let (found, i) = tiers.find(|t| t.multiplier_percent >= multiplier_percent);
        assert!(found, error::invalid_argument(EMULTIPLIER_NOT_AVAILABLE));
        tiers[i].min_stake
    }

    /// Only called during genesis.
    friend fun initialize(
        aptos_framework: &signer,
        execution_tiers: vector<TxnLimitTier>,
        io_tiers: vector<TxnLimitTier>
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);
        validate_tiers(&execution_tiers);
        validate_tiers(&io_tiers);

        move_to(
            aptos_framework,
            TxnLimitsConfig::V1 { execution_tiers, io_tiers }
        );
    }

    /// Governance-only: update stake thresholds and multipliers.
    public fun update_config(
        aptos_framework: &signer,
        execution_min_stakes: vector<u64>,
        execution_multipliers_percent: vector<u64>,
        io_min_stakes: vector<u64>,
        io_multipliers_percent: vector<u64>
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        let execution_tiers = new_tiers(
            execution_min_stakes, execution_multipliers_percent
        );
        let io_tiers = new_tiers(io_min_stakes, io_multipliers_percent);

        if (!exists<TxnLimitsConfig>(@aptos_framework)) {
            move_to(
                aptos_framework,
                TxnLimitsConfig::V1 { execution_tiers, io_tiers }
            );
        } else {
            let config = &mut TxnLimitsConfig[@aptos_framework];
            config.execution_tiers = execution_tiers;
            config.io_tiers = io_tiers;
        }
    }

    /// Aborts if:
    ///   - Requested multipliers are not well-formed.
    ///   - Transaction limits config does not exist or there is no tier
    ///     matching the requested multipliers.
    ///   - There is not enough stake to cover the minimum required amount.
    fun validate_enough_stake(
        stake_amount: u64, multipliers: RequestedMultipliers
    ) {
        let (execution_multiplier_percent, io_multiplier_percent) =
            match(multipliers) {
                RequestedMultipliers::V1 {
                    execution_multiplier_percent,
                    io_multiplier_percent
                } => (execution_multiplier_percent, io_multiplier_percent)
            };
        assert!(
            execution_multiplier_percent > MIN_MULTIPLIER_PERCENT
                && execution_multiplier_percent <= MAX_MULTIPLIER_PERCENT,
            error::invalid_argument(EINVALID_MULTIPLIER)
        );
        assert!(
            io_multiplier_percent > MIN_MULTIPLIER_PERCENT
                && io_multiplier_percent <= MAX_MULTIPLIER_PERCENT,
            error::invalid_argument(EINVALID_MULTIPLIER)
        );

        let config = &TxnLimitsConfig[@aptos_framework];
        let execution_threshold =
            find_min_stake_required(&config.execution_tiers, execution_multiplier_percent);
        let io_threshold =
            find_min_stake_required(&config.io_tiers, io_multiplier_percent);

        assert!(
            stake_amount >= execution_threshold,
            error::permission_denied(EINSUFFICIENT_STAKE)
        );
        assert!(
            stake_amount >= io_threshold, error::permission_denied(EINSUFFICIENT_STAKE)
        );
    }

    /// Only called during prologue to validate that the fee payer qualifies
    /// for the requested limit multipliers.
    friend fun validate_high_txn_limits(
        fee_payer: address, request: UserTxnLimitsRequest
    ) {
        match(request) {
            StakePoolOwner { multipliers } => {
                assert!(
                    stake::owner_cap_exists(fee_payer),
                    error::permission_denied(ENOT_STAKE_POOL_OWNER)
                );
                let pool_address = stake::get_pool_address_for_owner(fee_payer);
                assert!(
                    stake::is_current_epoch_validator(pool_address),
                    error::permission_denied(EPOOL_NOT_IN_VALIDATOR_SET)
                );
                let stake_amount = aptos_governance::get_voting_power(pool_address);
                validate_enough_stake(stake_amount, multipliers);
            },
            DelegatedVoter { pool_address, multipliers } => {
                assert!(
                    stake::stake_pool_exists(pool_address),
                    error::not_found(ESTAKE_POOL_NOT_FOUND)
                );
                assert!(
                    fee_payer == stake::get_delegated_voter(pool_address),
                    error::permission_denied(ENOT_DELEGATED_VOTER)
                );
                assert!(
                    stake::is_current_epoch_validator(pool_address),
                    error::permission_denied(EPOOL_NOT_IN_VALIDATOR_SET)
                );
                let stake_amount = aptos_governance::get_voting_power(pool_address);
                validate_enough_stake(stake_amount, multipliers);
            },
            DelegationPoolDelegator { pool_address, multipliers } => {
                assert!(
                    delegation_pool::delegation_pool_exists(pool_address),
                    error::not_found(EDELEGATION_POOL_NOT_FOUND)
                );
                assert!(
                    stake::is_current_epoch_validator(pool_address),
                    error::permission_denied(EPOOL_NOT_IN_VALIDATOR_SET)
                );
                let (active, _, pending_inactive) = delegation_pool::get_stake(
                    pool_address, fee_payer
                );
                validate_enough_stake(active + pending_inactive, multipliers);
            }
        }
    }

    #[test_only]
    use aptos_framework::account;

    #[test_only]
    fun initialize_for_test(aptos_framework: &signer) {
        // Execution: 2x - 100, 4x -  500, 8x - 1000
        // IO:        2x - 200, 4x - 1000, 8x - 2000
        initialize(
            aptos_framework,
            vector[new_tier(100, 200), new_tier(500, 400), new_tier(1000, 800)],
            vector[new_tier(200, 200), new_tier(1000, 400), new_tier(2000, 800)]
        );
    }

    #[test_only]
    fun initialize_for_test_with_staking_impl(
        aptos_framework: &signer,
        validator: &signer,
        amount: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
    ) {
        initialize_for_test(aptos_framework);
        stake::initialize_for_test(aptos_framework);
        let (_sk, pk, pop) = stake::generate_identity();
        stake::initialize_test_validator(
            &pk,
            &pop,
            validator,
            amount,
            should_join_validator_set,
            should_end_epoch
        );
    }

    #[test_only]
    fun initialize_for_test_with_staking(
        aptos_framework: &signer, validator: &signer, amount: u64
    ) {
        initialize_for_test_with_staking_impl(
            aptos_framework,
            validator,
            amount,
            true,
            true,
        )
    }

    #[test_only]
    fun initialize_for_test_with_inactive_stake_pool(
        aptos_framework: &signer, validator: &signer, amount: u64
    ) {
        initialize_for_test_with_staking_impl(
            aptos_framework,
            validator,
            amount,
            false,
            false,
        )
    }

    #[test_only]
    fun initialize_for_test_with_delegation_pool_impl(
        aptos_framework: &signer,
        pool_owner: &signer,
        delegator: &signer,
        delegator_stake: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
    ) {
        // Use higher thresholds for delegation pool tests since because we
        // need to have at least 10 APT in the pool.
        //
        // Execution: 2x - 5 APT, 8x - 50 APT
        // IO:        2x - 10 APT, 8x - 100 APT
        initialize(
            aptos_framework,
            vector[new_tier(5_0000_0000, 200), new_tier(50_0000_0000, 800)],
            vector[new_tier(10_0000_0000, 200), new_tier(100_0000_0000, 800)]
        );
        delegation_pool::initialize_for_test(aptos_framework);
        delegation_pool::initialize_test_validator(
            pool_owner, 1_000_0000_0000, should_join_validator_set, should_end_epoch
        );

        let delegator_addr = std::signer::address_of(delegator);
        if (!account::exists_at(delegator_addr)) {
            account::create_account_for_test(delegator_addr);
        };
        stake::mint(delegator, delegator_stake);
        let pool_address =
            delegation_pool::get_owned_pool_address(
                std::signer::address_of(pool_owner)
            );
        delegation_pool::add_stake(delegator, pool_address, delegator_stake);
        delegation_pool::end_aptos_epoch();
    }

    #[test_only]
    fun initialize_for_test_with_delegation_pool(
        aptos_framework: &signer,
        pool_owner: &signer,
        delegator: &signer,
        delegator_stake: u64
    ) {
        initialize_for_test_with_delegation_pool_impl(
            aptos_framework,
            pool_owner,
            delegator,
            delegator_stake,
            true,
            true,
        )
    }

    #[test_only]
    fun initialize_for_test_with_inactive_delegation_pool(
        aptos_framework: &signer,
        pool_owner: &signer,
        delegator: &signer,
        delegator_stake: u64
    ) {
        initialize_for_test_with_delegation_pool_impl(
            aptos_framework,
            pool_owner,
            delegator,
            delegator_stake,
            false,
            false,
        )
    }

    #[test]
    fun test_new_tier() {
        let tier = new_tier(1000, 200);
        assert!(tier.min_stake == 1000);
        assert!(tier.multiplier_percent == 200);

        let tier = new_tier(1000, 10000);
        assert!(tier.min_stake == 1000);
        assert!(tier.multiplier_percent == 10000);
    }

    #[test]
    #[expected_failure(abort_code = 0x10007)]
    fun test_new_tier_multiplier_1x() {
        new_tier(1000, 100);
    }

    #[test]
    #[expected_failure(abort_code = 0x10007)]
    fun test_new_tier_multiplier_below_1x() {
        new_tier(1000, 30);
    }

    #[test]
    #[expected_failure(abort_code = 0x10007)]
    fun test_new_tier_multiplier_above_100x() {
        new_tier(1000, 10001);
    }

    #[test]
    fun test_new_tiers() {
        let tiers = new_tiers(vector[], vector[]);
        assert!(tiers.is_empty());

        let tiers = new_tiers(vector[100], vector[200]);
        assert!(tiers.length() == 1);
        assert!(tiers[0].min_stake == 100);
        assert!(tiers[0].multiplier_percent == 200);

        let tiers = new_tiers(vector[500, 500, 1000], vector[200, 400, 800]);
        assert!(tiers.length() == 3);
        assert!(tiers[0].min_stake == 500);
        assert!(tiers[0].multiplier_percent == 200);
        assert!(tiers[1].min_stake == 500);
        assert!(tiers[1].multiplier_percent == 400);
        assert!(tiers[2].min_stake == 1000);
        assert!(tiers[2].multiplier_percent == 800);
    }

    #[test]
    #[expected_failure(abort_code = 0x1000A)]
    fun test_new_tiers_length_mismatch() {
        new_tiers(vector[100, 200], vector[200]);
    }

    #[test]
    #[expected_failure(abort_code = 0x10006)]
    fun test_new_tiers_descreasing_stakes() {
        new_tiers(vector[500, 100], vector[200, 400]);
    }

    #[test]
    #[expected_failure(abort_code = 0x10006)]
    fun test_new_tiers_same_multipliers() {
        new_tiers(vector[100, 200], vector[200, 200]);
    }

    #[test]
    #[expected_failure(abort_code = 0x10006)]
    fun test_new_tiers_descreasing_multipliers() {
        new_tiers(vector[100, 200], vector[200, 200]);
    }

    #[test]
    fun test_find_min_stake_required() {
        let tiers = new_tiers(vector[10, 50, 100], vector[200, 400, 800]);

        // Exact match.
        assert!(find_min_stake_required(&tiers, 200) == 10);
        assert!(find_min_stake_required(&tiers, 400) == 50);
        assert!(find_min_stake_required(&tiers, 800) == 100);

        // 250 rounds up to 400 tier, threshold 50.
        assert!(find_min_stake_required(&tiers, 250) == 50);
        // 101 rounds up to 200 tier, threshold 10.
        assert!(find_min_stake_required(&tiers, 101) == 10);
        // 799 rounds up to 800 tier, threshold 100.
        assert!(find_min_stake_required(&tiers, 799) == 100);
    }

    #[test]
    #[expected_failure(abort_code = 0x10008)]
    fun test_find_threshold_exceeds_max() {
        let tiers = new_tiers(vector[10, 50], vector[200, 400]);
        find_min_stake_required(&tiers, 401);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_validate_enough_stake(aptos_framework: &signer) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            200, RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50005)]
    fun test_validate_enough_stake_execution_fails(
        aptos_framework: &signer
    ) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            300, RequestedMultipliers::V1 { execution_multiplier_percent: 400, io_multiplier_percent: 200 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50005)]
    fun test_validate_enough_stake_io_fails(aptos_framework: &signer) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            300, RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 400 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10007)]
    fun test_validate_enough_stake_too_small_execution_multiplier(
        aptos_framework: &signer
    ) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            9999, RequestedMultipliers::V1 { execution_multiplier_percent: 100, io_multiplier_percent: 200 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10007)]
    fun test_validate_enough_stake_too_small_io_multiplier(
        aptos_framework: &signer
    ) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            9999, RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 100 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10007)]
    fun test_validate_enough_stake_too_large_execution_multiplier(
        aptos_framework: &signer
    ) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            9999, RequestedMultipliers::V1 { execution_multiplier_percent: 10001, io_multiplier_percent: 200 }
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10007)]
    // EINVALID_MULTIPLIER
    fun test_validate_enough_stake_too_large_io_multiplier(
        aptos_framework: &signer
    ) {
        initialize_for_test(aptos_framework);
        validate_enough_stake(
            9999, RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 10001 }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    fun test_validate_stake_pool_owner_success(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 1000);
        validate_high_txn_limits(
            @0x123,
            UserTxnLimitsRequest::StakePoolOwner {
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x50002)]
    fun test_validate_stake_pool_owner_no_cap(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 1000);
        // @0x456 has no OwnerCapability.
        validate_high_txn_limits(
            @0x456,
            UserTxnLimitsRequest::StakePoolOwner {
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x50005)]
    fun test_validate_stake_pool_owner_insufficient(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 100);
        validate_high_txn_limits(
            @0x123,
            UserTxnLimitsRequest::StakePoolOwner {
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 800, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    fun test_validate_delegated_voter_success(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 1000);
        stake::set_delegated_voter(validator, @0x456);
        validate_high_txn_limits(
            @0x456,
            UserTxnLimitsRequest::DelegatedVoter {
                pool_address: @0x123,
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x50003)]
    fun test_validate_delegated_voter_wrong_voter(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 1000);
        // @0x789 is not the delegated voter (default voter is the owner @0x123).
        validate_high_txn_limits(
            @0x789,
            UserTxnLimitsRequest::DelegatedVoter {
                pool_address: @0x123,
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001)]
    fun test_validate_delegated_voter_pool_not_found(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_staking(aptos_framework, validator, 1000);
        validate_high_txn_limits(
            @0x456,
            UserTxnLimitsRequest::DelegatedVoter {
                pool_address: @0x789,
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, pool_owner = @0x111, delegator = @0x222)]
    fun test_validate_delegation_pool_delegator_success(
        aptos_framework: &signer, pool_owner: &signer, delegator: &signer
    ) {
        // 20 APT stake, 2x needs 5 APT for execution and 10 APT for IO, passes.
        initialize_for_test_with_delegation_pool(
            aptos_framework,
            pool_owner,
            delegator,
            20_0000_0000
        );
        validate_high_txn_limits(
            @0x222,
            UserTxnLimitsRequest::DelegationPoolDelegator {
                pool_address: delegation_pool::get_owned_pool_address(@0x111),
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, pool_owner = @0x111, delegator = @0x222)]
    #[expected_failure(abort_code = 0x60004)]
    fun test_validate_delegation_pool_not_found(
        aptos_framework: &signer, pool_owner: &signer, delegator: &signer
    ) {
        initialize_for_test_with_delegation_pool(
            aptos_framework,
            pool_owner,
            delegator,
            20_0000_0000
        );
        validate_high_txn_limits(
            @0x222,
            UserTxnLimitsRequest::DelegationPoolDelegator {
                pool_address: @0x333,
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 200, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, pool_owner = @0x111, delegator = @0x222)]
    #[expected_failure(abort_code = 0x50005)]
    fun test_validate_delegation_pool_insufficient_stake(
        aptos_framework: &signer, pool_owner: &signer, delegator: &signer
    ) {
        // 20 APT stake, but 8x execution needs 50 APT, so test fails.
        initialize_for_test_with_delegation_pool(
            aptos_framework,
            pool_owner,
            delegator,
            20_0000_0000
        );
        validate_high_txn_limits(
            @0x222,
            UserTxnLimitsRequest::DelegationPoolDelegator {
                pool_address: delegation_pool::get_owned_pool_address(@0x111),
                multipliers: RequestedMultipliers::V1 { execution_multiplier_percent: 800, io_multiplier_percent: 200 }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x50009)]
    fun test_validate_stake_pool_owner_pool_not_in_validator_set(
        aptos_framework: &signer, validator: &signer
    ) {
        // Stake pool has plenty of APT but never joined the validator set.
        initialize_for_test_with_inactive_stake_pool(aptos_framework, validator, 1000);
        validate_high_txn_limits(
            @0x123,
            UserTxnLimitsRequest::StakePoolOwner {
                multipliers: RequestedMultipliers::V1 {
                    execution_multiplier_percent: 200,
                    io_multiplier_percent: 200
                }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x50009)]
    fun test_validate_delegated_voter_pool_not_in_validator_set(
        aptos_framework: &signer, validator: &signer
    ) {
        initialize_for_test_with_inactive_stake_pool(aptos_framework, validator, 1000);
        stake::set_delegated_voter(validator, @0x456);
        validate_high_txn_limits(
            @0x456,
            UserTxnLimitsRequest::DelegatedVoter {
                pool_address: @0x123,
                multipliers: RequestedMultipliers::V1 {
                    execution_multiplier_percent: 200,
                    io_multiplier_percent: 200
                }
            }
        );
    }

    #[test(aptos_framework = @aptos_framework, pool_owner = @0x111, delegator = @0x222)]
    #[expected_failure(abort_code = 0x50009)]
    fun test_validate_delegation_pool_delegator_pool_not_in_validator_set(
        aptos_framework: &signer, pool_owner: &signer, delegator: &signer
    ) {
        initialize_for_test_with_inactive_delegation_pool(
            aptos_framework,
            pool_owner,
            delegator,
            20_0000_0000
        );
        validate_high_txn_limits(
            @0x222,
            UserTxnLimitsRequest::DelegationPoolDelegator {
                pool_address: delegation_pool::get_owned_pool_address(@0x111),
                multipliers: RequestedMultipliers::V1 {
                    execution_multiplier_percent: 200,
                    io_multiplier_percent: 200
                }
            }
        );
    }
}
