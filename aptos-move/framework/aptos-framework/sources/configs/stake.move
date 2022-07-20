/**
 * Validator lifecycle:
 * 1. Prepare a validator node set up and call Stake::register_validator_candidate
 * 2. Once ready to deposit stake (or have funds assigned by a staking service in exchange for ownership capability),
 * call Stake::add_stake and Stake::increase_lockup (or *_with_cap versions if called from the staking service)
 * 3. Call Stake::join_validator_set (or _with_cap version) to join the active validator set. Changes are effective in
 * the next epoch.
 * 4. Validate and gain rewards.
 * 5. At any point, if the validator operator wants to switch validator node operator, they can call
 * Stake::rotate_consensus_key.
 * 6. When lockup has expired, validator (or the owner of owner capability) can choose to either (1) increase the lockup
 * to keep validating and receiving rewards, or (2) call Stake::unlock to unlock their stake and Stake::withdraw to
 * withdraw in the next epoch.
 * 7. After exiting, the validator can either explicitly leave the validator set by calling Stake::leave_validator_set
 * or if their stake drops below the min required, they would get removed at the end of the epoch.
 * 8. Validator can always rejoin the validator set by going through steps 2-3 again.
 * 9. Owner can always switch operators by calling Stake::set_operator.
*/
module aptos_framework::stake {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::comparator;
    use aptos_std::event::{Self, EventHandle};
    use aptos_std::signature;
    use aptos_framework::governance_proposal::{GovernanceProposal};
    use aptos_framework::coin::{Self, Coin, MintCapability};
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::aptos_coin::AptosCoin;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    /// Lockup period is shorter than required.
    const ELOCK_TIME_TOO_SHORT: u64 = 1;
    /// Withdraw not allowed, the stake is still locked.
    const EWITHDRAW_NOT_ALLOWED: u64 = 2;
    /// Validator Config not published.
    const EVALIDATOR_CONFIG: u64 = 3;
    /// Not enough stake to join validator set.
    const ESTAKE_TOO_LOW: u64 = 4;
    /// Too much stake to join validator set.
    const ESTAKE_TOO_HIGH: u64 = 5;
    /// Account is already a validator or pending validator.
    const EALREADY_ACTIVE_VALIDATOR: u64 = 6;
    /// Account is not a validator.
    const ENOT_VALIDATOR: u64 = 7;
    /// Can't remove last validator.
    const ELAST_VALIDATOR: u64 = 8;
    /// Total stake exceeds maximum allowed.
    const ESTAKE_EXCEEDS_MAX: u64 = 9;
    /// Account is already registered as a validator candidate.
    const EALREADY_REGISTERED: u64 = 10;
    /// Account does not have the right ownership capability.
    const ENOT_OWNER: u64 = 11;
    /// No coins in inactive state to withdraw from specified pool.
    const ENO_COINS_TO_WITHDRAW: u64 = 12;
    /// Account does not have the right operator capability.
    const ENOT_OPERATOR: u64 = 13;
    /// Lockup period is longer than allowed.
    const ELOCK_TIME_TOO_LONG: u64 = 14;
    const ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED: u64 = 15;
    /// Invalid consensus public key
    const EINVALID_PUBLIC_KEY: u64 = 16;
    /// Invalid required stake range, usually happens if min > max.
    const EINVALID_STAKE_RANGE: u64 = 17;
    /// Invalid required stake lockup, usually happens if min > max.
    const EINVALID_LOCKUP_RANGE: u64 = 18;
    /// Invalid rewards rate.
    const EINVALID_REWARDS_RATE: u64 = 19;
    /// Invalid stake amount (usuaully 0).
    const EINVALID_STAKE_AMOUNT: u64 = 20;

    /// Validator status enum. We can switch to proper enum later once Move supports it.
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;
    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Capability that represents ownership and can be used to control the validator and the associated stake pool.
    /// Having this be separate from the signer for the account that the validator resources are hosted at allows
    /// modules to have control over a validator.
    struct OwnerCapability has key, store {
        pool_address: address,
    }

    /// Each validator has a separate StakePool resource and can provide a stake.
    /// Changes in stake for an active validator:
    /// 1. If a validator calls add_stake, the newly added stake is moved to pending_active.
    /// 2. If validator calls unlock, their stake is moved to pending_inactive.
    /// 2. When the next epoch starts, any pending_inactive stake is moved to inactive and can be withdrawn.
    ///    Any pending_active stake is moved to active and adds to the validator's voting power.
    ///
    /// Changes in stake for an inactive validator:
    /// 1. If a validator calls add_stake, the newly added stake is moved directly to active.
    /// 2. If validator calls unlock, their stake is moved directly to inactive.
    /// 3. When the next epoch starts, the validator can be activated if their active stake is more than the minimum.
    struct StakePool has key {
        // active stake
        active: Coin<AptosCoin>,
        // inactive stake, can be withdrawn
        inactive: Coin<AptosCoin>,
        // pending activation for next epoch
        pending_active: Coin<AptosCoin>,
        // pending deactivation for next epoch
        pending_inactive: Coin<AptosCoin>,
        locked_until_secs: u64,
        // Track the current operator of the validator node.
        // This allows the operator to be different from the original account and allow for separation of
        // the validator operations and ownership.
        // Only the account holding OwnerCapability of the staking pool can update this.
        operator_address: address,

        // Track the current vote delegator of the staking pool.
        // Only the account holding OwnerCapability of the staking pool can update this.
        delegated_voter: address,
    }

    /// Validator info stored in validator address.
    struct ValidatorConfig has key, copy, store, drop {
        consensus_pubkey: vector<u8>,
        network_addresses: vector<u8>,
        // to make it compatible with previous definition, remove later
        fullnode_addresses: vector<u8>,
        // Index in the active set if the validator corresponding to this stake pool is active.
        validator_index: u64,
    }

    /// Consensus information per validator, stored in ValidatorSet.
    struct ValidatorInfo has copy, store, drop {
        addr: address,
        voting_power: u64,
        config: ValidatorConfig,
    }

    /// Validator set configurations that will be stored with the @aptos_framework account.
    struct ValidatorSetConfiguration has key {
        // A validator needs to stake at least this amount to be able to join the validator set.
        // If after joining the validator set and at the start of any epoch, a validator's stake drops below this amount
        // they will be removed from the set.
        minimum_stake: u64,
        // A validator can only stake at most this amount. Any larger stake will be rejected.
        // If after joining the validator set and at the start of any epoch, a validator's stake exceeds this amount,
        // their voting power and rewards would only be issued for the max stake amount.
        maximum_stake: u64,
        // A validator needs to initially lock up for at least this amount of time (in secs) to be able to join the
        // validator set. However, if over time, their remaining lockup drops below this, they won't get removed from
        // the validator set.
        min_lockup_duration_secs: u64,
        // A validator can only lock up for at most this amount of time (in secs).
        max_lockup_duration_secs: u64,
        // Whether validators are allow to join/leave post genesis.
        allow_validator_set_change: bool,
        // The maximum rewards given out every epoch. This will be divided by the rewards rate denominator.
        // For example, 0.001% (0.00001) can be represented as 10 / 1000000.
        rewards_rate: u64,
        rewards_rate_denominator: u64,
    }

    /// Full ValidatorSet, stored in @aptos_framework.
    /// 1. join_validator_set adds to pending_active queue.
    /// 2. leave_valdiator_set moves from active to pending_inactive queue.
    /// 3. on_new_epoch processes two pending queues and refresh ValidatorInfo from the owner's address.
    struct ValidatorSet has key {
        consensus_scheme: u8,
        // active validators for the current epoch
        active_validators: vector<ValidatorInfo>,
        // pending validators to leave in next epoch (still active)
        pending_inactive: vector<ValidatorInfo>,
        // pending validators to join in next epoch
        pending_active: vector<ValidatorInfo>,
    }

    /// AptosCoin capabilities, set during genesis and stored in @CoreResource account.
    /// This allows the Stake module to mint rewards to stakers.
    struct AptosCoinCapabilities has key {
        mint_cap: MintCapability<AptosCoin>,
    }

    struct ValidatorPerformance has key {
        num_blocks: u64,
        missed_votes: vector<u64>,
    }

