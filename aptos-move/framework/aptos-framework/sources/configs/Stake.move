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
module AptosFramework::Stake {
    use Std::Errors;
    use Std::Option::{Self, Option};
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Coin::{Self, Coin, MintCapability};
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
    use AptosFramework::TestCoin::TestCoin;

    friend AptosFramework::Reconfiguration;
    friend AptosFramework::Genesis;

    /// TODO: Parameterize these.
    const REWARDS_RATE_PERCENT: u64 = 1;
    const MINIMUM_LOCK_PERIOD: u64 = 86400;

    /// Lock period is shorter than required.
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
    const EALREADY_VALIDATOR: u64 = 6;
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
        active: Coin<TestCoin>,
        // inactive stake, can be withdrawn
        inactive: Coin<TestCoin>,
        // pending activation for next epoch
        pending_active: Coin<TestCoin>,
        // pending deactivation for next epoch
        pending_inactive: Coin<TestCoin>,
        locked_until_secs: u64,
        // Track the current operator of the validator node.
        // This allows the operator to be different from the original account and allow for separation of
        // the validator operations and ownership.
        operator_address: address,
    }

    /// Validator info stored in validator address.
    struct ValidatorConfig has key, copy, store, drop {
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
        // to make it compatible with previous definition, remove later
        fullnode_address: vector<u8>,
    }

    /// Consensus information per validator, stored in ValidatorSet.
    struct ValidatorInfo has copy, store, drop {
        addr: address,
        voting_power: u64,
        config: ValidatorConfig,
    }

    /// Full ValidatorSet, stored in @CoreResource.
    /// 1. join_validator_set adds to pending_active queue.
    /// 2. leave_valdiator_set moves from active to pending_inactive queue.
    /// 3. on_new_epoch processes two pending queues and refresh ValidatorInfo from the owner's address.
    struct ValidatorSet has key {
        consensus_scheme: u8,
        // minimum stakes required to join validator set
        minimum_stake: u64,
        // maximum stakes allowed to join validator set
        maximum_stake: u64,
        // active validators for the current epoch
        active_validators: vector<ValidatorInfo>,
        // pending validators to leave in next epoch (still active)
        pending_inactive: vector<ValidatorInfo>,
        // pending validators to join in next epoch
        pending_active: vector<ValidatorInfo>,
    }

    /// TestCoin capabilities, set during genesis and stored in @CoreResource account.
    /// This allows the Stake module to mint rewards to stakers.
    struct TestCoinCapabilities has key {
        mint_cap: MintCapability<TestCoin>,
    }

    /// Initialize validator set to the core resource account.
    public fun initialize_validator_set(core_resources: &signer, minimum_stake: u64, maximum_stake: u64) {
        SystemAddresses::assert_core_resource(core_resources);
        move_to(core_resources, ValidatorSet {
            consensus_scheme: 0,
            minimum_stake,
            maximum_stake,
            active_validators: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
        });
    }

    /// This is only called during Genesis, which is where MintCapability<TestCoin> can be created.
    /// Beyond genesis, no one can create TestCoin mint/burn capabilities.
    public fun store_test_coin_mint_cap(core_resources: &signer, mint_cap: MintCapability<TestCoin>) {
        SystemAddresses::assert_core_resource(core_resources);
        move_to(core_resources, TestCoinCapabilities { mint_cap })
    }

    /// Initialize the validator account and give ownership to the signing account.
    public(script) fun register_validator_candidate(
        account: &signer,
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
        fullnode_address: vector<u8>,
    ) {
        let account_address = Signer::address_of(account);
        assert!(!exists<StakePool>(account_address), Errors::invalid_argument(EALREADY_REGISTERED));

        move_to(account, StakePool {
            active: Coin::zero<TestCoin>(),
            pending_active: Coin::zero<TestCoin>(),
            pending_inactive: Coin::zero<TestCoin>(),
            inactive: Coin::zero<TestCoin>(),
            locked_until_secs: 0,
            operator_address: account_address,
        });
        move_to(account, ValidatorConfig {
            consensus_pubkey,
            network_address,
            fullnode_address,
        });
        move_to(account, OwnerCapability {
           pool_address: account_address,
        });
    }

    /// Extract and return owner capability from the signing account.
    public fun extract_owner_cap(account: &signer): OwnerCapability acquires OwnerCapability {
        move_from<OwnerCapability>(Signer::address_of(account))
    }

    /// Deposit `owner_cap` into `account`. This requires `account` to not already have owernship of another
    /// staking pool.
    public fun deposit_owner_cap(account: &signer, owner_cap: OwnerCapability) {
        move_to(account, owner_cap);
    }

    /// Allows an owner to change the operator of the stake pool.
    public(script) fun set_operator(account: &signer, new_operator: address) acquires OwnerCapability, StakePool {
        let account_addr = Signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        set_operator_with_cap(account_addr, ownership_cap, new_operator);
    }

    /// Allows an account with ownership capability to change the operator of the stake pool.
    public fun set_operator_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        new_operator: address,
    ) acquires StakePool {
        assert!(owner_cap.pool_address == pool_address, Errors::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        stake_pool.operator_address = new_operator;
    }

    /// Add `amount` of coins from the `account` owning the StakePool.
    public(script) fun add_stake(
        account: &signer,
        amount: u64,
    ) acquires OwnerCapability, StakePool, ValidatorSet {
        let account_addr = Signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        add_stake_with_cap(account_addr, ownership_cap, Coin::withdraw<TestCoin>(account, amount));
    }

    /// Add `coins` into `pool_address`. this requires the corresponding `owner_cap` to be passed in.
    public fun add_stake_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        coins: Coin<TestCoin>,
    ) acquires StakePool, ValidatorSet {
        assert!(owner_cap.pool_address == pool_address, Errors::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        // Add to pending_active if it's a current validator because the stake is not counted until the next epoch.
        // Otherwise, the delegation can be added to active directly as the validator is also activated in the epoch.
        if (is_current_validator(pool_address)) {
            Coin::merge<TestCoin>(&mut stake_pool.pending_active, coins);
        } else {
            Coin::merge<TestCoin>(&mut stake_pool.active, coins);
        };

        let maximum_stake = borrow_global<ValidatorSet>(@CoreResources).maximum_stake;
        let total_stake = Coin::value<TestCoin>(&stake_pool.active) +
                          Coin::value<TestCoin>(&stake_pool.pending_active);
        assert!(total_stake <= maximum_stake, Errors::invalid_argument(ESTAKE_EXCEEDS_MAX));
    }

    /// Rotate the consensus key of the validator, it'll take effect in next epoch.
    public(script) fun rotate_consensus_key(
        account: &signer,
        pool_address: address,
        consensus_pubkey: vector<u8>,
    ) acquires StakePool, ValidatorConfig {
        let stake_pool = borrow_global<StakePool>(pool_address);
        assert!(Signer::address_of(account) == stake_pool.operator_address, Errors::invalid_argument(ENOT_OPERATOR));

        assert!(exists<ValidatorConfig>(pool_address), Errors::not_published(EVALIDATOR_CONFIG));
        let validator_info = borrow_global_mut<ValidatorConfig>(pool_address);
        validator_info.consensus_pubkey = consensus_pubkey;
    }

    /// Similar to increase_lockup_with_cap but will use ownership capability from the signing account.
    public(script) fun increase_lockup(
        account: &signer,
        new_locked_until_secs: u64,
    ) acquires OwnerCapability, StakePool {
        let account_addr = Signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        increase_lockup_with_cap(account_addr, ownership_cap, new_locked_until_secs);
    }

    /// Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly inactive if it's not from an active validator.
    public fun increase_lockup_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
        new_locked_until_secs: u64,
    ) acquires StakePool {
        assert!(owner_cap.pool_address == pool_address, Errors::invalid_argument(ENOT_OWNER));
        validate_lockup_time(new_locked_until_secs);

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        assert!(new_locked_until_secs > stake_pool.locked_until_secs, Errors::invalid_argument(ELOCK_TIME_TOO_SHORT));
        stake_pool.locked_until_secs = new_locked_until_secs;
    }

    /// Request to have `pool_address` join the validator set. Can only be called after calling `register_validator_candidate`.
    /// If the validator has the required stake (more than minimum and less than maximum allowed), they will be
    /// added to the pending_active queue. All validators in this queue will be added to the active set when the next
    /// epoch starts (eligibility will be rechecked).
    ///
    /// This can only called by the operator of the validator/staking pool.
    public(friend) fun join_validator_set(
        account: &signer,
        pool_address: address,
    ) acquires StakePool, ValidatorConfig, ValidatorSet {
        let stake_pool = borrow_global<StakePool>(pool_address);
        // Account has to be the operator.
        assert!(Signer::address_of(account) == stake_pool.operator_address, Errors::invalid_argument(ENOT_OPERATOR));

        // Requires the validator to have a lockup more than the minimum.
        validate_lockup_time(stake_pool.locked_until_secs);

        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        let exist =  Option::is_some(&find_validator(&validator_set.active_validators, pool_address)) ||
                     Option::is_some(&find_validator(&validator_set.pending_inactive, pool_address)) ||
                     Option::is_some(&find_validator(&validator_set.pending_active, pool_address));
        assert!(!exist, Errors::invalid_argument(EALREADY_VALIDATOR));

        // The validator is not yet activated so all added stake should be in active.
        let voting_power = Coin::value<TestCoin>(&stake_pool.active);
        assert!(voting_power >= validator_set.minimum_stake, Errors::invalid_argument(ESTAKE_TOO_LOW));
        assert!(voting_power <= validator_set.maximum_stake, Errors::invalid_argument(ESTAKE_TOO_HIGH));

        // Add validator to pending_active, to be activated in the next epoch.
        Vector::push_back(&mut validator_set.pending_active, generate_validator_info(pool_address));
    }

    /// Similar to unlock_with_cap but will use ownership capability from the signing account.
    public(script) fun unlock(account: &signer, amount: u64) acquires OwnerCapability, StakePool, ValidatorSet {
        let account_addr = Signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        unlock_with_cap(account_addr, amount, ownership_cap);
    }

    /// Unlock `amount` from the active stake. Only possible if the lockup has expired.
    public fun unlock_with_cap(
        pool_address: address,
        amount: u64,
        owner_cap: &OwnerCapability,
    ) acquires StakePool, ValidatorSet {
        assert!(owner_cap.pool_address == pool_address, Errors::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let unlocked_stake = Coin::extract<TestCoin>(&mut stake_pool.active, amount);

        // Withdrawal is not allowed if lockup has not expired.
        // This is also enforced even if the validator is not active in order to prevent a validator from intentionally
        // leave the set early to be able to unlock.
        let current_time = Timestamp::now_seconds();
        if (stake_pool.locked_until_secs > current_time) {
            abort Errors::invalid_argument(EWITHDRAW_NOT_ALLOWED)
        };

        // Move requested amount to pending_inactive if the validator is already active.
        // Otherwise, we can move directly to inactive.
        if (is_current_validator(pool_address)) {
            Coin::merge<TestCoin>(&mut stake_pool.pending_inactive, unlocked_stake);
        } else {
            Coin::merge<TestCoin>(&mut stake_pool.inactive, unlocked_stake);
        };
    }

    /// Withdraw from `account`'s inactive stake.
    public(script) fun withdraw(account: &signer) acquires OwnerCapability, StakePool {
        let account_addr = Signer::address_of(account);
        let ownership_cap = borrow_global<OwnerCapability>(account_addr);
        let coins = withdraw_with_cap(account_addr, ownership_cap);
        Coin::deposit<TestCoin>(account_addr, coins);
    }

    /// Withdraw from `pool_address`'s inactive stake with the corresponding `owner_cap`.
    public fun withdraw_with_cap(
        pool_address: address,
        owner_cap: &OwnerCapability,
    ): Coin<TestCoin> acquires StakePool {
        assert!(owner_cap.pool_address == pool_address, Errors::invalid_argument(ENOT_OWNER));

        let stake_pool = borrow_global_mut<StakePool>(pool_address);
        let withdraw_amount = Coin::value<TestCoin>(&stake_pool.inactive);
        assert!(withdraw_amount > 0, Errors::invalid_argument(ENO_COINS_TO_WITHDRAW));

        Coin::extract<TestCoin>(&mut stake_pool.inactive, withdraw_amount)
    }

    /// Request to have `pool_address` leave the validator set. The validator is only actually removed from the set when
    /// the next epoch starts.
    /// The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
    /// is still operational.
    ///
    /// Can only be called by the operator of the validator/staking pool.
    public(friend) fun leave_validator_set(
        account: &signer,
        pool_address: address,
    ) acquires StakePool, ValidatorSet {
        let stake_pool = borrow_global<StakePool>(pool_address);
        // Account has to be the operator.
        assert!(Signer::address_of(account) == stake_pool.operator_address, Errors::invalid_argument(ENOT_OPERATOR));

        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        let maybe_index = find_validator(&validator_set.active_validators, pool_address);
        assert!(Option::is_some(&maybe_index), Errors::invalid_argument(ENOT_VALIDATOR));
        let index = Option::extract(&mut maybe_index);

        let validator_info = Vector::swap_remove(&mut validator_set.active_validators, index);
        assert!(Vector::length(&validator_set.active_validators) > 0, Errors::invalid_argument(ELAST_VALIDATOR));
        Vector::push_back(&mut validator_set.pending_inactive, validator_info);
    }

    /// Returns if the current validator is still active (can still vote) in the current epoch.
    /// This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
    /// when the epoch starts.
    public fun is_current_validator(addr: address): bool acquires ValidatorSet{
        let validator_set = borrow_global<ValidatorSet>(@CoreResources);
        Option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
        Option::is_some(&find_validator(&validator_set.pending_inactive, addr))
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
    public(friend) fun on_new_epoch() acquires StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        // Process pending stake and distribute rewards for each currently active validator.
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.active_validators, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };

        // Process pending stake and distribute rewards for each currently pending_inactive validator (requested to
        // leave but not removed yet).
        let i = 0;
        let len = Vector::length(&validator_set.pending_inactive);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.pending_inactive, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };

        // Activate currently pending_active validators.
        append(&mut validator_set.active_validators, &mut validator_set.pending_active);

        // Officially deactivate all pending_inactive validators. They will now no longer receive rewards.
        validator_set.pending_inactive = Vector::empty();

        // Update validator info (so network address/public key change takes effect)
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        let active_validators = Vector::empty();
        while (i < len) {
            let old_validator_info = Vector::borrow_mut(&mut validator_set.active_validators, i);
            let new_validator_info = generate_validator_info(old_validator_info.addr);
            if (new_validator_info.voting_power >= validator_set.minimum_stake &&
                new_validator_info.voting_power <= validator_set.maximum_stake
            ) {
                Vector::push_back(&mut active_validators, new_validator_info);
            };
            i = i + 1;
        };
        validator_set.active_validators = active_validators;
    }

    /// Update individual validator's stake pool
    /// 1. distribute rewards to active/pending_inactive delegations
    /// 2. process pending_active, pending_inactive correspondingly
    /// This function shouldn't abort.
    fun update_stake_pool(addr: address) acquires StakePool, TestCoinCapabilities {
        let stake_pool = borrow_global_mut<StakePool>(addr);
        distribute_reward( &mut stake_pool.active);
        distribute_reward( &mut stake_pool.pending_inactive);

        // move pending_active to active
        Coin::merge<TestCoin>(&mut stake_pool.active, Coin::extract_all<TestCoin>(&mut stake_pool.pending_active));
        Coin::merge<TestCoin>(&mut stake_pool.inactive, Coin::extract_all<TestCoin>(&mut stake_pool.pending_inactive));
    }

    /// Distribute reward corresponding to `stake`. Rewards is currently simple and just a fixed % of staked coins.
    fun distribute_reward(stake: &mut Coin<TestCoin>) acquires TestCoinCapabilities {
        let rewards_amount = Coin::value<TestCoin>(stake) * REWARDS_RATE_PERCENT / 100;
        if (rewards_amount > 0) {
            let mint_cap = &borrow_global<TestCoinCapabilities>(@CoreResources).mint_cap;
            let rewards = Coin::mint<TestCoin>(rewards_amount, mint_cap);
            Coin::merge<TestCoin>(stake, rewards);
        }
    }

    fun append<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!Vector::is_empty(v2)) {
            Vector::push_back(v1, Vector::pop_back(v2));
        }
    }

    fun find_validator(v: &vector<ValidatorInfo>, addr: address): Option<u64> {
        let i = 0;
        let len = Vector::length(v);
        while (i < len) {
            if (Vector::borrow(v, i).addr == addr) {
                return Option::some(i)
            };
            i = i + 1;
        };
        Option::none()
    }

    fun generate_validator_info(addr: address): ValidatorInfo acquires StakePool, ValidatorConfig {
        let config = *borrow_global<ValidatorConfig>(addr);
        let stake_pool = borrow_global<StakePool>(addr);
        // As pending_active validators have all been activated. The new epoch's voting power is just the pool's
        // total active stake.
        let voting_power = Coin::value<TestCoin>(&stake_pool.active);
        ValidatorInfo {
            addr,
            voting_power,
            config,
        }
    }

    /// Validate that the lockup time is at least more than the minimum required.
    fun validate_lockup_time(locked_until_secs: u64) {
        let current_time = Timestamp::now_seconds();
        assert!(current_time + MINIMUM_LOCK_PERIOD < locked_until_secs, Errors::invalid_argument(ELOCK_TIME_TOO_SHORT));
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, validator = @0x123)]
    public(script) fun test_basic_staking(
        core_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        use AptosFramework::TestCoin;

        Timestamp::set_time_has_started_for_testing(&core_resources);

        initialize_validator_set(&core_resources, 100, 10000);

        let validator_address = Signer::address_of(&validator);
        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);
        Coin::register<TestCoin>(&validator);
        Coin::deposit<TestCoin>(validator_address, Coin::mint<TestCoin>(1000, &mint_cap));
        store_test_coin_mint_cap(&core_resources, mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);

        register_validator_candidate(&validator, Vector::empty(), Vector::empty(), Vector::empty());

        // Add stake when the validator is not yet activated.
        add_stake(&validator, 100);
        increase_lockup(&validator, 100000);
        assert!(Coin::balance<TestCoin>(validator_address) == 900, 0);
        assert_stake_pool(validator_address, 100, 0, 0, 0);

        // Join the validator set with enough stake.
        join_validator_set(&validator, validator_address);
        on_new_epoch();
        assert!(is_current_validator(validator_address), 1);

        // Validator adds more stake (validator is already active).
        add_stake(&validator, 100);
        assert!(Coin::balance<TestCoin>(validator_address) == 800, 3);
        assert_stake_pool(validator_address, 100, 0, 100, 0);

        // pending_active and pending_inactive stakes are processed in the new epoch.
        // Rewards were also distributed.
        on_new_epoch();
        assert_stake_pool(validator_address, 201, 0, 0, 0);

        // Unlock the entire stake after lockup expires.
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&validator, 100);
        assert_stake_pool(validator_address, 101, 0, 0, 100);
        on_new_epoch();

        // Validator withdraws from inactive stake, including rewards on the withdrawn amount.
        withdraw(&validator);
        assert!(Coin::balance<TestCoin>(validator_address) == 901, 4);
        assert_stake_pool(validator_address, 102, 0, 0, 0);
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, validator = @0x123)]
    public(script) fun test_delegated_staking_with_owner_cap(
        core_framework: signer,
        core_resources: signer,
        validator: signer,
    ) acquires OwnerCapability, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        use AptosFramework::TestCoin;

        Timestamp::set_time_has_started_for_testing(&core_resources);

        initialize_validator_set(&core_resources, 100, 10000);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);
        let stake = Coin::mint<TestCoin>(100, &mint_cap);
        store_test_coin_mint_cap(&core_resources, mint_cap);

        register_validator_candidate(&validator, Vector::empty(), Vector::empty(), Vector::empty());
        let owner_cap = extract_owner_cap(&validator);

        // Add stake when the validator is not yet activated.
        let pool_address = Signer::address_of(&validator);
        add_stake_with_cap(pool_address, &owner_cap, stake);
        increase_lockup_with_cap(pool_address, &owner_cap, 100000);
        assert_stake_pool(pool_address, 100, 0, 0, 0);

        // Join the validator set with enough stake.
        join_validator_set(&validator, pool_address);
        on_new_epoch();
        assert!(is_current_validator(pool_address), 0);

        // Unlock the entire stake after lockup expires.
        Timestamp::update_global_time_for_test(100001000000);
        unlock_with_cap(pool_address, 100, &owner_cap);
        assert_stake_pool(pool_address, 0, 0, 0, 100);
        on_new_epoch();

        // Withdraw stake.
        let coins = withdraw_with_cap(pool_address, &owner_cap);
        // Extra rewards added.
        assert!(Coin::value<TestCoin>(&coins) == 101, 1);
        assert_stake_pool(pool_address, 0, 0, 0, 0);

        // Operator can separately rotate consensus key.
        rotate_consensus_key(&validator, pool_address, x"1234");

        let OwnerCapability { pool_address: _ } = owner_cap;
        Coin::burn(coins, &burn_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, validator_1 = @0x123, validator_2 = @0x234, validator_3 = @0x345)]
    public(script) fun test_validator_join_leave(
        core_framework: signer,
        core_resources: signer,
        validator_1: signer,
        validator_2: signer,
        validator_3: signer
    ) acquires OwnerCapability, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        use AptosFramework::TestCoin;

        Timestamp::set_time_has_started_for_testing(&core_resources);
        let validator_1_address = Signer::address_of(&validator_1);
        let validator_2_address = Signer::address_of(&validator_2);
        let validator_3_address = Signer::address_of(&validator_3);

        initialize_validator_set(&core_resources, 100, 10000);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);
        Coin::register<TestCoin>(&validator_1);
        Coin::register<TestCoin>(&validator_2);
        Coin::register<TestCoin>(&validator_3);
        Coin::deposit<TestCoin>(validator_1_address, Coin::mint<TestCoin>(1000, &mint_cap));
        Coin::deposit<TestCoin>(validator_2_address, Coin::mint<TestCoin>(1000, &mint_cap));
        Coin::deposit<TestCoin>(validator_3_address, Coin::mint<TestCoin>(1000, &mint_cap));
        store_test_coin_mint_cap(&core_resources, mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);

        // Register and add stake for all 3 validators.
        register_validator_candidate(&validator_1, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&validator_2, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&validator_3, Vector::empty(), Vector::empty(), Vector::empty());
        add_stake(&validator_1, 100);
        increase_lockup(&validator_1, 100000);
        add_stake(&validator_2, 100);
        increase_lockup(&validator_2, 100000);
        add_stake(&validator_3, 100);
        increase_lockup(&validator_3, 100000);
        assert_stake_pool(validator_1_address, 100, 0, 0, 0);
        assert_stake_pool(validator_2_address, 100, 0, 0, 0);
        assert_stake_pool(validator_3_address, 100, 0, 0, 0);

        // Validator 1 and 2 join the validator set.
        join_validator_set(&validator_1, validator_1_address);
        join_validator_set(&validator_2, validator_2_address);
        on_new_epoch();
        assert!(is_current_validator(validator_1_address), 0);
        assert!(is_current_validator(validator_2_address), 1);

        // Validator 1 rotates consensus key. Validator 2 leaves. Validator 3 joins.
        rotate_consensus_key(&validator_1, validator_1_address, x"1234");
        leave_validator_set(&validator_2, validator_2_address);
        join_validator_set(&validator_3, validator_3_address);
        // Validator 2 is not effectively removed until next epoch.
        assert!(is_current_validator(validator_2_address), 2);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_inactive, 0).addr == validator_2_address, 0);
        assert!(!is_current_validator(validator_3_address), 3);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == validator_3_address, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == Vector::empty(), 0);

        // Changes applied after new epoch
        on_new_epoch();
        assert!(!is_current_validator(validator_2_address), 4);
        assert!(is_current_validator(validator_3_address), 5);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == x"1234", 0);

        // validators without enough stake will be removed
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&validator_1, 50);
        on_new_epoch();
        assert!(!is_current_validator(validator_1_address), 6);
    }

    #[test_only]
    fun assert_stake_pool(
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_active_stake: u64,
        pending_inactive_stake: u64,
    ) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(pool_address);
        assert!(Coin::value<TestCoin>(&stake_pool.active) == active_stake, 0);
        assert!(Coin::value<TestCoin>(&stake_pool.inactive) == inactive_stake, 1);
        assert!(Coin::value<TestCoin>(&stake_pool.pending_active) == pending_active_stake, 2);
        assert!(Coin::value<TestCoin>(&stake_pool.pending_inactive) == pending_inactive_stake, 3);
    }
}
