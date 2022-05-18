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

    /// Delegation not found for the account.
    const EDELEGATION_NOT_FOUND: u64 = 1;
    /// Lock period is shorter than required.
    const ELOCK_TIME_TOO_SHORT: u64 = 2;
    /// Withdraw not allowed, the stake is still locked.
    const EWITHDRAW_NOT_ALLOWED: u64 = 3;
    /// Validator Config not published.
    const EVALIDATOR_CONFIG: u64 = 4;
    /// Not enough stake to join validator set.
    const ESTAKE_TOO_LOW: u64 = 5;
    /// Too much stake to join validator set.
    const ESTAKE_TOO_HIGH: u64 = 6;
    /// Account is already a validator or pending validator.
    const EALREADY_VALIDATOR: u64 = 7;
    /// Account is not an active validator.
    const ENOT_ACTIVE_VALIDATOR: u64 = 8;
    /// Can't remove last validator.
    const ELAST_VALIDATOR: u64 = 9;
    /// Delegation from the address already exists in this pool
    const EDELEGATION_ALREADY_EXIST: u64 = 10;
    /// Delegation to this stake pool will make it exceed tht maximum stake specified in ValidatorSet.
    const EDELEGATION_EXCEED_MAX: u64 = 11;
    /// Account is not a validator.
    const ENOT_VALIDATOR: u64 = 12;
    /// Delegation found but for the wrong validator.
    const EDELEGATION_EXISTS_WITH_DIFFERENT_VALIDATOR: u64 = 13;
    /// Delegation is not active and cannot be unlocked.
    const EDELEGATION_NOT_ACTIVE: u64 = 14;
    /// Delegation is not yet marked as inactive and cannot be withdrawn.
    const EDELEGATION_NOT_INACTIVE: u64 = 15;
    /// Delegation amount is below the required minimum.
    const EDELEGATION_AMOUNT_BELOW_MINIMUM: u64 = 16;

    /// Delegation state enum values.
    const DELEGATION_STATE_ACTIVE: u64 = 1;
    const DELEGATION_STATE_INACTIVE: u64 = 2;
    const DELEGATION_STATE_PENDING_ACTIVE: u64 = 3;
    const DELEGATION_STATE_PENDING_INACTIVE: u64 = 4;

    /// Represent a delegation share in a staking pool. Shares represent % ownership of the pool, including rewards.
    struct DelegationShare has key {
        // pool_address and locked_until_secs should never change after a DelegationShare is created.
        pool_address: address,
        locked_until_secs: u64,

        // Shares and state can change after a DelegationShare is created.
        shares: u64,
        // Enum value.
        state: u64,
    }

    /// Represent a pool of coins. Shares are issued according to the current share price (total coins / total shares).
    struct CoinPool has store {
        coins: Coin<TestCoin>,
        total_shares: u64,
    }

    /// Wrapper around CoinPool to store the delegator addresses.
    struct PendingCoinPool has store {
        pool: CoinPool,
        delegator_addresses: vector<address>,
    }

    /// Each validator has an associated StakingPool and can receive stake delegations from other accounts to it.
    ///
    /// Similar to validator set, any changes to delegations or voting power for an active validator only take effect
    /// in the new epoch. If the stake pool belongs to a validator that's already removed from the validator set,
    /// changes will take effect immediately.
    ///
    /// Invariants:
    /// 1. voting_power = sum(active delegations + pending_inactive delegations)
    /// 2. user interacts with pending_active and inactive if it's in the ValidatorSet.
    /// 3. user interacts with active, inactive if it's not in the ValidatorSet.
    /// 4. pending_active and pending_inactive are empty if it's not in the ValidatorSet.
    struct StakePool has key {
        // Total stake in active delegations pool, only updated on epoch boundary for active validators.
        voting_power: u64,
        // Active pool of delegated coins. Only updated when new epoch starts if the validator is already active.
        // Otherwise, updated immediately when a delegation happens.
        // This pool does receive rewards and is counted in current voting power.
        active: CoinPool,
        // Inactive pool of delegated coins.
        // This pool does not receive rewards, is not counted in current or future voting power, and can be withdrawn.
        inactive: CoinPool,
        // Pending pool of delegated coins to be added to the active pool in the next epoch.
        // This pool does not receive rewards and is not counted in the current voting power.
        pending_active: PendingCoinPool,
        // Pending pool of delegated coins to be added to the inactive pool in the next epoch.
        // This pool still receives rewards and is counted in the current voting power.
        pending_inactive: PendingCoinPool,
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

    /// Full ValidatorSet, stored in @CoreResource account.
    /// All changes to the validator set only take effect in the next epoch.
    ///
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

    /// Delegation parameters, set during genesis and stored in @CoreResource account.
    struct DelegationParameters has key {
        minimum_delegation: u64,
    }

    /// TestCoin capabilities, set during genesis and stored in @CoreResource account.
    /// This allows the Stake module to mint rewards to stakers.
    struct TestCoinCapabilities has key {
        mint_cap: MintCapability<TestCoin>,
    }

    /// Any user can delegate a stake to a validator.
    ///
    /// Only one active delegation can be done per user at the moment. No parameters (validator address, amount, lockup)
    /// can be changed until `delegator` withdraws the delegation.
    ///
    /// `validator_address` delegated to does not have to be active yet. If they're already active, the delegation is
    /// not yet effective until the next epoch.
    public(friend) fun delegate_stake(
        delegator: &signer,
        validator_address: address,
        amount: u64,
        locked_until_secs: u64,
    ) acquires DelegationParameters, StakePool, ValidatorSet {
        let current_time = Timestamp::now_seconds();
        assert!(current_time + MINIMUM_LOCK_PERIOD < locked_until_secs, Errors::invalid_argument(ELOCK_TIME_TOO_SHORT));

        let minimum_delegation = borrow_global<DelegationParameters>(@CoreResources).minimum_delegation;
        assert!(amount >= minimum_delegation, Errors::invalid_argument(EDELEGATION_AMOUNT_BELOW_MINIMUM));

        let delegator_address = Signer::address_of(delegator);
        // We don't allow changing an existing delegation. Delegator needs to withdraw first.
        assert!(!exists<DelegationShare>(delegator_address), Errors::invalid_argument(EDELEGATION_ALREADY_EXIST));
        let delegation = DelegationShare {
            pool_address: validator_address,
            locked_until_secs,
            // These params will be set correctly later.
            shares: 0,
            state: DELEGATION_STATE_PENDING_ACTIVE,
        };

        assert!(exists<StakePool>(validator_address), Errors::invalid_argument(ENOT_VALIDATOR));
        let stake_pool = borrow_global_mut<StakePool>(validator_address);
        let coins_to_delegate = Coin::withdraw<TestCoin>(delegator, amount);
        // Add to pending_active if it's a current validator because the delegation is not counted until the next epoch.
        // Otherwise, the delegation can be added to active directly as the validator is also activated in the epoch.
        if (is_current_validator(validator_address)) {
            let pending_active_pool = &mut stake_pool.pending_active;
            delegation.shares = buy_shares_from_pool(&mut pending_active_pool.pool, coins_to_delegate);
            delegation.state = DELEGATION_STATE_PENDING_ACTIVE;
            Vector::push_back(&mut pending_active_pool.delegator_addresses, delegator_address);
        } else {
            delegation.shares = buy_shares_from_pool(&mut stake_pool.active, coins_to_delegate);
            delegation.state = DELEGATION_STATE_ACTIVE;
        };
        move_to(delegator, delegation);

        // We do not allow voting power to exceed the configured maximum stake.
        let maximum_stake = borrow_global<ValidatorSet>(@CoreResources).maximum_stake;
        let total_stake = Coin::value<TestCoin>(&stake_pool.active.coins) +
                          Coin::value<TestCoin>(&stake_pool.pending_active.pool.coins);
        assert!(total_stake <= maximum_stake, Errors::invalid_argument(EDELEGATION_EXCEED_MAX));
    }

    /// Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly inactive if it's not from an active validator.
    public(script) fun unlock(
        delegator: &signer,
        validator_address: address,
    ) acquires DelegationShare, StakePool, ValidatorSet {
        let delegator_address = Signer::address_of(delegator);
        let current_time = Timestamp::now_seconds();

        assert!(exists<StakePool>(validator_address), Errors::invalid_argument(ENOT_VALIDATOR));
        let stake_pool = borrow_global_mut<StakePool>(validator_address);

        assert!(exists<DelegationShare>(delegator_address), Errors::invalid_argument(EDELEGATION_NOT_FOUND));
        let delegation = borrow_global_mut<DelegationShare>(delegator_address);
        assert!(
            delegation.pool_address == validator_address,
            Errors::invalid_argument(EDELEGATION_EXISTS_WITH_DIFFERENT_VALIDATOR),
        );
        assert!(
            delegation.state == DELEGATION_STATE_ACTIVE,
            Errors::invalid_argument(EDELEGATION_NOT_ACTIVE),
        );

        let unlocked_coins = extract_from_pool(&mut stake_pool.active, delegation.shares);

        if (!is_current_validator(validator_address)) {
            delegation.state = DELEGATION_STATE_INACTIVE;
            // move to inactive directly if it's not from an active validator.
            delegation.shares = buy_shares_from_pool(&mut stake_pool.inactive, unlocked_coins);
        } else if (delegation.locked_until_secs < current_time) {
            delegation.state = DELEGATION_STATE_PENDING_INACTIVE;
            let pending_inactive_pool = &mut stake_pool.pending_inactive;
            // move to pending_inactive if it can be unlocked.
            delegation.shares = buy_shares_from_pool(&mut pending_inactive_pool.pool, unlocked_coins);
            Vector::push_back(&mut pending_inactive_pool.delegator_addresses, delegator_address);
        } else {
            abort Errors::invalid_argument(EWITHDRAW_NOT_ALLOWED)
        };
    }

    /// Withdraw from inactive pool.
    public(script) fun withdraw(
        delegator: &signer,
        validator_address: address,
    ): Coin<TestCoin> acquires DelegationShare, StakePool {
        assert!(exists<StakePool>(validator_address), Errors::invalid_argument(ENOT_VALIDATOR));
        let stake_pool = borrow_global_mut<StakePool>(validator_address);

        let delegator_address = Signer::address_of(delegator);
        assert!(exists<DelegationShare>(delegator_address), Errors::invalid_argument(EDELEGATION_NOT_FOUND));
        let DelegationShare {
            pool_address,
            shares,
            state: _,
            locked_until_secs: _,
        } = move_from<DelegationShare>(delegator_address);
        assert!(
            pool_address == validator_address,
            Errors::invalid_argument(EDELEGATION_EXISTS_WITH_DIFFERENT_VALIDATOR),
        );

        extract_from_pool(&mut stake_pool.inactive, shares)
    }

    /// Initialize the ValidatorInfo for account.
    public(script) fun register_validator_candidate(
        account: &signer,
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
        fullnode_address: vector<u8>
    ) {
        move_to(account, StakePool {
            voting_power: 0,
            active: new_coin_pool(),
            inactive: new_coin_pool(),
            pending_active: new_pending_coin_pool(),
            pending_inactive: new_pending_coin_pool(),
        });
        move_to(account, ValidatorConfig {
            consensus_pubkey,
            network_address,
            fullnode_address,
        });
    }

    /// Rotate the consensus key of the validator, it'll take effect in next epoch.
    public(script) fun rotate_consensus_key(account: &signer, consensus_pubkey: vector<u8>) acquires ValidatorConfig {
        let addr = Signer::address_of(account);
        assert!(exists<ValidatorConfig>(addr), Errors::not_published(EVALIDATOR_CONFIG));
        let validator_info = borrow_global_mut<ValidatorConfig>(addr);
        validator_info.consensus_pubkey = consensus_pubkey;
    }

    /// Initialize validator set and delegation parameters to the core resource account.
    public fun initialize_validator_set(
        account: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        minimum_delegation: u64,
    ) {
        SystemAddresses::assert_core_resource(account);
        move_to(account, ValidatorSet {
            consensus_scheme: 0,
            minimum_stake,
            maximum_stake,
            active_validators: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
        });
        move_to(account, DelegationParameters { minimum_delegation });
    }

    /// Request to join the validator set. Can only be called after calling `register_validator_candidate`.
    /// If the validator has the required stake (more than minimum and less than maximum allowed), they will be
    /// added to the pending_active queue. All validators in this queue will be added to the active set when the next
    /// epoch starts (eligibility will be rechecked).
    public(friend) fun join_validator_set(account: &signer) acquires StakePool, ValidatorConfig, ValidatorSet {
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global<StakePool>(addr);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        let total_stake = Coin::value<TestCoin>(&stake_pool.active.coins);
        assert!(total_stake >= validator_set.minimum_stake, Errors::invalid_argument(ESTAKE_TOO_LOW));
        assert!(total_stake <= validator_set.maximum_stake, Errors::invalid_argument(ESTAKE_TOO_HIGH));

        let validator_already_exists = Option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
                                       Option::is_some(&find_validator(&validator_set.pending_inactive, addr)) ||
                                       Option::is_some(&find_validator(&validator_set.pending_active, addr));
        assert!(!validator_already_exists, Errors::invalid_argument(EALREADY_VALIDATOR));

        Vector::push_back(&mut validator_set.pending_active, generate_validator_info(addr, stake_pool.voting_power));
    }

    /// Request to leave the validator set. The validator is only actually removed from the validator set when the next
    /// epoch starts.
    /// The last validator in the set cannot leave. This is an edge case that should never happen as long as the network
    /// is still operational.
    public(friend) fun leave_validator_set(account: &signer) acquires ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);

        let maybe_index = find_validator(&validator_set.active_validators, addr);
        assert!(Option::is_some(&maybe_index), Errors::invalid_argument(ENOT_ACTIVE_VALIDATOR));
        let index = Option::extract(&mut maybe_index);

        let validator_info = Vector::swap_remove(&mut validator_set.active_validators, index);
        assert!(Vector::length(&validator_set.active_validators) > 0, Errors::invalid_argument(ELAST_VALIDATOR));
        Vector::push_back(&mut validator_set.pending_inactive, validator_info);
    }

    /// Returns if the current validator is still active (can still vote) in the current epoch.
    /// This includes validators that requested to leave but are still in the pending_inactive queue and will be removed
    /// when the epoch starts.
    public fun is_current_validator(addr: address): bool acquires ValidatorSet {
        let validator_set = borrow_global<ValidatorSet>(@CoreResources);
        Option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
        Option::is_some(&find_validator(&validator_set.pending_inactive, addr))
    }

    /// Triggers at epoch boundary. This function shouldn't abort.
    ///
    /// 1. Distribute rewards to stake pools of active and pending inactive validators (requested to leave but not yet
    /// removed).
    /// 2. Officially move pending active delegators to the active set and move pending inactive delegators to the
    /// inactive delegators list so they withdraw their stake. The staking pool's voting power in this new epoch will
    /// be updated accordingly.
    /// 3. Add pending active validators to the active set if they satisfy requirements so they can vote and remove
    /// pending inactive validators so they no longer can vote.
    /// 4. The validator's voting power in the validator set is updated to be the corresponding staking pool's voting
    /// power.
    public(friend) fun on_new_epoch() acquires DelegationShare, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        // distribute reward
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        while (i < len) {
            let validator_address = Vector::borrow(&validator_set.active_validators, i).addr;
            update_stake_pool(validator_address);
            i = i + 1;
        };
        let i = 0;
        let len = Vector::length(&validator_set.pending_inactive);
        while (i < len) {
            let validator_address = Vector::borrow(&validator_set.pending_inactive, i).addr;
            update_stake_pool(validator_address);
            i = i + 1;
        };
        // purge pending queue
        append_vectors(&mut validator_set.active_validators, &mut validator_set.pending_active);
        validator_set.pending_inactive = Vector::empty();
        // update validator info (so network address/public key change takes effect)
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        let active_validators = Vector::empty();
        while (i < len) {
            let old_validator_info = Vector::borrow_mut(&mut validator_set.active_validators, i);
            let stake_pool = borrow_global_mut<StakePool>(old_validator_info.addr);
            // There should be no more pending delegations at this point so we can calculate the pool's voting power
            // as the total stake in the active delegations pool.
            let total_active_stake = Coin::value<TestCoin>(&stake_pool.active.coins);
            stake_pool.voting_power = total_active_stake;
            let new_validator_info = generate_validator_info(old_validator_info.addr, stake_pool.voting_power);
            if (new_validator_info.voting_power >= validator_set.minimum_stake &&
                new_validator_info.voting_power <= validator_set.maximum_stake
            ) {
                Vector::push_back(&mut active_validators, new_validator_info);
            };
            i = i + 1;
        };
        validator_set.active_validators = active_validators;
    }

    /// This is only called during Genesis, which is where MintCapability<TestCoin> can be created.
    /// Beyond genesis, no one can create TestCoin mint/burn capabilities.
    public fun store_test_coin_mint_cap(core_resources: &signer, mint_cap: MintCapability<TestCoin>) {
        move_to(core_resources, TestCoinCapabilities { mint_cap })
    }

    /// Update individual validator's stake pool
    /// 1. distribute rewards to active/pending_inactive delegations
    /// 2. process pending_active, pending_inactive delegations correspondingly
    /// This function shouldn't abort.
    fun update_stake_pool(validator_address: address) acquires DelegationShare, StakePool, TestCoinCapabilities {
        let stake_pool = borrow_global_mut<StakePool>(validator_address);
        let pending_inactive_pool = &mut stake_pool.pending_inactive;
        distribute_reward(&mut stake_pool.active);
        distribute_reward(&mut pending_inactive_pool.pool);

        // move pending_active to active
        process_pending_pool(&mut stake_pool.active, &mut stake_pool.pending_active, DELEGATION_STATE_ACTIVE);
        // move pending_inactive to inactive
        process_pending_pool(&mut stake_pool.inactive, pending_inactive_pool, DELEGATION_STATE_INACTIVE);
    }

    /// For each delegator, withdraw coins from `pending_pool` and deposit in `pool`.
    fun process_pending_pool(
        pool: &mut CoinPool,
        pending_pool: &mut PendingCoinPool,
        new_delegation_state: u64,
    ) acquires DelegationShare {
        let delegator_addresses = &mut pending_pool.delegator_addresses;
        let i = 0;
        let len = Vector::length(delegator_addresses);
        while (i < len) {
            let delegator_address = *Vector::borrow(delegator_addresses, i);
            let delegation = borrow_global_mut<DelegationShare>(delegator_address);
            let coins = extract_from_pool(&mut pending_pool.pool, delegation.shares);
            delegation.shares = buy_shares_from_pool(pool, coins);
            delegation.state = new_delegation_state;

            i = i + 1;
        };
        pending_pool.pool.total_shares = 0;
        pending_pool.delegator_addresses = Vector::empty();
    }

    /// Distribute reward to each delegation. Rewards is currently simple and just a fixed % of staked coins.
    fun distribute_reward(pool: &mut CoinPool) acquires TestCoinCapabilities {
        let rewards_amount = Coin::value<TestCoin>(&pool.coins) * REWARDS_RATE_PERCENT / 100;
        if (rewards_amount > 0) {
            let mint_cap = &borrow_global<TestCoinCapabilities>(@CoreResources).mint_cap;
            let rewards = Coin::mint<TestCoin>(rewards_amount, mint_cap);
            Coin::merge<TestCoin>(&mut pool.coins, rewards);
        }
    }

    /// Buy the shares in `pool` with `coins`.
    fun buy_shares_from_pool(pool: &mut CoinPool, coins: Coin<TestCoin>): u64 {
        let buy_in_amount = Coin::value<TestCoin>(&coins);
        let pool_balance = Coin::value<TestCoin>(&pool.coins);
        Coin::merge<TestCoin>(&mut pool.coins, coins);

        // If pool has no balance yet, issue as many shares as the buy in amount of coins.
        // This is effectively issuing shares at 1:1 value to coin.
        if (pool_balance == 0) {
            pool.total_shares = buy_in_amount;
            buy_in_amount
        } else {
            // Multiply first to avoid precision loss from integer division.
            // u64 can hold up to ~10**19 so this shouldn't overflow if maximum_stake is below 10**9 (1B).
            // total_shares and buy_in_amount are at most maximum_stakes, so total_shares * buy_in_amount <= 10**18.
            let shares_amount = buy_in_amount * pool.total_shares / pool_balance;
            pool.total_shares = pool.total_shares + shares_amount;
            shares_amount
        }
    }

    /// Redeem the shares in `delegation` for coins from `pool`.
    fun extract_from_pool(pool: &mut CoinPool, shares_to_redeem: u64): Coin<TestCoin> {
        let pool_balance = Coin::value<TestCoin>(&pool.coins);
        // Multiply first to avoid precision loss from integer division.
        // u64 can hold up to ~10**19 so this shouldn't overflow if maximum_stake is below 10**9 (1B).
        // pool_balance and shares_to_redeem are at most maximum_stakes, so pool_balance * shares_to_redeem <= 10**18.
        let redeem_amount = pool_balance * shares_to_redeem / pool.total_shares;
        pool.total_shares = pool.total_shares - shares_to_redeem;

        Coin::extract<TestCoin>(&mut pool.coins, redeem_amount)
    }

    // Return a new coin pool with zero balance and zero shares.
    fun new_coin_pool(): CoinPool {
        CoinPool {
            coins: Coin::zero<TestCoin>(),
            total_shares: 0,
        }
    }

    // Return a new pending coin pool.
    fun new_pending_coin_pool(): PendingCoinPool {
        PendingCoinPool {
            pool: new_coin_pool(),
            delegator_addresses: Vector::empty(),
        }
    }

    /// Append all elements in `v2` to 'v1'.
    fun append_vectors<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!Vector::is_empty(v2)) {
            Vector::push_back(v1, Vector::pop_back(v2));
        }
    }

    /// Find and return the validator corresponding to `validator_addr` in `v`.
    fun find_validator(v: &vector<ValidatorInfo>, validator_addr: address): Option<u64> {
        let i = 0;
        let len = Vector::length(v);
        while (i < len) {
            if (Vector::borrow(v, i).addr == validator_addr) {
                return Option::some(i)
            };
            i = i + 1;
        };
        Option::none()
    }

    /// Construct and return a new ValidatorInfo struct with `addr` and `voting_power`.
    fun generate_validator_info(addr: address, voting_power: u64): ValidatorInfo acquires ValidatorConfig {
        let config = *borrow_global<ValidatorConfig>(addr);
        ValidatorInfo {
            addr,
            voting_power,
            config,
        }
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, validator = @0x123, delegator_1 = @0x234, delegator_2 = @0x345)]
    public(script) fun test_basic_delegation(
        core_framework: signer,
        core_resources: signer,
        validator: signer,
        delegator_1: signer,
        delegator_2: signer,
    ) acquires DelegationShare, DelegationParameters, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        use Std::ASCII;

        let (mint_cap, burn_cap) = Coin::initialize<TestCoin>(
            &core_framework,
            ASCII::string(b"Test Coin"),
            ASCII::string(b"TC"),
            6, /* decimals */
            false, /* monitor_supply */
        );
        store_test_coin_mint_cap(&core_resources, copy mint_cap);

        let validator_address = Signer::address_of(&validator);
        let delegator_1_address = Signer::address_of(&delegator_1);
        let delegator_2_address = Signer::address_of(&delegator_2);
        Coin::register<TestCoin>(&validator);
        Coin::register<TestCoin>(&delegator_1);
        Coin::register<TestCoin>(&delegator_2);

        initialize_validator_set(&core_resources, 100, 10000, 10);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        let initial_balance = 10000;
        Coin::deposit<TestCoin>(validator_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));
        Coin::deposit<TestCoin>(delegator_1_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));
        Coin::deposit<TestCoin>(delegator_2_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));
        register_validator_candidate(&validator, Vector::empty(), Vector::empty(), Vector::empty());

        // Delegate when the validator has not yet joined the validator set.
        delegate_stake(&validator, validator_address, 100, 100000);
        assert!(Coin::balance<TestCoin>(validator_address) == initial_balance - 100, 0);
        assert_active_stake_pool(validator_address, 100, 100);
        assert_delegation(validator_address, validator_address, 100, DELEGATION_STATE_ACTIVE);
        delegate_stake(&delegator_1, validator_address, 101, 100000);
        assert!(Coin::balance<TestCoin>(delegator_1_address) == initial_balance - 101, 1);
        assert_active_stake_pool(validator_address, 201, 201);
        assert_delegation(delegator_1_address, validator_address, 101, DELEGATION_STATE_ACTIVE);

        // Join the validator set with enough stake.
        join_validator_set(&validator);
        on_new_epoch();

        // Delegate to an already an active validator.
        assert!(is_current_validator(validator_address), 2);
        delegate_stake(&delegator_2, validator_address, 102, 100000);
        // Current epoch's voting power has not changed but next epoch's voting power has.
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert!(stake_pool.voting_power == 201, 3);
        // Delegation has been added to the pending_active list since the validator is already active.
        assert!(Coin::balance<TestCoin>(delegator_2_address) == initial_balance - 102, 5);
        assert_pending_active_stake_pool(validator_address, 102, 102);
        assert_delegation(delegator_2_address, validator_address, 102, DELEGATION_STATE_PENDING_ACTIVE);

        // Unlock active stakes.
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&validator, validator_address);
        assert_pending_inactive_stake_pool(validator_address, 100, 100);
        assert_active_stake_pool(validator_address, 101, 101);
        assert_delegation(validator_address, validator_address, 100, DELEGATION_STATE_PENDING_INACTIVE);
        // Total stake doesn't change until next epoch.
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert!(stake_pool.voting_power == 201, 6);

        // All pending delegations are processed on new epoch.
        // Rewards are issued.
        on_new_epoch();
        assert_pending_active_stake_pool(validator_address, 0, 0);
        assert_pending_inactive_stake_pool(validator_address, 0, 0);
        // 101 origin coins from delegator_1 earning 1 coin as reward = 102 = active pool's balance at end of the epoch.
        // delegator_2 buys in with 102 coins => They get the same amount of shares as delegator_1 (101 shares).
        // => Active pool should now have 204 Coins and 202 shares (delegator_1 and delegator_2 each owns half).
        assert_active_stake_pool(validator_address, 204, 202);
        assert_inactive_stake_pool(validator_address, 101, 101);
        assert!(borrow_global<StakePool>(validator_address).voting_power == 204, 8);
        // Validator delegation gained 1 coin as reward (1% of 100 coins) and got moved from pending_inactive to
        // inactive. Since inactive was empty, validator delegation gains 101 shares (corresponding to 101 coins).
        assert_delegation(validator_address, validator_address, 101, DELEGATION_STATE_INACTIVE);
        assert_delegation(delegator_1_address, validator_address, 101, DELEGATION_STATE_ACTIVE);
        assert_delegation(delegator_2_address, validator_address, 101, DELEGATION_STATE_ACTIVE);

        // Withdraw from inactive delegations.
        let coins = withdraw(&validator, validator_address);
        assert_inactive_stake_pool(validator_address, 0, 0);
        Coin::deposit<TestCoin>(validator_address, coins);
        // Extra 1 Coin from staking rewards.
        assert!(Coin::balance<TestCoin>(validator_address) == initial_balance + 1, 9);
        assert!(!exists<DelegationShare>(validator_address), 10);

        Coin::destroy_mint_cap<TestCoin>(mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, validator_1 = @0x123, validator_2 = @0x234, validator_3 = @0x345)]
    public(script) fun test_validator_join_leave(
        core_framework: signer,
        core_resources: signer,
        validator_1: signer,
        validator_2: signer,
        validator_3: signer,
    ) acquires DelegationShare, DelegationParameters, StakePool, TestCoinCapabilities, ValidatorConfig, ValidatorSet {
        use Std::ASCII;

        let (mint_cap, burn_cap) = Coin::initialize<TestCoin>(
            &core_framework,
            ASCII::string(b"Test Coin"),
            ASCII::string(b"TC"),
            6, /* decimals */
            false, /* monitor_supply */
        );
        store_test_coin_mint_cap(&core_resources, copy mint_cap);
        let validator_1_address = Signer::address_of(&validator_1);
        let validator_2_address = Signer::address_of(&validator_2);
        let validator_3_address = Signer::address_of(&validator_3);
        Coin::register<TestCoin>(&validator_1);
        Coin::register<TestCoin>(&validator_2);
        Coin::register<TestCoin>(&validator_3);

        initialize_validator_set(&core_resources, 100, 10000, 10);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        let initial_balance = 10000;
        Coin::deposit<TestCoin>(validator_1_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));
        Coin::deposit<TestCoin>(validator_2_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));
        Coin::deposit<TestCoin>(validator_3_address, Coin::mint<TestCoin>(initial_balance, &mint_cap));

        register_validator_candidate(&validator_1, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&validator_2, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&validator_3, Vector::empty(), Vector::empty(), Vector::empty());
        delegate_stake(&validator_1, validator_1_address, 100, 100000);
        delegate_stake(&validator_2, validator_2_address, 100, 100000);
        delegate_stake(&validator_3, validator_3_address, 100, 100000);

        join_validator_set(&validator_1);
        join_validator_set(&validator_2);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == validator_1_address, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 1).addr == validator_2_address, 0);

        on_new_epoch();
        assert!(is_current_validator(validator_1_address), 0);
        assert!(is_current_validator(validator_2_address), 0);
        // Changes don't take effect until next epoch.
        leave_validator_set(&validator_2);
        join_validator_set(&validator_3);
        rotate_consensus_key(&validator_1, x"1234");
        assert!(is_current_validator(validator_2_address), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_inactive, 0).addr == validator_2_address, 0);
        assert!(!is_current_validator(validator_3_address), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == validator_3_address, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == Vector::empty(), 0);

        // Changes applied after new epoch.
        on_new_epoch();
        assert!(!is_current_validator(validator_2_address), 0);
        assert!(is_current_validator(validator_3_address), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == x"1234", 0);

        // Validators without enough stake will be removed.
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&validator_1, validator_1_address);
        on_new_epoch();
        assert!(!is_current_validator(validator_1_address), 0);

        Coin::destroy_mint_cap<TestCoin>(mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);
    }

    #[test_only]
    fun assert_active_stake_pool(
        validator_address: address,
        expected_coins: u64,
        expected_shares: u64,
    ) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert_pool(&stake_pool.active, expected_coins, expected_shares);
    }

    #[test_only]
    fun assert_inactive_stake_pool(
        validator_address: address,
        expected_coins: u64,
        expected_shares: u64,
    ) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert_pool(&stake_pool.inactive, expected_coins, expected_shares);
    }

    #[test_only]
    fun assert_pending_active_stake_pool(
        validator_address: address,
        expected_coins: u64,
        expected_shares: u64,
    ) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert_pool(&stake_pool.pending_active.pool, expected_coins, expected_shares);
    }

    #[test_only]
    fun assert_pending_inactive_stake_pool(
        validator_address: address,
        expected_coins: u64,
        expected_shares: u64,
    ) acquires StakePool {
        let stake_pool = borrow_global<StakePool>(validator_address);
        assert_pool(&stake_pool.pending_inactive.pool, expected_coins, expected_shares);
    }

    #[test_only]
    fun assert_pool(pool: &CoinPool, expected_coins: u64, expected_shares: u64) {
        assert!(Coin::value<TestCoin>(&pool.coins) == expected_coins, 0);
        assert!(pool.total_shares == expected_shares, 1);
    }

    #[test_only]
    fun assert_delegation(
        delegator_address: address,
        validator_address: address,
        expected_shares: u64,
        expected_state: u64,
    ) acquires DelegationShare {
        let delegation = borrow_global<DelegationShare>(delegator_address);
        assert!(delegation.pool_address == validator_address, 0);
        assert!(delegation.shares == expected_shares, 1);
        assert!(delegation.state == expected_state, 2);
    }
}