    /// The events emitted for the entire StakePool's lifecycle.
    struct StakePoolEvents has key {
        register_validator_candidate_events: EventHandle<RegisterValidatorCandidateEvent>,
        set_operator_events: EventHandle<SetOperatorEvent>,
        add_stake_events: EventHandle<AddStakeEvent>,
        rotate_consensus_key_events: EventHandle<RotateConsensusKeyEvent>,
        update_network_and_fullnode_addresses_events: EventHandle<UpdateNetworkAndFullnodeAddressesEvent>,
        increase_lockup_events: EventHandle<IncreaseLockupEvent>,
        join_validator_set_events: EventHandle<JoinValidatorSetEvent>,
        distribute_rewards_events: EventHandle<DistributeRewardsEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: EventHandle<WithdrawStakeEvent>,
        leave_validator_set_events: EventHandle<LeaveValidatorSetEvent>,
    }

    struct RegisterValidatorCandidateEvent has drop, store {
        pool_address: address,
    }

    struct SetOperatorEvent has drop, store {
        pool_address: address,
        old_operator: address,
        new_operator: address,
    }

    struct AddStakeEvent has drop, store {
        pool_address: address,
        amount_added: u64,
    }

    struct RotateConsensusKeyEvent has drop, store {
        pool_address: address,
        old_consensus_pubkey: vector<u8>,
        new_consensus_pubkey: vector<u8>,
    }

    struct UpdateNetworkAndFullnodeAddressesEvent has drop, store {
        pool_address: address,
        old_network_addresses: vector<u8>,
        new_network_addresses: vector<u8>,
        old_fullnode_addresses: vector<u8>,
        new_fullnode_addresses: vector<u8>,
    }

    struct IncreaseLockupEvent has drop, store {
        pool_address: address,
        old_locked_until_secs: u64,
        new_locked_until_secs: u64,
    }

    struct JoinValidatorSetEvent has drop, store {
        pool_address: address,
    }

