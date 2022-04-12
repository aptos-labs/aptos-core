module AptosFramework::Stake {
    use Std::Vector;
    use Std::Signer;
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
//    use AptosFramework::Reconfiguration;
    use AptosFramework::TestCoin::{Self, Coin};

    const MINIMUM_LOCK_PERIOD: u64 = 86400;
    const MINIMUM_RECONFIG_PERIOD: u64 = 3600;

    /// Basic unit of stake delegation, it's stored in StakePool.
    struct Delegation has store {
        coin: Coin,
        from: address,
        locked_until_secs: u64,
    }

    /// Aggregation of delegation and represent a validator's voting power, stored in ValidatorInfo.
    /// Invariants:
    /// 1. current_stake = sum(active + pending_inactive)
    /// 2. user interact with pending_active and inactive if it's in the ValidatorSet.
    /// 3. user interact with active, inactive if it's not in the ValidatorSet.
    /// 4. pending_active and pending_inactive are empty if it's not in the ValidatorSet.
    struct StakePool has store {
        // sum of active and pending_inactive stakes, updated on epoch boundary.
        current_stake: u64,
        // active stake
        active: vector<Delegation>,
        // inactive stake, can be withdrawn
        inactive: vector<Delegation>,
        // pending activation for next epoch
        pending_active: vector<Delegation>,
        // pending deactivation for next epoch
        pending_inactive: vector<Delegation>,
    }

    /// Consensus information per validator, stored in validator address.
    struct ValidatorInfo has key {
        stake_pool: StakePool,
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
    }

    /// Full ValidatorSet, stored in @CoreResource.
    struct ValidatorSet has key {
        consensus_scheme: u8,
        // minimum stakes requires to join validator set
        minimum_stake: u64,
        // maximum stakes allowed to join validator set
        maximum_stake: u64,
        // should we store the ValidatorInfo in place?
        validators: vector<address>,
        last_update_time_secs: u64,
    }

    /// Any user can delegate a stake.
    fun delegate_stake(account: &signer, to: address, coin: Coin, locked_until_secs: u64) acquires ValidatorInfo, ValidatorSet {
        let current_time = Timestamp::now_seconds();
        assert!(current_time + MINIMUM_LOCK_PERIOD < locked_until_secs, 0);
        let stake_pool = &mut borrow_global_mut<ValidatorInfo>(to).stake_pool;
        let delegation = Delegation {
            coin,
            locked_until_secs,
            from: Signer::address_of(account),
        };
        // add to pending_active if it's a current validator otherwise add to active directly
        if (is_validator(to)) {
            Vector::push_back(&mut stake_pool.pending_active, delegation);
        } else {
            stake_pool.current_stake = stake_pool.current_stake + TestCoin::value(&delegation.coin);
            Vector::push_back(&mut stake_pool.active, delegation);
        }
    }

    /// Withdraw from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly deposit if it's not from an active validator.
    fun withdraw_active(account: &signer, from: address) acquires ValidatorInfo, ValidatorSet {
        let addr = Signer::address_of(account);
        let current_time = Timestamp::now_seconds();
        let stake_pool = &mut borrow_global_mut<ValidatorInfo>(from).stake_pool;
        let d = withdraw_internal(&mut stake_pool.inactive, addr);
        let is_validator = is_validator(from);
        if (!is_validator) {
            // directly deposit if it's not active validator
            let Delegation {coin, from: _, locked_until_secs: _} = d;
            TestCoin::deposit(addr, coin);
        } else if (d.locked_until_secs < current_time) {
            // move to pending_inactive if it can be unlocked
            Vector::push_back(&mut stake_pool.pending_inactive, d);
        } else {
            // not allowed to withdraw
            abort 0
        };
    }

    /// Withdraw from inactive delegation, directly deposited to the account's balance.
    fun withdraw_inactive(account: &signer, from: address) acquires  ValidatorInfo {
        let addr = Signer::address_of(account);
        let stake_pool = &mut borrow_global_mut<ValidatorInfo>(from).stake_pool;
        let d = withdraw_internal(&mut stake_pool.inactive, addr);
        let Delegation {coin, from: _, locked_until_secs: _} = d;
        TestCoin::deposit(addr, coin);
    }

    /// Initialize the ValidatorInfo for account.
    fun register_validator_candidate(account: &signer, consensus_pubkey: vector<u8>, network_address: vector<u8>) {
        let stake_pool = StakePool {
            current_stake: 0,
            active: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
            inactive: Vector::empty(),
        };
        move_to(account, ValidatorInfo {
            stake_pool,
            consensus_pubkey,
            network_address,
        });
    }

    /// Rotate the consensus key of the validator.
    fun rotate_consensus_key(account: &signer, consensus_pubkey: vector<u8>) acquires ValidatorInfo, ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        borrow_global_mut<ValidatorInfo>(addr).consensus_pubkey = consensus_pubkey;
        if (Vector::contains(&validator_set.validators, &addr)) {
            on_new_epoch(validator_set);
        };
    }

    /// Initialize validator set to the core resource account.
    fun initialize_validator_set(account: &signer, minimum_stake: u64, maximum_stake: u64) {
        SystemAddresses::assert_core_resource(account);
        move_to(account, ValidatorSet {
            consensus_scheme: 0,
            minimum_stake,
            maximum_stake,
            validators: Vector::empty(),
            last_update_time_secs: Timestamp::now_seconds(),
        });
    }

    /// Initiate by the validator info owner
    fun join_validator_set(account: &signer) acquires ValidatorInfo, ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_info = borrow_global_mut<ValidatorInfo>(addr);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        assert!(!Vector::contains(&validator_set.validators, &addr), 0);

        assert!(validator_info.stake_pool.current_stake >= validator_set.minimum_stake, 0);
        assert!(validator_info.stake_pool.current_stake <= validator_set.maximum_stake, 0);

        on_new_epoch(validator_set);
        Vector::push_back(&mut validator_set.validators, addr);
    }

    /// Initiate by the validator info owner.
    fun leave_validator_set(account: &signer) acquires ValidatorInfo, ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);

        let (exist, index) = Vector::index_of(&validator_set.validators, &addr);
        assert!(exist, 0);
        on_new_epoch(validator_set);
        Vector::swap_remove(&mut validator_set.validators, index);
        assert!(Vector::length(&validator_set.validators) > 0, 0);
    }

    /// Triggers when validator set changes or after certain time period (in block prologue).
    fun on_new_epoch(validator_set: &mut ValidatorSet) acquires ValidatorInfo {
        let current_time = Timestamp::now_seconds();
        assert!(validator_set.last_update_time_secs + MINIMUM_RECONFIG_PERIOD < current_time, 0);
        validator_set.last_update_time_secs = current_time;
        let i = 0;
        let len = Vector::length(&validator_set.validators);
        while (i < len) {
            let addr = *Vector::borrow(&validator_set.validators, i);
            update_stake_pool(addr);
            i = i + 1;
        };
        // Reconfiguration::reconfigure();
        // do we remove validators without enough stakes?
    }

    /// Update individual validator's stake pool
    /// 1. distribute rewards to active/pending_inactive delegations
    /// 2. process pending_active, pending_inactive correspondingly
    /// 3. update the current stake
    fun update_stake_pool(addr: address) acquires ValidatorInfo {
        let stake_pool = &mut borrow_global_mut<ValidatorInfo>(addr).stake_pool;
        distribute_reward( &mut stake_pool.active);
        distribute_reward( &mut stake_pool.pending_inactive);
        // move pending_active to active
        append(&mut stake_pool.active, &mut stake_pool.pending_active);
        // move pending_inactive to inactive
        append(&mut stake_pool.inactive, &mut stake_pool.pending_inactive);
        let current_stake = 0;
        let i = 0;
        let len = Vector::length(&stake_pool.active);
        while (i < len) {
            current_stake = current_stake + TestCoin::value(&Vector::borrow(&stake_pool.active, i).coin);
            i = i + 1;
        };
        stake_pool.current_stake = current_stake;
    }

    /// Mint the reward and add to the delegation based on some formula
    fun distribute_reward(v: &mut vector<Delegation>) {
        let i = 0;
        let len = Vector::length(v);
        while (i < len) {
            let d = Vector::borrow_mut(v, i);
            let reward = TestCoin::zero(); // mint some coins based on delegation, timestamp, maybe also totaly stakes
            TestCoin::merge(&mut d.coin, reward);
            i = i + 1;
        };
    }

    fun append<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!Vector::is_empty(v2)) {
            Vector::push_back(v1, Vector::pop_back(v2));
        }
    }

    fun find(v: &vector<Delegation>, addr: address): u64 {
        let i = 0;
        let len =  Vector::length(v);
        while (i < len) {
            let d = Vector::borrow(v, i);
            if (d.from == addr) {
                return i
            };
            i = i + 1;
        };
        abort 0
    }

    fun is_validator(addr: address): bool acquires ValidatorSet {
        let validator_set =borrow_global<ValidatorSet>(addr);
        Vector::contains(&validator_set.validators, &addr)
    }

    fun withdraw_internal(v: &mut vector<Delegation>, addr: address): Delegation {
        let index = find(v, addr);
        Vector::swap_remove(v, index)
    }

}
