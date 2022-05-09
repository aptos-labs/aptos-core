module AptosFramework::Stake {
    use Std::Errors;
    use Std::Option::{Self, Option};
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::IterableTable::{Self, IterableTable};
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
    use AptosFramework::TestCoin::{Self, Coin};

    friend AptosFramework::Reconfiguration;
    friend AptosFramework::Genesis;

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
    /// Account is not a validator.
    const ENOT_VALIDATOR: u64 = 8;
    /// Can't remove last validator.
    const ELAST_VALIDATOR: u64 = 9;
    /// Delegation from the address already exists in this pool
    const EDELEGATION_ALREADY_EXIST: u64 = 10;
    /// Delegation to this stake pool will make it exceed tht maximum stake specified in ValidatorSet.
    const EDELEGATION_EXCEED_MAX: u64 = 11;

    /// Basic unit of stake delegation, it's stored in StakePool.
    struct Delegation has store {
        coins: Coin,
        rewards: Coin,
        locked_until_secs: u64,
    }

    /// Aggregation of delegation and represent a validator's voting power, stored in ValidatorInfo.
    /// Invariants:
    /// 1. voting_power = sum(active + pending_inactive)
    /// 2. user interact with pending_active and inactive if it's in the ValidatorSet.
    /// 3. user interact with active, inactive if it's not in the ValidatorSet.
    /// 4. pending_active and pending_inactive are empty if it's not in the ValidatorSet.
    struct StakePool has key, store {
        // sum of active and pending_inactive stakes, updated on epoch boundary for active validators.
        voting_power: u64,
        // sum of active and pending_active stakes, updated when delegate/unlock happens.
        next_epoch_voting_power: u64,
        // active stake
        active: IterableTable<address, Delegation>,
        // inactive stake, can be withdrawn
        inactive: IterableTable<address,Delegation>,
        // pending activation for next epoch
        pending_active: IterableTable<address, Delegation>,
        // pending deactivation for next epoch
        pending_inactive: IterableTable<address, Delegation>,
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

    /// Any user can delegate a stake.
    public(friend) fun delegate_stake(account: &signer, to: address, amount: u64, locked_until_secs: u64) acquires StakePool, ValidatorSet {
        let coins = TestCoin::withdraw(account, amount);
        let current_time = Timestamp::now_seconds();
        assert!(current_time + MINIMUM_LOCK_PERIOD < locked_until_secs, Errors::invalid_argument(ELOCK_TIME_TOO_SHORT));
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global_mut<StakePool>(to);
        assert!(!find_delegation_from_pool(stake_pool, addr), Errors::invalid_argument(EDELEGATION_ALREADY_EXIST));
        let delegation = Delegation {
            coins,
            rewards: TestCoin::zero(),
            locked_until_secs,
        };
        // add to pending_active if it's a current validator otherwise add to active directly
        stake_pool.next_epoch_voting_power = stake_pool.next_epoch_voting_power + TestCoin::value(&delegation.coins);
        let maximum_stake = borrow_global<ValidatorSet>(@CoreResources).maximum_stake;
        assert!(stake_pool.next_epoch_voting_power <= maximum_stake, Errors::invalid_argument(EDELEGATION_EXCEED_MAX));
        if (!is_current_validator(to)) {
            IterableTable::add(&mut stake_pool.active, &addr, delegation);
        } else {
            IterableTable::add(&mut stake_pool.pending_active, &addr, delegation);
        }
    }

    /// Unlock from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly inactive if it's not from an active validator.
    public(script) fun unlock(account: &signer, from: address) acquires StakePool, ValidatorSet {
        let addr = Signer::address_of(account);
        let current_time = Timestamp::now_seconds();
        let stake_pool = borrow_global_mut<StakePool>(from);
        let delegation = withdraw_internal(&mut stake_pool.active, addr);
        let is_current_validator = is_current_validator(from);
        stake_pool.next_epoch_voting_power = stake_pool.next_epoch_voting_power - TestCoin::value(&delegation.coins);
        if (!is_current_validator) {
            // move to inactive directly if it's not from an active validator
            IterableTable::add(&mut stake_pool.inactive, &addr, delegation);
        } else if (delegation.locked_until_secs < current_time) {
            // move to pending_inactive if it can be unlocked
            IterableTable::add(&mut stake_pool.pending_inactive, &addr,  delegation);
        } else {
            abort Errors::invalid_argument(EWITHDRAW_NOT_ALLOWED)
        };
    }

    /// Withdraw from inactive delegation.
    public(script) fun withdraw(account: &signer, from: address): Coin acquires StakePool {
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global_mut<StakePool>(from);
        let delegation = withdraw_internal(&mut stake_pool.inactive, addr);
        let Delegation {coins, rewards, locked_until_secs: _} = delegation;
        TestCoin::merge(&mut coins, rewards);
        coins
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
            next_epoch_voting_power: 0,
            active: IterableTable::new(),
            pending_active: IterableTable::new(),
            pending_inactive: IterableTable::new(),
            inactive: IterableTable::new(),
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

    /// Initialize validator set to the core resource account.
    public fun initialize_validator_set(account: &signer, minimum_stake: u64, maximum_stake: u64) {
        SystemAddresses::assert_core_resource(account);
        move_to(account, ValidatorSet {
            consensus_scheme: 0,
            minimum_stake,
            maximum_stake,
            active_validators: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
        });
    }

    /// Initiate by the validator info owner
    public(friend) fun join_validator_set(account: &signer) acquires StakePool, ValidatorConfig, ValidatorSet {
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global<StakePool>(addr);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        assert!(stake_pool.next_epoch_voting_power >= validator_set.minimum_stake, Errors::invalid_argument(ESTAKE_TOO_LOW));
        assert!(stake_pool.next_epoch_voting_power <= validator_set.maximum_stake, Errors::invalid_argument(ESTAKE_TOO_HIGH));
        let exist =  Option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
                     Option::is_some(&find_validator(&validator_set.pending_inactive, addr)) ||
                     Option::is_some(&find_validator(&validator_set.pending_active, addr));
        assert!(!exist, Errors::invalid_argument(EALREADY_VALIDATOR));

        Vector::push_back(&mut validator_set.pending_active, generate_validator_info(addr));
    }

    /// Initiate by the validator info owner.
    public(friend) fun leave_validator_set(account: &signer) acquires ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);

        let maybe_index = find_validator(&validator_set.active_validators, addr);
        assert!(Option::is_some(&maybe_index), Errors::invalid_argument(ENOT_VALIDATOR));
        let index = Option::extract(&mut maybe_index);

        let validator_info = Vector::swap_remove(&mut validator_set.active_validators, index);
        assert!(Vector::length(&validator_set.active_validators) > 0, Errors::invalid_argument(ELAST_VALIDATOR));
        Vector::push_back(&mut validator_set.pending_inactive, validator_info);
    }

    public fun is_current_validator(addr: address): bool acquires ValidatorSet{
        let validator_set = borrow_global<ValidatorSet>(@CoreResources);
        Option::is_some(&find_validator(&validator_set.active_validators, addr)) ||
        Option::is_some(&find_validator(&validator_set.pending_inactive, addr))
    }

    /// Triggers at epoch boundary.
    /// 1. distribute rewards to stake pool of active and pending inactive validators
    /// 2. purge pending queues
    /// 3. update the validator info from owners' address
    /// This function shouldn't abort.
    public(friend) fun on_new_epoch() acquires StakePool, ValidatorConfig, ValidatorSet {
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        // distribute reward
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.active_validators, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };
        let i = 0;
        let len = Vector::length(&validator_set.pending_inactive);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.pending_inactive, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };
        // purge pending queue
        append(&mut validator_set.active_validators, &mut validator_set.pending_active);
        validator_set.pending_inactive = Vector::empty();
        // update validator info (so network address/public key change takes effect)
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        let active_validators = Vector::empty();
        while (i < len) {
            let old_validator_info = Vector::borrow_mut(&mut validator_set.active_validators, i);
            let stake_pool = borrow_global_mut<StakePool>(old_validator_info.addr);
            stake_pool.voting_power = stake_pool.next_epoch_voting_power;
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
    fun update_stake_pool(addr: address) acquires StakePool {
        let stake_pool = borrow_global_mut<StakePool>(addr);
        distribute_reward( &mut stake_pool.active);
        distribute_reward( &mut stake_pool.pending_inactive);
        // move pending_active to active
        IterableTable::append(&mut stake_pool.active, &mut stake_pool.pending_active);
        // move pending_inactive to inactive
        IterableTable::append(&mut stake_pool.inactive, &mut stake_pool.pending_inactive);
    }

    /// Mint the reward and add to the delegation based on some formula
    fun distribute_reward(v: &mut IterableTable<address, Delegation>) {
        let key = IterableTable::head_key(v);
        while (Option::is_some(&key)) {
            let (delegation, _, next) = IterableTable::borrow_iter_mut(v, Option::borrow(&key));
            let reward = TestCoin::zero(); // mint some coins based on delegation, timestamp, maybe also total stakes
            TestCoin::merge(&mut delegation.rewards, reward);
            key = next;
        };
    }

    fun append<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!Vector::is_empty(v2)) {
            Vector::push_back(v1, Vector::pop_back(v2));
        }
    }

    fun find_delegation_from_pool(pool: &StakePool, addr: address): bool {
        find_delegation(&pool.active, addr) ||
        find_delegation(&pool.pending_active, addr) ||
        find_delegation(&pool.pending_inactive, addr) ||
        find_delegation(&pool.inactive, addr)
    }

    fun find_delegation(v: &IterableTable<address, Delegation>, addr: address): bool {
        IterableTable::contains(v, &addr)
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

    fun withdraw_internal(v: &mut IterableTable<address, Delegation>, addr: address): Delegation {
        assert!(find_delegation(v, addr), Errors::invalid_argument(EDELEGATION_NOT_FOUND));
        IterableTable::remove(v, &addr)
    }

    fun generate_validator_info(addr: address): ValidatorInfo acquires StakePool, ValidatorConfig {
        let config = *borrow_global<ValidatorConfig>(addr);
        let voting_power = borrow_global<StakePool>(addr).voting_power;
        ValidatorInfo {
            addr,
            voting_power,
            config,
        }
    }

    #[test(core_resources = @CoreResources, account_1 = @0x123, account_2 = @0x234, account_3 = @0x345)]
    public(script) fun test_basic_delegation(
        core_resources: signer,
        account_1: signer,
        account_2: signer,
        account_3: signer,
    ) acquires StakePool, ValidatorConfig, ValidatorSet {
        initialize_validator_set(&core_resources, 100, 10000);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        TestCoin::mint_for_test(&account_1, 10000);
        TestCoin::mint_for_test(&account_2, 10000);
        TestCoin::mint_for_test(&account_3, 10000);
        register_validator_candidate(&account_1, Vector::empty(), Vector::empty(), Vector::empty());
        let addr1 = Signer::address_of(&account_1);
        let addr2 = Signer::address_of(&account_2);
        let addr3 = Signer::address_of(&account_3);
        // delegation when the address is not a validator
        delegate_stake(&account_1, addr1, 100, 100000);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).active, &addr1).coins) == 100, 0);
        delegate_stake(&account_2, addr1, 101, 100000);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).active, &addr2).coins) == 101, 0);
        // join the validator set with enough stake
        join_validator_set(&account_1);
        on_new_epoch();
        // delegation when the address is active valdiator
        assert!(is_current_validator(addr1), 0);
        delegate_stake(&account_3, addr1, 102, 100000);
        assert!(borrow_global<StakePool>(addr1).voting_power == 201, 0);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).active, &addr1).coins) == 100, 0);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).active, &addr2).coins) == 101, 0);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).pending_active, &addr3).coins) == 102, 0);
        // unlock active stakes
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&account_1, addr1);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).pending_inactive, &addr1).coins) == 100, 0);
        // total stake doesn't change until next epoch
        assert!(borrow_global<StakePool>(addr1).voting_power == 201, 0);
        // pending delegations are processed on new epoch
        on_new_epoch();
        assert!(IterableTable::length(&borrow_global<StakePool>(addr1).pending_active) == 0, 0);
        assert!(IterableTable::length(&borrow_global<StakePool>(addr1).pending_inactive) == 0, 0);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).inactive, &addr1).coins) == 100, 0);
        assert!(TestCoin::value(&IterableTable::borrow(&borrow_global<StakePool>(addr1).active, &addr3).coins) == 102, 0);
        assert!(borrow_global<StakePool>(addr1).voting_power == 203, 0);
        // withdraw
        let coins = withdraw(&account_1, addr1);
        assert!(TestCoin::value(&coins) == 100, 0);
        TestCoin::deposit(addr1, coins);
    }

    #[test(core_resources = @CoreResources, account_1 = @0x123, account_2 = @0x234, account_3 = @0x345)]
    public(script) fun test_validator_join_leave(
        core_resources: signer,
        account_1: signer,
        account_2: signer,
        account_3: signer
    ) acquires StakePool, ValidatorConfig, ValidatorSet {
        initialize_validator_set(&core_resources, 100, 10000);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        TestCoin::mint_for_test(&account_1, 10000);
        let addr1 = Signer::address_of(&account_1);
        let addr2 = Signer::address_of(&account_2);
        let addr3 = Signer::address_of(&account_3);
        register_validator_candidate(&account_1, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&account_2, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&account_3, Vector::empty(), Vector::empty(), Vector::empty());
        delegate_stake(&account_1, addr1, 100, 100000);
        delegate_stake(&account_1, addr2, 100, 100000);
        delegate_stake(&account_1, addr3, 100, 100000);
        join_validator_set(&account_1);
        join_validator_set(&account_2);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == addr1, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 1).addr == addr2, 0);
        on_new_epoch();
        assert!(is_current_validator(addr1), 0);
        assert!(is_current_validator(addr2), 0);
        // changes don't take effect until next epoch
        leave_validator_set(&account_2);
        join_validator_set(&account_3);
        rotate_consensus_key(&account_1, x"1234");
        assert!(is_current_validator(addr2), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_inactive, 0).addr == addr2, 0);
        assert!(!is_current_validator(addr3), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == addr3, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == Vector::empty(), 0);
        // changes applied after new epoch
        on_new_epoch();
        assert!(!is_current_validator(addr2), 0);
        assert!(is_current_validator(addr3), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == x"1234", 0);
        // validators without enough stake will be removed
        Timestamp::update_global_time_for_test(100001000000);
        unlock(&account_1, addr1);
        on_new_epoch();
        assert!(!is_current_validator(addr1), 0);
    }
}
