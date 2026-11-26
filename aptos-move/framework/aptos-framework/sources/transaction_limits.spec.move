spec aptos_framework::transaction_limits {
    /// The fee payer must own, or be the delegated voter of, a stake pool in the current validator set, or the
    /// requested delegation pool must exist and be in it.
    spec validate_high_txn_limits(fee_payer: address, request: UserTxnLimitsRequest) {
        use aptos_framework::delegation_pool;
        pragma opaque;
        // Partial: the aborts of the stake lookups and of the multiplier and stake checks are not specified.
        pragma aborts_if_is_partial;
        let owner = request is UserTxnLimitsRequest::StakePoolOwner;
        let pool_address = if (owner) {
            global<stake::OwnerCapability>(fee_payer).pool_address
        } else {
            request.pool_address
        };
        aborts_if owner && !exists<stake::OwnerCapability>(fee_payer);
        aborts_if (request is UserTxnLimitsRequest::DelegatedVoter) && (!stake::spec_has_stake_pool(pool_address)
            || fee_payer != global<stake::StakePool>(pool_address).delegated_voter);
        aborts_if (request is UserTxnLimitsRequest::DelegationPoolDelegator)
            && !exists<delegation_pool::DelegationPool>(pool_address);
        aborts_if !stake::spec_is_current_epoch_validator(pool_address);
    }

    spec fun spec_valid_multiplier(multiplier_percent: u64): bool {
        multiplier_percent > MIN_MULTIPLIER_PERCENT && multiplier_percent <= MAX_MULTIPLIER_PERCENT
    }

    /// Tier `j` is ordered against tier `j - 1`.
    spec fun spec_tier_ordered(tiers: vector<TxnLimitTier>, j: u64): bool {
        tiers[j].min_stake >= tiers[j - 1].min_stake
            && tiers[j].multiplier_percent > tiers[j - 1].multiplier_percent
    }

    spec fun spec_tiers_valid(tiers: vector<TxnLimitTier>): bool {
        forall j in 1..len(tiers): spec_tier_ordered(tiers, j)
    }

    /// Some tier covers the requested multiplier.
    spec fun spec_has_tier(tiers: vector<TxnLimitTier>, multiplier_percent: u64): bool {
        exists i in 0..len(tiers): tiers[i].multiplier_percent >= multiplier_percent
    }

    /// Minimum stake of the first tier covering the requested multiplier.
    spec fun spec_min_stake_required(tiers: vector<TxnLimitTier>, multiplier_percent: u64): u64 {
        tiers[choose min i in 0..len(tiers) where tiers[i].multiplier_percent >= multiplier_percent].min_stake
    }

    /// `tiers` pairs `min_stakes` with `multipliers_percent` element-wise.
    spec fun spec_tiers_built_from(
        tiers: vector<TxnLimitTier>, min_stakes: vector<u64>, multipliers_percent: vector<u64>
    ): bool {
        len(tiers) == len(min_stakes)
            && (forall j in 0..len(tiers):
                tiers[j].min_stake == min_stakes[j] && tiers[j].multiplier_percent == multipliers_percent[j])
    }

    spec schema NewTiersAbortsIf {
        min_stakes: vector<u64>;
        multipliers_percent: vector<u64>;
        aborts_if len(min_stakes) != len(multipliers_percent) with error::INVALID_ARGUMENT;
        aborts_if exists j in 0..len(min_stakes): !spec_valid_multiplier(multipliers_percent[j])
            with error::INVALID_ARGUMENT;
        aborts_if exists j in 1..len(min_stakes):
            min_stakes[j] < min_stakes[j - 1] || multipliers_percent[j] <= multipliers_percent[j - 1]
            with error::INVALID_ARGUMENT;
    }

    /// Consecutive tiers must be ordered.
    spec validate_tiers(tiers: &vector<TxnLimitTier>) {
        pragma opaque;
        aborts_if !spec_tiers_valid(tiers) with error::INVALID_ARGUMENT;
    }

    /// Pairs the inputs into tiers; every multiplier must be valid and the tiers ordered.
    spec new_tiers(min_stakes: vector<u64>, multipliers_percent: vector<u64>): vector<TxnLimitTier> {
        pragma opaque;
        include NewTiersAbortsIf;
        ensures spec_tiers_built_from(result, min_stakes, multipliers_percent);
    }

    /// Returns the minimum stake of the first tier that covers the multiplier.
    spec find_min_stake_required(tiers: &vector<TxnLimitTier>, multiplier_percent: u64): u64 {
        pragma opaque;
        aborts_if !spec_has_tier(tiers, multiplier_percent) with error::INVALID_ARGUMENT;
        ensures result == spec_min_stake_required(tiers, multiplier_percent);
    }

    spec initialize(aptos_framework: &signer, execution_tiers: vector<TxnLimitTier>, io_tiers: vector<TxnLimitTier>) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if !spec_tiers_valid(execution_tiers);
        aborts_if !spec_tiers_valid(io_tiers);
        aborts_if exists<TxnLimitsConfig>(addr);
        ensures global<TxnLimitsConfig>(addr) == TxnLimitsConfig::V1 { execution_tiers, io_tiers };
    }

    /// Only the framework may update the tiers; each min-stake vector must match its multiplier vector in length,
    /// every multiplier must be valid and the tiers ordered.
    spec update_config(
        aptos_framework: &signer,
        execution_min_stakes: vector<u64>,
        execution_multipliers_percent: vector<u64>,
        io_min_stakes: vector<u64>,
        io_multipliers_percent: vector<u64>
    ) {
        use std::signer;
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        include NewTiersAbortsIf { min_stakes: execution_min_stakes, multipliers_percent: execution_multipliers_percent };
        include NewTiersAbortsIf { min_stakes: io_min_stakes, multipliers_percent: io_multipliers_percent };
        modifies global<TxnLimitsConfig>(@aptos_framework);
        let post config = global<TxnLimitsConfig>(@aptos_framework);
        ensures exists<TxnLimitsConfig>(@aptos_framework);
        ensures spec_tiers_built_from(config.execution_tiers, execution_min_stakes, execution_multipliers_percent);
        ensures spec_tiers_built_from(config.io_tiers, io_min_stakes, io_multipliers_percent);
    }

    /// Both multipliers must be in (1x, 100x] and covered by a tier, and the stake must reach both tiers' minimum
    /// stakes.
    spec validate_enough_stake(stake_amount: u64, multipliers: RequestedMultipliers) {
        let execution = multipliers.execution_multiplier_percent;
        let io = multipliers.io_multiplier_percent;
        let config = global<TxnLimitsConfig>(@aptos_framework);
        // Each guard holds iff all earlier checks passed, so every state maps to the code the body produces.
        let multipliers_ok = spec_valid_multiplier(execution) && spec_valid_multiplier(io);
        let config_ok = multipliers_ok && exists<TxnLimitsConfig>(@aptos_framework);
        let tiers_ok = config_ok && spec_has_tier(config.execution_tiers, execution)
            && spec_has_tier(config.io_tiers, io);
        aborts_if !spec_valid_multiplier(execution) with error::INVALID_ARGUMENT;
        aborts_if !spec_valid_multiplier(io) with error::INVALID_ARGUMENT;
        aborts_if multipliers_ok && !exists<TxnLimitsConfig>(@aptos_framework);
        aborts_if config_ok && !spec_has_tier(config.execution_tiers, execution) with error::INVALID_ARGUMENT;
        aborts_if config_ok && !spec_has_tier(config.io_tiers, io) with error::INVALID_ARGUMENT;
        aborts_if tiers_ok && stake_amount < spec_min_stake_required(config.execution_tiers, execution)
            with error::PERMISSION_DENIED;
        aborts_if tiers_ok && stake_amount < spec_min_stake_required(config.io_tiers, io)
            with error::PERMISSION_DENIED;
    }
}
