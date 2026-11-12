spec aptos_framework::transaction_limits {
    spec module {
        pragma verify = false;
    }

    /// The fee payer must own, or be the delegated voter of, a stake pool in the current validator set, or the
    /// requested delegation pool must exist and be in it.
    spec validate_high_txn_limits(fee_payer: address, request: UserTxnLimitsRequest) {
        use aptos_framework::delegation_pool;
        pragma verify = true;
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

    spec initialize(aptos_framework: &signer, execution_tiers: vector<TxnLimitTier>, io_tiers: vector<TxnLimitTier>) {
        use std::signer;
        pragma verify = true;
        // Partial: the aborts of the tier validation are not specified.
        pragma aborts_if_is_partial;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<TxnLimitsConfig>(addr);
        ensures global<TxnLimitsConfig>(addr) == TxnLimitsConfig::V1 { execution_tiers, io_tiers };
    }

    /// Only the framework may update the tiers; each min-stake vector must match its multiplier vector in length.
    spec update_config(
        aptos_framework: &signer,
        execution_min_stakes: vector<u64>,
        execution_multipliers_percent: vector<u64>,
        io_min_stakes: vector<u64>,
        io_multipliers_percent: vector<u64>
    ) {
        use std::signer;
        pragma verify = true;
        // Partial: the aborts of the tier construction and validation are not specified.
        pragma aborts_if_is_partial;
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        aborts_if len(execution_min_stakes) != len(execution_multipliers_percent);
        aborts_if len(io_min_stakes) != len(io_multipliers_percent);
        ensures exists<TxnLimitsConfig>(@aptos_framework);
    }

    /// Both multipliers must be in (1x, 100x].
    spec validate_enough_stake(stake_amount: u64, multipliers: RequestedMultipliers) {
        pragma verify = true;
        // Partial: the aborts of the tier matching and the stake thresholds are not specified.
        pragma aborts_if_is_partial;
        let execution = multipliers.execution_multiplier_percent;
        let io = multipliers.io_multiplier_percent;
        aborts_if execution <= MIN_MULTIPLIER_PERCENT || execution > MAX_MULTIPLIER_PERCENT;
        aborts_if io <= MIN_MULTIPLIER_PERCENT || io > MAX_MULTIPLIER_PERCENT;
        aborts_if !exists<TxnLimitsConfig>(@aptos_framework);
    }
}