    struct DistributeRewardsEvent has drop, store {
        pool_address: address,
        rewards_amount: u64,
    }

    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        amount_unlocked: u64,
    }

    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        amount_withdrawn: u64,
    }

    struct LeaveValidatorSetEvent has drop, store {
        pool_address: address,
    }

    /// Return the lockup expiration of the stake pool at `pool_address`.
    /// This will throw an error if there's no stake pool at `pool_address`.
    public fun get_lockup_secs(pool_address: address): u64 acquires StakePool {
        borrow_global<StakePool>(pool_address).locked_until_secs
    }

    /// Return the different stake amounts for `pool_address` (whether the validator is active or not).
    /// The returned amounts are for (active, inactive, pending_active, pending_inactive) stake respectively.
    public fun get_stake(pool_address: address): (u64, u64, u64, u64) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(pool_address);
        (
            coin::value<AptosCoin>(&stake_pool.active),
            coin::value<AptosCoin>(&stake_pool.inactive),
            coin::value<AptosCoin>(&stake_pool.pending_active),
            coin::value<AptosCoin>(&stake_pool.pending_inactive),
        )
    }

    public fun get_validator_state(pool_address: address): u64 acquires ValidatorSet {
        let validator_set = borrow_global<ValidatorSet>(@aptos_framework);
        if (option::is_some(&find_validator(&validator_set.pending_active, pool_address))) {
            VALIDATOR_STATUS_PENDING_ACTIVE
        } else if (option::is_some(&find_validator(&validator_set.active_validators, pool_address))) {
            VALIDATOR_STATUS_ACTIVE
        } else if (option::is_some(&find_validator(&validator_set.pending_inactive, pool_address))) {
            VALIDATOR_STATUS_PENDING_INACTIVE
        } else {
            VALIDATOR_STATUS_INACTIVE
        }
    }

    /// Return the active staked balance of the stake pool at `pool_address`. Any pending_inactive and pending_active
    /// stake are not considered.
    ///
    /// If the stake pool is not yet active, the voting power will be 0.
    public fun get_active_staked_balance(pool_address: address): u64 acquires StakePool, ValidatorSet {
        if (get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE) {
            0
        } else {
            coin::value<AptosCoin>(&borrow_global<StakePool>(pool_address).active)
        }
    }

    /// Return the delegated voter of the validator at `pool_address`.
    public fun get_delegated_voter(pool_address: address): address acquires StakePool {
        borrow_global<StakePool>(pool_address).delegated_voter
    }

    /// Return the operator of the validator at `pool_address`.
    public fun get_operator(pool_address: address): address acquires StakePool {
        borrow_global<StakePool>(pool_address).operator_address
    }

    /// Update the min and max stake amounts.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_required_stake(
        _gov_proposal: &GovernanceProposal,
        minimum_stake: u64,
        maximum_stake: u64,
    ) acquires ValidatorSetConfiguration {
        validate_required_stake(minimum_stake, maximum_stake);

        let validator_set_config = borrow_global_mut<ValidatorSetConfiguration>(@aptos_framework);
        validator_set_config.minimum_stake = minimum_stake;
        validator_set_config.maximum_stake = maximum_stake;
    }

    /// Update the min and max lockup duration.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_required_lockup(
        _gov_proposal: &GovernanceProposal,
        min_lockup_duration_secs: u64,
        max_lockup_duration_secs: u64,
    ) acquires ValidatorSetConfiguration {
        validate_required_lockup(min_lockup_duration_secs, max_lockup_duration_secs);

        let validator_set_config = borrow_global_mut<ValidatorSetConfiguration>(@aptos_framework);
        validator_set_config.min_lockup_duration_secs = min_lockup_duration_secs;
        validator_set_config.max_lockup_duration_secs = max_lockup_duration_secs;
    }

    /// Update the rewards rate.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_rewards_rate(
        _gov_proposal: &GovernanceProposal,
        new_rewards_rate: u64,
        new_rewards_rate_denominator: u64,
    ) acquires ValidatorSetConfiguration {
        assert!(
            new_rewards_rate_denominator > 0,
            error::invalid_argument(EINVALID_REWARDS_RATE),
        );

        let validator_set_config = borrow_global_mut<ValidatorSetConfiguration>(@aptos_framework);
        validator_set_config.rewards_rate = new_rewards_rate;
        validator_set_config.rewards_rate_denominator = new_rewards_rate_denominator;
    }

    /// Initialize validator set to the core resource account.
    public fun initialize_validator_set(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        min_lockup_duration_secs: u64,
        max_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        // This can fail genesis but is necessary so that any misconfigurations can be corrected before genesis succeeds
        validate_required_stake(minimum_stake, maximum_stake);
        validate_required_lockup(min_lockup_duration_secs, max_lockup_duration_secs);
        assert!(
            rewards_rate_denominator > 0,
            error::invalid_argument(EINVALID_REWARDS_RATE),
        );

        move_to(aptos_framework, ValidatorSet {
            consensus_scheme: 0,
            active_validators: vector::empty(),
            pending_active: vector::empty(),
            pending_inactive: vector::empty(),
        });
        move_to(aptos_framework, ValidatorSetConfiguration {
            minimum_stake,
            maximum_stake,
            min_lockup_duration_secs,
            max_lockup_duration_secs,
            allow_validator_set_change,
            rewards_rate,
            rewards_rate_denominator,
        });
        move_to(aptos_framework, ValidatorPerformance {
            num_blocks: 0,
            missed_votes: vector::empty(),
        });
    }

    /// This is only called during Genesis, which is where MintCapability<AptosCoin> can be created.
    /// Beyond genesis, no one can create AptosCoin mint/burn capabilities.
    public fun store_aptos_coin_mint_cap(account: &signer, mint_cap: MintCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, AptosCoinCapabilities { mint_cap })
    }

    /// Initialize the validator account and give ownership to the signing account.
    public entry fun register_validator_candidate(
        account: &signer,
        consensus_pubkey: vector<u8>,
        proof_of_possession: vector<u8>,
        network_addresses: vector<u8>,
        fullnode_addresses: vector<u8>,
    ) {
        let account_address = signer::address_of(account);
        assert!(!exists<StakePool>(account_address), error::invalid_argument(EALREADY_REGISTERED));
        assert!(signature::bls12381_validate_pubkey(consensus_pubkey, proof_of_possession), error::invalid_argument(EINVALID_PUBLIC_KEY));

        move_to(account, StakePool {
            active: coin::zero<AptosCoin>(),
            pending_active: coin::zero<AptosCoin>(),
            pending_inactive: coin::zero<AptosCoin>(),
            inactive: coin::zero<AptosCoin>(),
            locked_until_secs: 0,
            operator_address: account_address,
            delegated_voter: account_address,
        });
        move_to(account, StakePoolEvents {
            register_validator_candidate_events: event::new_event_handle<RegisterValidatorCandidateEvent>(account),
            set_operator_events: event::new_event_handle<SetOperatorEvent>(account),
            add_stake_events: event::new_event_handle<AddStakeEvent>(account),
            rotate_consensus_key_events: event::new_event_handle<RotateConsensusKeyEvent>(account),
            update_network_and_fullnode_addresses_events: event::new_event_handle<UpdateNetworkAndFullnodeAddressesEvent>(account),
            increase_lockup_events: event::new_event_handle<IncreaseLockupEvent>(account),
            join_validator_set_events: event::new_event_handle<JoinValidatorSetEvent>(account),
            distribute_rewards_events: event::new_event_handle<DistributeRewardsEvent>(account),
            unlock_stake_events: event::new_event_handle<UnlockStakeEvent>(account),
            withdraw_stake_events: event::new_event_handle<WithdrawStakeEvent>(account),
            leave_validator_set_events: event::new_event_handle<LeaveValidatorSetEvent>(account),
        });
        move_to(account, ValidatorConfig {
            consensus_pubkey,
            network_addresses,
            fullnode_addresses,
            validator_index: 0,
        });
        move_to(account, OwnerCapability {
           pool_address: account_address,
        });
    }

    /// Extract and return owner capability from the signing account.
    public fun extract_owner_cap(account: &signer): OwnerCapability acquires OwnerCapability {
        move_from<OwnerCapability>(signer::address_of(account))
    }

    /// Deposit `owner_cap` into `account`. This requires `account` to not already have owernship of another
    /// staking pool.
    public fun deposit_owner_cap(account: &signer, owner_cap: OwnerCapability) {
        move_to(account, owner_cap);
    }

    /// Allows an owner to change the operator of the stake pool.
    public entry fun set_operator(
        account: &signer,
        new_operator: address,
    ) acquires OwnerCapability, StakePool, StakePoolEvents {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        set_operator_with_cap(account_addr, ownership_cap, new_operator);
    }

    /// Allows an account with ownership capability to change the operator of the stake pool.
    public fun set_operator_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        new_operator: address,
    ) acquires StakePool, StakePoolEvents {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let old_operator = stake_pool.operator_address;
        stake_pool.operator_address = new_operator;

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<SetOperatorEvent>(
            &mut stake_pool_events.set_operator_events,
            SetOperatorEvent {
                pool_address,
                old_operator,
                new_operator,
            },
        );
    }

    /// Allows an owner to change the delegated voter of the stake pool.
    public entry fun set_delegated_voter(
        account: &signer,
        new_delegated_voter: address,
    ) acquires OwnerCapability, StakePool {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        set_delegated_voter_with_cap(account_addr, ownership_cap, new_delegated_voter);
    }

    /// Allows an owner to change the delegated voter of the stake pool.
    public entry fun set_delegated_voter_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        new_delegated_voter: address,
    ) acquires StakePool {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        stake_pool.delegated_voter = new_delegated_voter;
    }

    /// Add `amount` of coins from the `account` owning the StakePool.
    public entry fun add_stake(
        account: &signer,
        amount: u64,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, ValidatorSet, ValidatorSetConfiguration {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        add_stake_with_cap(account_addr, ownership_cap, coin::withdraw<AptosCoin>(account, amount));
    }

    /// Add `coins` into `pool_address`. this requires the corresponding `owner_cap` to be passed in.
    public fun add_stake_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        coins: Coin<AptosCoin>,
    ) acquires StakePool, StakePoolEvents, ValidatorSet, ValidatorSetConfiguration {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));

        let amount = coin::value<AptosCoin>(&coins);
        assert!(amount > 0, error::invalid_argument(EINVALID_STAKE_AMOUNT));

        // Add to pending_active if it's a current validator because the stake is not counted until the next epoch.
        // Otherwise, the delegation can be added to active directly as the validator is also activated in the epoch.
        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        if (is_current_epoch_validator(pool_address)) {
            coin::merge<AptosCoin>(&mut stake_pool.pending_active, coins);
        } else {
            coin::merge<AptosCoin>(&mut stake_pool.active, coins);
        };

        let maximum_stake = borrow_global<ValidatorSetConfiguration>(@aptos_framework).maximum_stake;
        let total_stake = coin::value<AptosCoin>(&stake_pool.active) +
                          coin::value<AptosCoin>(&stake_pool.pending_active);
        assert!(total_stake <= maximum_stake, error::invalid_argument(ESTAKE_EXCEEDS_MAX));

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<AddStakeEvent>(
            &mut stake_pool_events.add_stake_events,
            AddStakeEvent {
                pool_address,
                amount_added: amount,
            },
        );
    }

    /// Rotate the consensus key of the validator, it'll take effect in next epoch.
    public entry fun rotate_consensus_key(
        account: &signer,
        pool_address: address,
        new_consensus_pubkey: vector<u8>,
        proof_of_possession: vector<u8>,
    ) acquires StakePool, StakePoolEvents, ValidatorConfig {
        let stake_pool = borrow_global<StakePool>(pool_address);
        assert!(signer::address_of(account) == stake_pool.operator_address, error::invalid_argument(ENOT_OPERATOR));

        assert!(exists<ValidatorConfig>(pool_address), error::not_found(EVALIDATOR_CONFIG));
        let validator_info = borrow_global_mut<ValidatorConfig>(pool_address);
        let old_consensus_pubkey = validator_info.consensus_pubkey;
        assert!(signature::bls12381_validate_pubkey(new_consensus_pubkey, proof_of_possession), error::invalid_argument(EINVALID_PUBLIC_KEY));
        validator_info.consensus_pubkey = new_consensus_pubkey;

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<RotateConsensusKeyEvent>(
            &mut stake_pool_events.rotate_consensus_key_events,
            RotateConsensusKeyEvent {
                pool_address,
                old_consensus_pubkey,
                new_consensus_pubkey,
            },
        );
    }

    /// Update the network and full node addresses of the validator. This only takes effect in the next epoch.
    public entry fun update_network_and_fullnode_addresses(
        account: &signer,
        pool_address: address,
        new_network_addresses: vector<u8>,
        new_fullnode_addresses: vector<u8>,
    ) acquires StakePool, StakePoolEvents, ValidatorConfig {
        let stake_pool = borrow_global<StakePool>(pool_address);
        assert!(signer::address_of(account) == stake_pool.operator_address, error::invalid_argument(ENOT_OPERATOR));

        assert!(exists<ValidatorConfig>(pool_address), error::not_found(EVALIDATOR_CONFIG));
        let validator_info = borrow_global_mut<ValidatorConfig>(pool_address);
        let old_network_addresses = validator_info.network_addresses;
        validator_info.network_addresses = new_network_addresses;
        let old_fullnode_addresses = validator_info.fullnode_addresses;
        validator_info.fullnode_addresses = new_fullnode_addresses;

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<UpdateNetworkAndFullnodeAddressesEvent>(
            &mut stake_pool_events.update_network_and_fullnode_addresses_events,
            UpdateNetworkAndFullnodeAddressesEvent {
                pool_address,
                old_network_addresses,
                new_network_addresses,
                old_fullnode_addresses,
                new_fullnode_addresses,
            },
        );
    }

    /// Similar to increase_lockup_with_cap but will use ownership capability from the signing account.
    public entry fun increase_lockup(
        account: &signer,
        new_locked_until_secs: u64,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, ValidatorSetConfiguration {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        increase_lockup_with_cap(account_addr, ownership_cap, new_locked_until_secs);
    }

    /// Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly inactive if it's not from an active validator.
    public fun increase_lockup_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        new_locked_until_secs: u64,
    ) acquires StakePool, StakePoolEvents, ValidatorSetConfiguration {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));
        let validator_set_config = borrow_global<ValidatorSetConfiguration>(@aptos_framework);
        validate_lockup_time(new_locked_until_secs, validator_set_config);

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let old_locked_until_secs = stake_pool.locked_until_secs;
        stake_pool.locked_until_secs = new_locked_until_secs;

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<IncreaseLockupEvent>(
            &mut stake_pool_events.increase_lockup_events,
            IncreaseLockupEvent {
                pool_address,
                old_locked_until_secs,
                new_locked_until_secs,
            },
        );
    }

    /// This can only called by the operator of the validator/staking pool.
    public entry fun join_validator_set(
        account: &signer,
        pool_address: address,
    ) acquires StakePool, StakePoolEvents, ValidatorConfig, ValidatorSetConfiguration, ValidatorSet {
        let validator_set_config = borrow_global<ValidatorSetConfiguration>(@aptos_framework);
        assert!(
            validator_set_config.allow_validator_set_change,
            error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),
        );

        join_validator_set_internal(account, pool_address);
    }

    /// Request to have `pool_address` join the validator set. Can only be called after calling `register_validator_candidate`.
    /// If the validator has the required stake (more than minimum and less than maximum allowed), they will be
    /// added to the pending_active queue. All validators in this queue will be added to the active set when the next
    /// epoch starts (eligibility will be rechecked).
    ///
    /// This internal version can only be called by the Genesis module during Genesis.
    public(friend) fun join_validator_set_internal(
        account: &signer,
        pool_address: address,
    ) acquires StakePool, StakePoolEvents, ValidatorConfig, ValidatorSet, ValidatorSetConfiguration {
        let stake_pool = borrow_global<StakePool>(pool_address);
        // Account has to be the operator.
        assert!(signer::address_of(account) == stake_pool.operator_address, error::invalid_argument(ENOT_OPERATOR));

        // Requires the validator to have a lockup more than the minimum.
        let validator_set_config = borrow_global<ValidatorSetConfiguration>(@aptos_framework);
        validate_lockup_time(stake_pool.locked_until_secs, validator_set_config);

        // Throw an error is the validator is already active.
        assert!(
            get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE,
            error::invalid_argument(EALREADY_ACTIVE_VALIDATOR),
        );

        // The validator is not yet activated so all added stake should be in active.
        let voting_power = coin::value<AptosCoin>(&stake_pool.active);
        assert!(voting_power >= validator_set_config.minimum_stake, error::invalid_argument(ESTAKE_TOO_LOW));
        assert!(voting_power <= validator_set_config.maximum_stake, error::invalid_argument(ESTAKE_TOO_HIGH));

        // Add validator to pending_active, to be activated in the next epoch.
        let validator_config = borrow_global_mut<ValidatorConfig>(pool_address);
        let validator_set = borrow_global_mut<ValidatorSet>(@aptos_framework);
        vector::push_back(&mut validator_set.pending_active, generate_validator_info(pool_address, *validator_config));

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<JoinValidatorSetEvent>(
            &mut stake_pool_events.join_validator_set_events,
            JoinValidatorSetEvent { pool_address },
        );
    }

    /// Similar to unlock_with_cap but will use ownership capability from the signing account.
    public entry fun unlock(account: &signer, amount: u64) acquires OwnerCapability, StakePool, StakePoolEvents, ValidatorSet {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        unlock_with_cap(account_addr, amount, ownership_cap);
    }

    /// Unlock `amount` from the active stake. Only possible if the lockup has expired.
    public fun unlock_with_cap(
        pool_address: address,
        amount: u64,
        owner_cap: &OwnerCapability,
    ) acquires StakePool, StakePoolEvents, ValidatorSet {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));

        // Short-circuit if amount to unlock is 0 so we don't emit events.
        if (amount == 0) {
            return
        };

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let unlocked_stake = coin::extract<AptosCoin>(&mut stake_pool.active, amount);

        // Withdrawal is not allowed if lockup has not expired.
        // This is also enforced even if the validator is not active in order to prevent a validator from intentionally
        // leave the set early to be able to unlock.
        if (stake_pool.locked_until_secs > timestamp::now_seconds()) {
            abort error::invalid_argument(EWITHDRAW_NOT_ALLOWED)
        };

        // Move requested amount to pending_inactive if the validator is already active.
        // Otherwise, we can move directly to inactive.
        if (is_current_epoch_validator(pool_address)) {
            coin::merge<AptosCoin>(&mut stake_pool.pending_inactive, unlocked_stake);
        } else {
            coin::merge<AptosCoin>(&mut stake_pool.inactive, unlocked_stake);
        };

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<UnlockStakeEvent>(
            &mut stake_pool_events.unlock_stake_events,
            UnlockStakeEvent {
                pool_address,
                amount_unlocked: amount,
            },
        );
    }

    /// Withdraw from `account`'s inactive stake.
    public entry fun withdraw(account: &signer) acquires OwnerCapability, StakePool, StakePoolEvents {
        let account_addr = signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        let coins = withdraw_with_cap(account_addr, ownership_cap);
        coin::deposit<AptosCoin>(account_addr, coins);
    }

    /// Withdraw from `pool_address`'s inactive stake with the corresponding `owner_cap`.
    public fun withdraw_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
    ): Coin<AptosCoin> acquires StakePool, StakePoolEvents {
        assert!(owner_cap.pool_address == pool_address, error::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let withdraw_amount = coin::value<AptosCoin>(&stake_pool.inactive);
        assert!(withdraw_amount > 0, error::invalid_argument(ENO_COINS_TO_WITHDRAW));

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<WithdrawStakeEvent>(
            &mut stake_pool_events.withdraw_stake_events,
            WithdrawStakeEvent {
                pool_address,
                amount_withdrawn: withdraw_amount,
            },
        );

        coin::extract<AptosCoin>(&mut stake_pool.inactive, withdraw_amount)
    }

    /// Request to have `pool_address` leave the validator set. The validator is only actually removed from the set when
    /// the next epoch starts.
    /// The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
    /// is still operational.
    ///
    /// Can only be called by the operator of the validator/staking pool.
    public entry fun leave_validator_set(
        account: &signer,
        pool_address: address,
    ) acquires StakePool, StakePoolEvents, ValidatorSet, ValidatorSetConfiguration {
        let validator_set_config = borrow_global_mut<ValidatorSetConfiguration>(@aptos_framework);
        assert!(
            validator_set_config.allow_validator_set_change,
            error::invalid_argument(ENO_POST_GENESIS_VALIDATOR_SET_CHANGE_ALLOWED),
        );
        let stake_pool = borrow_global<StakePool>(pool_address);
        // Account has to be the operator.
        assert!(signer::address_of(account) == stake_pool.operator_address, error::invalid_argument(ENOT_OPERATOR));

        let validator_set = borrow_global_mut<ValidatorSet>(@aptos_framework);
        // Validate that the validator is already part of the validator set.
        let maybe_index = find_validator(&validator_set.active_validators, pool_address);
        assert!(option::is_some(&maybe_index), error::invalid_argument(ENOT_VALIDATOR));
        let index = option::extract(&mut maybe_index);

        let validator_info = vector::swap_remove(&mut validator_set.active_validators, index);
        assert!(vector::length(&validator_set.active_validators) > 0, error::invalid_argument(ELAST_VALIDATOR));
        vector::push_back(&mut validator_set.pending_inactive, validator_info);

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<LeaveValidatorSetEvent>(
            &mut stake_pool_events.leave_validator_set_events,
            LeaveValidatorSetEvent {
                pool_address,
            },
        );
    }

    /// Returns if the current validator can still vote in the current epoch.
    /// This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
    /// when the epoch starts.
    public fun is_current_epoch_validator(addr: address): bool acquires ValidatorSet {
        let validator_set = borrow_global<ValidatorSet>(@aptos_framework);
        option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
        option::is_some(&find_validator(&validator_set.pending_inactive, addr))
    }

    /// Update the number of missed votes. This is only called by block::prologue().
    /// This function cannot abort.
    public(friend) fun update_performance_statistics(missed_votes: vector<u64>) acquires ValidatorPerformance {
        // Validator set cannot change until the end of the epoch, so the validator index in list of missed votes should
        // match with those of the missed vote counts in ValidatorPerformance resource.
        let validator_perf = borrow_global_mut<ValidatorPerformance>(@aptos_framework);
        let validator_missed_votes_counts = &mut validator_perf.missed_votes;
        let i = 0;
        let len = vector::length(&missed_votes);
        while (i < len) {
            let validator_index = *vector::borrow(&missed_votes, i);
            // Skip any validator indices that are out of bounds, this ensures that this function doesn't abort if there
            // are out of bounds errors.
            if (validator_index < vector::length(validator_missed_votes_counts)) {
                let missed_votes_count = vector::borrow_mut(validator_missed_votes_counts, validator_index);
                *missed_votes_count = *missed_votes_count + 1;
            };
            i = i + 1;
        };
        validator_perf.num_blocks = validator_perf.num_blocks + 1;
    }

    /// Triggers at epoch boundary. This function shouldn't abort.
    ///
    /// 1. Distribute rewards to stake pools of active and pending inactive validators (requested to leave but not yet
    /// removed).
    /// 2. Officially move pending active stake to active and move pending inactive stake to inactive.
    /// The staking pool's voting power in this new epoch will be updated to the total active stake.
    /// 3. Add pending active validators to the active set if they satisfy requirements so they can vote and remove
    /// pending inactive validators so they no longer can vote.
    /// 4. The validator's voting power in the validator set is updated to be the corresponding staking pool's voting
    /// power.
    public(friend) fun on_new_epoch() acquires StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        let validator_set = borrow_global_mut<ValidatorSet>(@aptos_framework);
        let validator_set_config = borrow_global_mut<ValidatorSetConfiguration>(@aptos_framework);
        let validator_perf = borrow_global_mut<ValidatorPerformance>(@aptos_framework);

        // Process pending stake and distribute rewards for each currently active validator.
        let i = 0;
        let len = vector::length(&validator_set.active_validators);
        while (i < len) {
            let validator = vector::borrow(&validator_set.active_validators, i);
            update_stake_pool(validator, validator_perf, validator.addr, validator_set_config);
            i = i + 1;
        };

        // Process pending stake and distribute rewards for each currently pending_inactive validator (requested to
        // leave but not removed yet).
        let i = 0;
        let len = vector::length(&validator_set.pending_inactive);
        while (i < len) {
            let validator = vector::borrow(&validator_set.pending_inactive, i);
            update_stake_pool(validator, validator_perf, validator.addr, validator_set_config);
            i = i + 1;
        };

        // Activate currently pending_active validators.
        append(&mut validator_set.active_validators, &mut validator_set.pending_active);

        // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.
        validator_set.pending_inactive = vector::empty();

        // Reset performance scores and update active validator set (so network address/public key change takes effect).
        let i = 0;
        let len = vector::length(&validator_set.active_validators);
        let active_validators = vector::empty();
        validator_perf.num_blocks = 0;
        validator_perf.missed_votes = vector::empty();
        while (i < len) {
            let old_validator_info = vector::borrow_mut(&mut validator_set.active_validators, i);
            let pool_address = old_validator_info.addr;
            let validator_config = borrow_global_mut<ValidatorConfig>(pool_address);
            let new_validator_info = generate_validator_info(pool_address, *validator_config);

            // Restrict a validator's voting power to the max stake allowed.
            if (new_validator_info.voting_power > validator_set_config.maximum_stake) {
                new_validator_info.voting_power = validator_set_config.maximum_stake;
            };

            // A validator needs at least the min stake required to join the validator set.
            if (new_validator_info.voting_power >= validator_set_config.minimum_stake) {
                vector::push_back(&mut active_validators, new_validator_info);
                vector::push_back(&mut validator_perf.missed_votes, 0);
            };
            i = i + 1;
        };

        // Sort the validator set, so we can ensure stable order and validator indices.
        sort_validators(&mut active_validators);

        // Store the validator index in the validator's ValidatorConfig resource, so we can look it up later.
        // We can't rely on the validator set in CoreResource's ValidatorSet because some validators might choose
        // to leave and get moved to pending_inactive later on.
        let i = 0;
        let len = vector::length(&active_validators);
        while (i < len) {
            let validator_info = vector::borrow_mut(&mut active_validators, i);
            let pool_address = validator_info.addr;
            let validator_config = borrow_global_mut<ValidatorConfig>(pool_address);
            validator_config.validator_index = i;
            validator_info.config.validator_index = i;
            i = i + 1;
        };

        validator_set.active_validators = active_validators;
    }

    fun sort_validators(validators: &mut vector<ValidatorInfo>) {
        let length = vector::length(validators);
        if (length == 0) {
            return
        };

        let ordered = false;
        while (!ordered) {
            ordered = true;
            let idx = 0;
            while (idx < length - 1) {
                let left = vector::borrow(validators, idx);
                let right = vector::borrow(validators, idx + 1);
                if (comparator::is_greater_than(&comparator::compare(left, right))) {
                    ordered = false;
                    vector::swap(validators, idx, idx + 1);
                };
                idx = idx + 1;
            }
        }
    }

    /// Update individual validator's stake pool
    /// 1. distribute rewards to active/pending_inactive delegations
    /// 2. process pending_active, pending_inactive correspondingly
    /// This function shouldn't abort.
    fun update_stake_pool(
        validator: &ValidatorInfo,
        validator_perf: &ValidatorPerformance,
        pool_address: address,
        validator_set_config: &ValidatorSetConfiguration,
    ) acquires StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig {
        let validator_config = borrow_global<ValidatorConfig>(pool_address);
        let num_missed_votes = *vector::borrow(&validator_perf.missed_votes, validator_config.validator_index);
        let num_blocks = validator_perf.num_blocks;
        let num_successful_votes = num_blocks - num_missed_votes;

        let current_time = timestamp::now_seconds();
        // Remaining lockup time needs to be at least one so the rewards calculation still works.
        let remaining_lockup_time = 1;
        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        if (current_time < stake_pool.locked_until_secs) {
            remaining_lockup_time = stake_pool.locked_until_secs - current_time;
        };

        // Always merge rewards to active. If a validator's remaining lockup has already expired and they're withdrawing
        // coins, their rewards would be 0 anyway as remaining lockup = 0.
        let rewards = mint_reward(
            validator.voting_power,
            num_blocks,
            num_successful_votes,
            remaining_lockup_time,
            validator_set_config,
        );
        let rewards_amount = coin::value<AptosCoin>(&rewards);
        coin::merge(&mut stake_pool.active, rewards);

        // Process any pending active or inactive stakes.
        coin::merge<AptosCoin>(&mut stake_pool.active, coin::extract_all<AptosCoin>(&mut stake_pool.pending_active));
        coin::merge<AptosCoin>(&mut stake_pool.inactive, coin::extract_all<AptosCoin>(&mut stake_pool.pending_inactive));

        let stake_pool_events = borrow_global_mut<StakePoolEvents>(pool_address);
        event::emit_event<DistributeRewardsEvent>(
            &mut stake_pool_events.distribute_rewards_events,
            DistributeRewardsEvent {
                pool_address,
                rewards_amount,
            },
        );
    }

    /// Mint reward corresponding to current epoch's `voting_power` and `num_successful_votes` and
    /// `remaining_lockup_time`.
    fun mint_reward(
        voting_power: u64,
        num_blocks: u64,
        num_successful_votes: u64,
        remaining_lockup_time: u64,
        validator_set_config: &ValidatorSetConfiguration,
    ): Coin<AptosCoin> acquires AptosCoinCapabilities {
        // Validators receive rewards based on their performance (number of successful votes) and how long is their
        // remaining lockup time.
        // The total rewards = base rewards * performance multiplier * lockup multiplier.
        // Here we do multiplication before division to minimize rounding errors.
        let base_rewards =
            voting_power * validator_set_config.rewards_rate / validator_set_config.rewards_rate_denominator;
        let rewards_denominator = num_blocks * validator_set_config.max_lockup_duration_secs;
        let rewards_amount = base_rewards * num_successful_votes * remaining_lockup_time / rewards_denominator;
        if (rewards_amount > 0) {
            let mint_cap = &borrow_global<AptosCoinCapabilities>(@aptos_framework).mint_cap;
            return coin::mint<AptosCoin>(rewards_amount, mint_cap)
        };
        coin::zero<AptosCoin>()
    }

    fun append<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!vector::is_empty(v2)) {
            vector::push_back(v1, vector::pop_back(v2));
        }
    }

    fun find_validator(v: &vector<ValidatorInfo>, addr: address): Option<u64> {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            if (vector::borrow(v, i).addr == addr) {
                return option::some(i)
            };
            i = i + 1;
        };
        option::none()
    }

    fun generate_validator_info(addr: address, config: ValidatorConfig): ValidatorInfo acquires StakePool {
        let stake_pool = borrow_global<StakePool>(addr);
        // As pending_active validators have all been activated. The new epoch's voting power is just the pool's
        // total active stake.
        let voting_power = coin::value<AptosCoin>(&stake_pool.active);
        ValidatorInfo {
            addr,
            voting_power,
            config,
        }
    }

    /// Validate that the lockup time is at least more than the minimum required.
    fun validate_lockup_time(locked_until_secs: u64, validator_set_config: &ValidatorSetConfiguration) {
        let current_time = timestamp::now_seconds();
        // Short-circuit early if current_time is 0. This only happens during Genesis before a first block
        // is produced.
        if (current_time == 0) {
            return
        };

        assert!(
            current_time + validator_set_config.min_lockup_duration_secs <= locked_until_secs,
            error::invalid_argument(ELOCK_TIME_TOO_SHORT),
        );
        assert!(
            locked_until_secs <= current_time + validator_set_config.max_lockup_duration_secs,
            error::invalid_argument(ELOCK_TIME_TOO_LONG),
        );
    }

    fun validate_required_stake(minimum_stake: u64, maximum_stake: u64) {
        assert!(minimum_stake <= maximum_stake && maximum_stake > 0, error::invalid_argument(EINVALID_STAKE_RANGE));
    }

    fun validate_required_lockup(min_lockup_duration_secs: u64, max_lockup_duration_secs: u64) {
        assert!(
            min_lockup_duration_secs <= max_lockup_duration_secs && max_lockup_duration_secs > 0,
            error::invalid_argument(EINVALID_LOCKUP_RANGE),
        );
    }

    #[test_only]
    use aptos_framework::aptos_coin;

    #[test_only]
    use aptos_framework::governance_proposal;

    #[test_only]
    const CONSENSUS_KEY_1: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP_1: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    const CONSENSUS_KEY_2: vector<u8> = x"a344eb437bcd8096384206e1be9c80be3893fd7fdf867acce5a048e5b1546028bdac4caf419413fd16d4d6a609e0b0a3";
    #[test_only]
    const CONSENSUS_POP_2: vector<u8> = x"909d3a378ad5c17faf89f7a2062888100027eda18215c7735f917a4843cd41328b42fa4242e36dedb04432af14608973150acbff0c5d3f325ba04b287be9747398769a91d4244689cfa9c535a5a4d67073ee22090d5ab0a88ab8d2ff680e991e";

    #[test_only]
    const MAXIMUM_LOCK_UP_SECS: u64 = 1000;

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator = @0x123)]
    public entry fun test_inactive_validator_can_add_stake_join_and_unlock_stake(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let validator_address = signer::address_of(&validator);
        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Join the validator set with enough stake.
        join_validator_set(&validator, validator_address);
        end_epoch();
        assert!(is_current_epoch_validator(validator_address), 1);

        // Validator adds more stake (validator is already active).
        add_stake(&validator, 100);
        assert!(coin::balance<AptosCoin>(validator_address) == 800, 3);
        assert_validator_state(validator_address, 100, 0, 100, 0, 0);

        // pending_active and pending_inactive stakes are processed in the new epoch.
        // Rewards were also distributed.
        end_epoch();
        assert_validator_state(validator_address, 201, 0, 0, 0, 0);

        // Unlock after lockup expires. Timestamp is in microseconds.
        timestamp::update_global_time_for_test(MAXIMUM_LOCK_UP_SECS * 1000000);
        unlock(&validator, 100);
        assert_validator_state(validator_address, 101, 0, 0, 100, 0);
        end_epoch();

        // Validator withdraws from inactive stake, including rewards on the withdrawn amount.
        withdraw(&validator);
        assert!(coin::balance<AptosCoin>(validator_address) == 900, 4);
        assert_validator_state(validator_address, 101, 0, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, core_resources = @core_resources, validator = @0x123)]
    public entry fun test_pending_active_validator_can_add_stake(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let validator_address = signer::address_of(&validator);
        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Join the validator set and add more stake while pending_active.
        join_validator_set(&validator, validator_address);
        add_stake(&validator, 100);
        assert!(!is_current_epoch_validator(validator_address), 0);
        assert_validator_state(validator_address, 200, 0, 0, 0, 0);

        // Validator is added to the set with all added stake in the next epoch.
        on_new_epoch();
        assert!(is_current_epoch_validator(validator_address), 1);
        assert_validator_state(validator_address, 200, 0, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, core_resources = @core_resources, validator = @0x123)]
    public entry fun test_voting_power_and_rewards_are_bounded_by_max_stake(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);

        // Set the rewards rate to be very high so the validator's stake exceeds the max allowed after rewards.
        initialize_validator_set(&aptos_framework, 0, 100, 0, MAXIMUM_LOCK_UP_SECS, true, 100, 100);

        let validator_address = signer::address_of(&validator);
        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Join the validator set with max stake allowed.
        join_validator_set(&validator, validator_address);
        end_epoch();
        assert!(is_current_epoch_validator(validator_address), 1);

        // Rewards have been distributed, the validator's stake is now double the original amount.
        end_epoch();
        assert_validator_state(validator_address, 200, 0, 0, 0, 0);

        // Validator's voting power is still not more than the max allowed amount of 100.
        let validator_set = borrow_global<ValidatorSet>(@aptos_framework);
        let voting_power = vector::borrow(&validator_set.active_validators, 0).voting_power;
        assert!(voting_power == 100, voting_power);

        // Unlock the excess stake after lockup expires. Timestamp is in microseconds.
        timestamp::update_global_time_for_test(MAXIMUM_LOCK_UP_SECS * 1000000);
        unlock(&validator, 100);
        assert_validator_state(validator_address, 100, 0, 0, 100, 0);
        // Also increase lockup so we can still receive rewards.
        increase_lockup(&validator, timestamp::now_seconds() + MAXIMUM_LOCK_UP_SECS);
        end_epoch();

        // Validator should only receives 100 more reward coins for the active stake and enough for the pending inactive
        assert_validator_state(validator_address, 200, 100, 0, 0, 0);
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator = @0x123)]
    public entry fun test_delegated_staking_with_owner_cap(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);

        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        let stake = coin::mint<AptosCoin>(100, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);

        let pool_address = signer::address_of(&validator);
        register_validator_candidate(&validator, CONSENSUS_KEY_1, CONSENSUS_POP_1, vector::empty(), vector::empty());
        let owner_cap = extract_owner_cap(&validator);

        // Add stake when the validator is not yet activated.
        add_stake_with_cap(pool_address, &owner_cap, stake);
        increase_lockup_with_cap(pool_address, &owner_cap, MAXIMUM_LOCK_UP_SECS);
        assert_validator_state(pool_address, 100, 0, 0, 0, 0);

        // Join the validator set with enough stake.
        join_validator_set(&validator, pool_address);
        end_epoch();
        assert!(is_current_epoch_validator(pool_address), 0);

        // Unlock the entire stake after lockup expires.
        timestamp::update_global_time_for_test(MAXIMUM_LOCK_UP_SECS * 1000000);
        unlock_with_cap(pool_address, 100, &owner_cap);
        assert_validator_state(pool_address, 0, 0, 0, 100, 0);
        end_epoch();

        // Withdraw stake.
        let coins = withdraw_with_cap(pool_address, &owner_cap);
        // Extra rewards added.
        assert!(coin::value<AptosCoin>(&coins) == 100, 1);
        assert_validator_state(pool_address, 0, 0, 0, 0, 0);

        // Operator can separately rotate consensus key.
        rotate_consensus_key(&validator, pool_address, CONSENSUS_KEY_2, CONSENSUS_POP_2);
        let validator_config = borrow_global<ValidatorConfig>(pool_address);
        assert!(validator_config.consensus_pubkey == CONSENSUS_KEY_2, 2);

        // Operator can update network and fullnode addresses.
        update_network_and_fullnode_addresses(&validator, pool_address, b"1", b"2");
        let validator_config = borrow_global<ValidatorConfig>(pool_address);
        assert!(validator_config.network_addresses == b"1", 3);
        assert!(validator_config.fullnode_addresses == b"2", 4);

        let OwnerCapability { pool_address: _ } = owner_cap;
        coin::burn(coins, &burn_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator_1 = @0x123, validator_2 = @0x234, validator_3 = @0x345)]
    public entry fun test_multiple_validators_join_and_leave(
        aptos_framework: signer,
        core_resources: signer,
        validator_1: signer,
        validator_2: signer,
        validator_3: signer
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        let validator_1_address = signer::address_of(&validator_1);
        let validator_2_address = signer::address_of(&validator_2);
        let validator_3_address = signer::address_of(&validator_3);

        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator_1, &mint_cap);
        register_mint_stake(&validator_2, &mint_cap);
        register_mint_stake(&validator_3, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Validator 1 and 2 join the validator set.
        join_validator_set(&validator_2, validator_2_address);
        join_validator_set(&validator_1, validator_1_address);
        end_epoch();
        assert!(is_current_epoch_validator(validator_1_address), 0);
        assert!(is_current_epoch_validator(validator_2_address), 1);

        // Validator indices should be ordered by validator addresses. In this case, validator 1 has a smaller address.
        assert_validator_state(validator_1_address, 100, 0, 0, 0, 0);
        assert_validator_state(validator_2_address, 100, 0, 0, 0, 1);
        let validator_set = borrow_global<ValidatorSet>(@aptos_framework);
        let validator_config_1 = vector::borrow(&validator_set.active_validators, 0);
        assert!(validator_config_1.addr == validator_1_address, 2);
        assert!(validator_config_1.config.validator_index == 0, 3);
        let validator_config_2 = vector::borrow(&validator_set.active_validators, 1);
        assert!(validator_config_2.addr == validator_2_address, 4);
        assert!(validator_config_2.config.validator_index == 1, 5);

        // Validator 1 rotates consensus key. Validator 2 leaves. Validator 3 joins.
        rotate_consensus_key(&validator_1, validator_1_address, CONSENSUS_KEY_2, CONSENSUS_POP_2);
        leave_validator_set(&validator_2, validator_2_address);
        join_validator_set(&validator_3, validator_3_address);
        // Validator 2 is not effectively removed until next epoch.
        assert!(is_current_epoch_validator(validator_2_address), 2);
        assert!(vector::borrow(&borrow_global<ValidatorSet>(@aptos_framework).pending_inactive, 0).addr == validator_2_address, 0);
        // Validator 3 is not effectively added until next epoch.
        assert!(!is_current_epoch_validator(validator_3_address), 3);
        assert!(vector::borrow(&borrow_global<ValidatorSet>(@aptos_framework).pending_active, 0).addr == validator_3_address, 0);
        assert!(vector::borrow(&borrow_global<ValidatorSet>(@aptos_framework).active_validators, 0).config.consensus_pubkey == CONSENSUS_KEY_1, 0);

        // Changes applied after new epoch
        end_epoch();
        assert!(is_current_epoch_validator(validator_1_address), 5);
        assert_validator_state(validator_1_address, 101, 0, 0, 0, 0);
        assert!(!is_current_epoch_validator(validator_2_address), 4);
        // The validator index of validator 2 stays the same but this doesn't matter as the next time they rejoin the
        // validator set, their index will get set correctly.
        assert_validator_state(validator_2_address, 101, 0, 0, 0, 1);
        assert!(is_current_epoch_validator(validator_3_address), 5);
        //assert_validator_state(validator_3_address, 100, 0, 0, 0, 1);
        assert!(vector::borrow(&borrow_global<ValidatorSet>(@aptos_framework).active_validators, 0).config.consensus_pubkey == CONSENSUS_KEY_2, 0);

        // validators without enough stake will be removed
        timestamp::update_global_time_for_test(MAXIMUM_LOCK_UP_SECS * 1000000);
        unlock(&validator_1, 50);
        end_epoch();
        assert!(!is_current_epoch_validator(validator_1_address), 6);
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator = @0x123)]
    #[expected_failure(abort_code = 0x1000f)]
    public entry fun test_validator_cannot_join_post_genesis(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, ValidatorConfig, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        initialize_validator_set(
            &aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, false, 1, 100);

        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Joining the validator set should fail as post genesis validator set change is not allowed.
        join_validator_set(&validator, signer::address_of(&validator));
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator = @0x123)]
    #[expected_failure(abort_code = 0x1000f)]
    public entry fun test_validator_cannot_leave_post_genesis(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        initialize_validator_set(
            &aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, false, 1, 100);

        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Bypass the check to join. This is the same function called during Genesis.
        let validator_address = signer::address_of(&validator);
        join_validator_set_internal(&validator, validator_address);
        end_epoch();

        // Leaving the validator set should fail as post genesis validator set change is not allowed.
        leave_validator_set(&validator, validator_address);
    }

    #[test(
        aptos_framework = @0x1,
        core_resources = @core_resources,
        validator_1 = @0x1,
        validator_2 = @0x2,
        validator_3 = @0x3,
        validator_4 = @0x4,
        validator_5 = @0x5
    )]
    public entry fun test_validator_set_is_sorted_by_address(
        aptos_framework: signer,
        core_resources: signer,
        validator_1: signer,
        validator_2: signer,
        validator_3: signer,
        validator_4: signer,
        validator_5: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        let v1_addr = signer::address_of(&validator_1);
        let v2_addr = signer::address_of(&validator_2);
        let v3_addr = signer::address_of(&validator_3);
        let v4_addr = signer::address_of(&validator_4);
        let v5_addr = signer::address_of(&validator_5);

        timestamp::set_time_has_started_for_testing(&aptos_framework);
        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator_1, &mint_cap);
        register_mint_stake(&validator_2, &mint_cap);
        register_mint_stake(&validator_3, &mint_cap);
        register_mint_stake(&validator_4, &mint_cap);
        register_mint_stake(&validator_5, &mint_cap);

        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        join_validator_set(&validator_3, v3_addr);
        end_epoch();
        assert!(validator_index(v3_addr) == 0, 0);

        join_validator_set(&validator_4, v4_addr);
        end_epoch();
        assert!(validator_index(v3_addr) == 0, 1);
        assert!(validator_index(v4_addr) == 1, 2);

        join_validator_set(&validator_1, v1_addr);
        end_epoch();
        assert!(validator_index(v1_addr) == 0, 3);
        assert!(validator_index(v3_addr) == 1, 4);
        assert!(validator_index(v4_addr) == 2, 5);

        join_validator_set(&validator_2, v2_addr);
        end_epoch();
        assert!(validator_index(v1_addr) == 0, 6);
        assert!(validator_index(v2_addr) == 1, 7);
        assert!(validator_index(v3_addr) == 2, 8);
        assert!(validator_index(v4_addr) == 3, 9);

        join_validator_set(&validator_5, v5_addr);
        end_epoch();
        assert!(validator_index(v1_addr) == 0, 10);
        assert!(validator_index(v2_addr) == 1, 11);
        assert!(validator_index(v3_addr) == 2, 12);
        assert!(validator_index(v4_addr) == 3, 13);
        assert!(validator_index(v5_addr) == 4, 14);
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator_1 = @0x123, validator_2 = @0x234)]
    public entry fun test_validator_rewards_are_performance_based(
        aptos_framework: signer,
        core_resources: signer,
        validator_1: signer,
        validator_2: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);

        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let validator_1_address = signer::address_of(&validator_1);
        let validator_2_address = signer::address_of(&validator_2);
        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator_1, &mint_cap);
        register_mint_stake(&validator_2, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);

        // Both validators join the set.
        join_validator_set(&validator_1, validator_1_address);
        join_validator_set(&validator_2, validator_2_address);
        end_epoch();

        // Validator 2 missed votes.
        let missed_votes = vector::empty<u64>();
        let validator_2_index = borrow_global<ValidatorConfig>(validator_2_address).validator_index;
        vector::push_back(&mut missed_votes, validator_2_index);
        update_performance_statistics(missed_votes);
        end_epoch();

        // Validator 2 received no rewards. Validator 1 didn't miss votes so it still receives rewards.
        assert_validator_state(validator_1_address, 101, 0, 0, 0, 0);
        assert_validator_state(validator_2_address, 100, 0, 0, 0, 1);

        // Validator 2 decides to leave. Both validators missed votes.
        timestamp::update_global_time_for_test(MAXIMUM_LOCK_UP_SECS * 1000000);
        unlock(&validator_2, 100);
        leave_validator_set(&validator_2, validator_2_address);
        let missed_votes = vector::empty<u64>();
        let validator_1_index = borrow_global<ValidatorConfig>(validator_1_address).validator_index;
        vector::push_back(&mut missed_votes, validator_1_index);
        vector::push_back(&mut missed_votes, validator_2_index);
        update_performance_statistics(missed_votes);
        end_epoch();

        // Validator 1 and 2 received no additional rewards due to missing votes.
        assert_validator_state(validator_1_address, 101, 0, 0, 0, 0);
        assert_validator_state(validator_2_address, 0, 100, 0, 0, 1);
    }

    #[test(aptos_framework = @0x1, core_resources = @core_resources, validator = @0x123)]
    public entry fun test_update_performance_statistics_should_not_fail_due_to_out_of_bounds(
        aptos_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        timestamp::set_time_has_started_for_testing(&aptos_framework);

        initialize_validator_set(&aptos_framework, 100, 10000, 0, MAXIMUM_LOCK_UP_SECS, true, 1, 100);

        let validator_address = signer::address_of(&validator);
        let (mint_cap, burn_cap) = aptos_coin::initialize(&aptos_framework, &core_resources);
        register_mint_stake(&validator, &mint_cap);
        store_aptos_coin_mint_cap(&aptos_framework, mint_cap);
        coin::destroy_burn_cap<AptosCoin>(burn_cap);
        join_validator_set(&validator, validator_address);
        end_epoch();

        // Invalid validator index in the missed votes vector should not lead to abort.
        let missed_votes = vector::empty<u64>();
        let valid_validator_index = borrow_global<ValidatorConfig>(validator_address).validator_index;
        let out_of_bounds_index = valid_validator_index + 100;
        vector::push_back(&mut missed_votes, valid_validator_index);
        vector::push_back(&mut missed_votes, out_of_bounds_index);
        update_performance_statistics(missed_votes);
        end_epoch();

        // Validator received no rewards due to missing votes.
        assert_validator_state(validator_address, 100, 0, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    public entry fun test_change_validator_set_configs(aptos_framework: signer) acquires ValidatorSetConfiguration {
        use aptos_framework::governance_proposal;

        initialize_validator_set(&aptos_framework, 0, 1, 0, 1, false, 1, 1);

        update_required_stake(&governance_proposal::create_test_proposal(), 100, 1000);
        update_required_lockup(&governance_proposal::create_test_proposal(), 1000, 10000);
        update_rewards_rate(&governance_proposal::create_test_proposal(), 10, 100);
        let config = borrow_global<ValidatorSetConfiguration>(@aptos_framework);
        assert!(config.minimum_stake == 100, 0);
        assert!(config.maximum_stake == 1000, 1);
        assert!(config.min_lockup_duration_secs == 1000, 2);
        assert!(config.max_lockup_duration_secs == 10000, 3);
        assert!(config.rewards_rate == 10, 4);
        assert!(config.rewards_rate_denominator == 100, 4);
    }

    #[test]
    #[expected_failure(abort_code = 0x10011)]
    public entry fun test_update_required_stake_invalid_range_should_fail() acquires ValidatorSetConfiguration {
        update_required_stake(&governance_proposal::create_test_proposal(), 10, 5);
    }

    #[test]
    #[expected_failure(abort_code = 0x10011)]
    public entry fun test_update_required_stake_zero_max_stake_should_fail() acquires ValidatorSetConfiguration {
        update_required_stake(&governance_proposal::create_test_proposal(), 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x10012)]
    public entry fun test_update_required_lockup_invalid_range_should_fail() acquires ValidatorSetConfiguration {
        update_required_lockup(&governance_proposal::create_test_proposal(), 10, 5);
    }

    #[test]
    #[expected_failure(abort_code = 0x10012)]
    public entry fun test_update_required_lockup_zero_max_lockup_should_fail() acquires ValidatorSetConfiguration {
        update_required_lockup(&governance_proposal::create_test_proposal(), 0, 0);
    }

    #[test_only]
    public entry fun register_mint_stake(
        account: &signer,
        mint_cap: &MintCapability<AptosCoin>,
    ) acquires OwnerCapability, StakePool, StakePoolEvents, ValidatorConfig, ValidatorSet, ValidatorSetConfiguration {
        coin::register<AptosCoin>(account);
        let address = signer::address_of(account);
        coin::deposit<AptosCoin>(address, coin::mint<AptosCoin>(1000, mint_cap));
        register_validator_candidate(account, CONSENSUS_KEY_1, CONSENSUS_POP_1, vector::empty(), vector::empty());
        add_stake(account, 100);
        increase_lockup(account, timestamp::now_seconds() + MAXIMUM_LOCK_UP_SECS);
        assert_validator_state(signer::address_of(account), 100, 0, 0, 0, 0);
    }

    #[test_only]
    fun end_epoch() acquires StakePool, StakePoolEvents, AptosCoinCapabilities, ValidatorConfig, ValidatorPerformance, ValidatorSet, ValidatorSetConfiguration {
        // Set the number of blocks to 1 so reward distribution doesn't error out with division by zero.
        borrow_global_mut<ValidatorPerformance>(@aptos_framework).num_blocks = 1;
        on_new_epoch();
    }

    #[test_only]
    fun validator_index(addr: address): u64 acquires ValidatorSet {
        let validator_set = borrow_global<ValidatorSet>(@aptos_framework);
        option::extract(&mut find_validator(&validator_set.active_validators, addr))
    }

    #[test_only]
    fun assert_validator_state(
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_active_stake: u64,
        pending_inactive_stake: u64,
        validator_index: u64,
    ) acquires StakePool, ValidatorConfig {
        let stake_pool = borrow_global<StakePool>(pool_address);
        let actual_active_stake = coin::value<AptosCoin>(&stake_pool.active);
        assert!(actual_active_stake == active_stake, actual_active_stake);
        let actual_inactive_stake = coin::value<AptosCoin>(&stake_pool.inactive);
        assert!(actual_inactive_stake == inactive_stake, actual_inactive_stake);
        let actual_pending_active_stake = coin::value<AptosCoin>(&stake_pool.pending_active);
        assert!(actual_pending_active_stake == pending_active_stake, actual_pending_active_stake);
        let actual_pending_inactive_stake = coin::value<AptosCoin>(&stake_pool.pending_inactive);
        assert!(actual_pending_inactive_stake == pending_inactive_stake, actual_pending_inactive_stake);
        let validator_config = borrow_global<ValidatorConfig>(pool_address);
        assert!(validator_config.validator_index == validator_index, validator_config.validator_index);
    }

    #[test_only]
    public fun create_validator_set(
        aptos_framework: &signer,
        active_validator_addresses: vector<address>
    ) {
        let active_validators = vector::empty<ValidatorInfo>();
        let i = 0;
        while (i < vector::length(&active_validator_addresses)) {
            let validator_address = vector::borrow(&active_validator_addresses, i);
            vector::push_back(&mut active_validators, ValidatorInfo {
                addr: *validator_address,
                voting_power: 0,
                config: ValidatorConfig {
                    consensus_pubkey: CONSENSUS_KEY_1,
                    network_addresses: b"",
                    fullnode_addresses: b"",
                    validator_index: 0,
                }
            });
            i = i + 1;
        };

        move_to(aptos_framework, ValidatorSet {
            consensus_scheme: 0,
            // active validators for the current epoch
            active_validators,
            // pending validators to leave in next epoch (still active)
            pending_inactive: vector::empty<ValidatorInfo>(),
            // pending validators to join in next epoch
            pending_active: vector::empty<ValidatorInfo>(),
        });
    }

    #[test_only]
    public fun create_stake_pool(
        account: &signer,
        active: Coin<AptosCoin>,
        locked_until_secs: u64,
    ) {
        let address = signer::address_of(account);
        move_to(account, StakePool {
            active,
            inactive: coin::zero<AptosCoin>(),
            pending_active: coin::zero<AptosCoin>(),
            pending_inactive: coin::zero<AptosCoin>(),
            locked_until_secs,
            operator_address: address,
            delegated_voter: address,
        });
        move_to(account, OwnerCapability {
            pool_address: address,
        });
    }
}
