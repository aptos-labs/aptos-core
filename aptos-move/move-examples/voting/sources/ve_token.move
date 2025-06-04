module voting::ve_token {
    use aptos_framework::event;
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::timestamp;
    use aptos_std::math128;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::table::{Self, Table};
    use std::signer;
    use voting::vote_token;

    /// The maximum number of locks a user or the global locks tracker can have.
    /// This is to ensure neither list grows too large and leads to gas limit issues.
    /// This should be at least equal to the maximum number of epochs that count in voting power calculation.
    /// For example, if voting power can grow over 52 epochs (e.g. 52 weeks) and stops afterward, the maximum number of
    /// locks should be at least 52 to ensure no loss of voting power.
    const MAX_LOCKS: u64 = 52;
    const MAX_WITHDRAW_RECORDS: u64 = 50;
    const VOTE_MULTIPLIER_NUMERATOR: u128 = 1;
    const VOTE_MULTIPLIER_DENOMINATOR: u128 = 13;

    const EPOCH_DURATION: u64 = 604800; // 7 days in seconds
    const UNLOCK_DELAY: u64 = 2592000; // 30 days in seconds
    
    /// User cannot unlock more than the total amount they've locked.
    const ECANNOT_UNLOCK_MORE_THAN_TOTAL_LOCKED: u64 = 1;
    /// User has no locks to unlock from.
    const ENO_USER_LOCKS: u64 = 2;
    /// Invalid protocol state. User locks exist but no global locks are found.
    const EINVALID_STATE_NO_GLOBAL_LOCKS: u64 = 3;
    /// User cannot withdraw more than the amount they have unlocked.
    const EZERO_WITHDRAWABLE_AMOUNT: u64 = 4;

    struct VeTokenManagement has key {
        /// The time when the first epoch starts (same as deployment time).
        start_epoch_time: u64,
        /// Epoch is a core concept in the voting system. Voting power for each lock remains constant during each epoch
        /// All deposits and withdrawals within the same epoch are combined to reduce the number of locks.
        epoch_duration: u64,
        /// After requesting to unlock, users will have to wait for a certain period before they can withdraw their tokens
        unlock_delay: u64,
        /// Maximum number of epochs that the voting power can grow. After that, the voting power will remain constant.
        maximum_lock_epochs: u64,
        /// The multiplier applied as a "bonus" on locked vote tokens to calculate voting power.
        /// Separated into numerator and denominator to allow fractional value.
        vote_multiplier_numerator: u128,
        vote_multiplier_denominator: u128,
        /// Track locks by epoch for each user. The number of locks will be capped by MAX_LOCKS.
        /// Each user interaction can trim this list if needed.
        /// User deposits will be added to the user lock for that epoch.
        /// User unlocks will be deducted from the earliest possible lock.
        user_locks: Table<address, LockRecords>,
        /// Track total locks by epoch for the entire system. The number of locks will be capped by MAX_LOCKS.
        /// User deposits in the same epoch will be added to the global lock for that epoch.
        /// User unlocks are deducted from the global lock for the same epoch as the user lock that are unlocked from.
        global_locks: LockRecords,
        /// Track unlock requests for each user.
        /// There's a cap on the number of unlock records to prevent the list from growing too large.
        /// User will need to wait to withdraw if they hit the cap.
        user_unlocks: Table<address, vector<TokenUnlock>>,
        /// Store the vote tokens
        vote_token_store: ExtendRef,
    }

    /// Each time user locks their tokens, a new lock is created.
    /// The voting power for each lock grows as time passes, up to the maximum number of epochs specified in the config.
    /// After that point, the voting power for the lock will remain constant.
    /// Unlocking tokens will require waiting for a certain period before the tokens can be withdrawn.
    struct LockRecords has copy, drop, store {
        /// Since the number of locks is capped, we can use a simple map to track the locks without worrying about
        /// performance from it growing too large.
        /// We expect the simple map to be sorted ascending by epoch.
        locks: SimpleMap<u64, u64>,
    }

    /// Each unlock request will be recorded with the amount and the time when it can be withdrawn.
    struct TokenUnlock has copy, drop, store {
        /// Amount of tokens that can be unlocked.
        unlock_amount: u64,
        /// Timestamp (in secs) when the unlock can be withdrawn.
        unlocks_at: u64,
    }

    #[event]
    struct LockEvent has drop, store {
        /// The address of the user who locked the tokens.
        user: address,
        /// The amount of tokens that were locked.
        lock_amount: u64,
        /// The epoch when the lock was created.
        lock_epoch: u64,
        /// The total amount of tokens locked at the epoch of the lock.
        total_lock_at_epoch: u64,
    }

    #[event]
    struct UnlockEvent has drop, store {
        /// The address of the user who requested the unlock.
        user: address,
        /// The amount of tokens that can be unlocked.
        unlock_amount: u64,
        /// The timestamp (in secs) when the unlock can be withdrawn.
        unlocks_at: u64,
        /// The epoch when the unlock was requested.
        requested_epoch: u64,
        /// The epochs at which locks were updated for the requestor and globally.
        updated_lock_epochs: vector<u64>
    }

    #[event]
    struct WithdrawEvent has drop, store {
        /// The address of the user who withdrew the tokens.
        user: address,
        /// The amount of tokens that were withdrawn.
        withdraw_amount: u64,
        /// The timestamp (in secs) when the withdraw was made.
        withdraw_at: u64,
    }

    fun init_module(voting_signer: &signer) {
        let vote_token_store = &object::create_object(@voting);
        fungible_asset::create_store(vote_token_store, vote_token::token());
        move_to(voting_signer, VeTokenManagement {
            start_epoch_time: timestamp::now_seconds(),
            epoch_duration: EPOCH_DURATION,
            unlock_delay: UNLOCK_DELAY,
            maximum_lock_epochs: MAX_LOCKS,
            vote_multiplier_numerator: VOTE_MULTIPLIER_NUMERATOR,
            vote_multiplier_denominator: VOTE_MULTIPLIER_DENOMINATOR,
            user_locks: table::new(),
            global_locks: LockRecords { locks: simple_map::new() },
            user_unlocks: table::new(),
            vote_token_store: object::generate_extend_ref(vote_token_store),
        });
    }

    #[view]
    public fun current_epoch(): u64 acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        (timestamp::now_seconds() - config.start_epoch_time) / config.epoch_duration + 1
    }

    #[view]
    public fun total_voting_power_at(ending_epoch: u64): u128 acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        calculate_voting_power(
            &config.global_locks.locks,
            ending_epoch,
            config.maximum_lock_epochs,
            config.vote_multiplier_numerator,
            config.vote_multiplier_denominator
        )
    }

    #[view]
    public fun voting_power_at(user: address, ending_epoch: u64): u128 acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        let user_locks = &config.user_locks.borrow_with_default(user, &LockRecords {
            locks: simple_map::new()
        }).locks;
        calculate_voting_power(
            user_locks,
            ending_epoch,
            config.maximum_lock_epochs,
            config.vote_multiplier_numerator,
            config.vote_multiplier_denominator
        )
    }

    #[view]
    /// Return total withdrawable and the remaining unexpired unlocks for a user.
    public fun withdrawable(user: address): (u64, vector<TokenUnlock>) acquires VeTokenManagement {
        let all_user_unlocks = &VeTokenManagement[@voting].user_unlocks;
        let user_unlocks = all_user_unlocks.borrow(user);
        let remaining_unlocks = vector[];
        let total_withdraw = 0;
        user_unlocks.for_each_ref(|unlock| {
            let unlock: &TokenUnlock = unlock;
            if (unlock.unlocks_at <= timestamp::now_seconds()) {
                total_withdraw += unlock.unlock_amount;
            } else {
                // User cannot withdraw the unlock amount yet.
                remaining_unlocks.push_back(*unlock);
            };
        });

        (total_withdraw, remaining_unlocks)
    }

    #[view]
    public fun locked_amount_at(user: address, epoch: u64): u64 acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        if (!config.user_locks.contains(user)) return 0;

        let user_locks = &config.user_locks.borrow(user).locks;
        if (user_locks.contains_key(&epoch)) {
            *user_locks.borrow(&epoch)
        } else {
            0
        }
    }

    #[view]
    public fun total_locked_amount(user: address): u64 acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        if (!config.user_locks.contains(user)) return 0;

        let user_locks = &config.user_locks.borrow(user).locks;
        user_locks.values().fold(0, |acc, amount| acc + amount)
    }

    #[view]
    public fun global_locked_amount_at(epoch: u64): u64 acquires VeTokenManagement {
        let global_locks = &VeTokenManagement[@voting].global_locks.locks;
        if (global_locks.contains_key(&epoch)) {
            *global_locks.borrow(&epoch)
        } else {
            0
        }
    }

    #[view]
    public fun total_global_locked_amount(): u64 acquires VeTokenManagement {
        let global_locks = &VeTokenManagement[@voting].global_locks.locks;
        global_locks.values().fold(0, |acc, amount| acc + amount)
    }

    #[view]
    public fun num_user_locks(user: address): u64 acquires VeTokenManagement {
        let user_locks = &VeTokenManagement[@voting].user_locks;
        if (!user_locks.contains(user)) return 0;
        user_locks.borrow(user).locks.length()
    }

    #[view]
    public fun num_global_locks(): u64 acquires VeTokenManagement {
        VeTokenManagement[@voting].global_locks.locks.length()
    }

    #[view]
    public fun epoch_duration(): u64 acquires VeTokenManagement {
        VeTokenManagement[@voting].epoch_duration
    }

    #[view]
    public fun unlock_delay(): u64 acquires VeTokenManagement {
        VeTokenManagement[@voting].unlock_delay
    }

    #[view]
    public fun voting_multiplier(): (u128, u128) acquires VeTokenManagement {
        let config = &VeTokenManagement[@voting];
        (config.vote_multiplier_numerator, config.vote_multiplier_denominator)
    }

    #[view]
    public fun maximum_lock_epochs(): u64 acquires VeTokenManagement {
        VeTokenManagement[@voting].maximum_lock_epochs
    }

    //////////////////////////////////// User interactions ///////////////////////////////////////

    public entry fun lock(user: &signer, amount: u64) acquires VeTokenManagement {
        // Always compact locks first to prevent user locks from growing too large.
        compact_user_locks(user);

        let current_epoch = current_epoch();
        let user_addr = signer::address_of(user);
        let protocol_token_store = protocol_token_store();
        let config = &mut VeTokenManagement[@voting];

        // Transfer the tokens from the user's primary store to the protocol token store.
        let vote_token = vote_token::token();
        fungible_asset::transfer(
            user,
            primary_fungible_store::primary_store(user_addr, vote_token),
            protocol_token_store,
            amount,
        );

        // Update the user lock records:
        // 1. Earliest lock epoch is set to the current epoch if it is unset (0).
        // 2. Add the new lock to the user lock records. If one already exists for current epoch, update the amount.
        if (!config.user_locks.contains(user_addr)) {
            config.user_locks.add(user_addr, LockRecords { locks: simple_map::new() });
        };
        let user_lock_records = config.user_locks.borrow_mut(user_addr);
        let total_lock_at_epoch = create_or_add_to_existing_lock(current_epoch, user_lock_records, amount);

        // Update the global lock records
        create_or_add_to_existing_lock(current_epoch, &mut config.global_locks, amount);

        event::emit(LockEvent {
            user: user_addr,
            lock_amount: amount,
            lock_epoch: current_epoch,
            total_lock_at_epoch,
        });
    }

    public entry fun unlock(user: &signer, unlock_amount: u64) acquires VeTokenManagement {
        // Always compact locks first to prevent user locks from growing too large.
        compact_user_locks(user);

        let user_addr = signer::address_of(user);
        let config = &mut VeTokenManagement[@voting];
        let user_lock_records = config.user_locks.borrow_mut(user_addr);
        let user_locks = &mut user_lock_records.locks;
        assert!(user_locks.length() > 0, ENO_USER_LOCKS);
        let global_locks = &mut config.global_locks.locks;
        assert!(global_locks.length() > 0, EINVALID_STATE_NO_GLOBAL_LOCKS);
        
        // Deduct the amount from locks starting from the earliest. If the locks are insufficient, abort.
        let updated_lock_epochs = vector[];
        let remaining_amount = unlock_amount;
        let user_epochs = user_locks.keys();
        for (i in 0..user_epochs.length()) {
            let epoch = user_epochs.borrow(i);
            if (!user_locks.contains_key(epoch)) {
                continue;
            };

            // Update the lock if it has enough amount to cover the unlock or else remove it.
            updated_lock_epochs.push_back(*epoch);
            let locked_amount = user_locks.borrow_mut(epoch);
            let amount_used = if (*locked_amount > remaining_amount) {
                *locked_amount -= remaining_amount;
                remaining_amount
            } else {
                let locked_amount = *locked_amount;
                user_locks.remove(epoch);
                locked_amount
            };

            remaining_amount -= amount_used;
            let global_locked_amount = global_locks.borrow_mut(epoch);
            *global_locked_amount -= amount_used;
            if (*global_locked_amount == 0) {
                global_locks.remove(epoch);
            };

            // Break early to save gas if we have unlocked the entire amount.
            if (remaining_amount == 0) {
                break;
            };
        };
        // User does not have enough locked tokens to cover the request amount.
        assert!(remaining_amount == 0, ECANNOT_UNLOCK_MORE_THAN_TOTAL_LOCKED);

        // Update the unlock records for the user.
        if (!config.user_unlocks.contains(user_addr)) {
            config.user_unlocks.add(user_addr, vector[]);
        };
        let unlocks = config.user_unlocks.borrow_mut(user_addr);
        let unlocks_at = timestamp::now_seconds() + config.unlock_delay;
        unlocks.push_back(TokenUnlock {
            unlock_amount,
            unlocks_at,
        });

        event::emit(UnlockEvent {
            user: user_addr,
            unlock_amount,
            unlocks_at,
            requested_epoch: current_epoch(),
            updated_lock_epochs
        });
    }

    public entry fun withdraw(user: &signer) acquires VeTokenManagement {
        let user_addr = signer::address_of(user);
        let (withdraw_amount, remaining_unlocks) = withdrawable(user_addr);
        assert!(withdraw_amount > 0, EZERO_WITHDRAWABLE_AMOUNT);

        let protocol_token_store = protocol_token_store();
        let config = &mut VeTokenManagement[@voting];
        config.user_unlocks.upsert(user_addr, remaining_unlocks);

        // Transfer tokens to user.
        let protocol_store_signer = &object::generate_signer_for_extending(&config.vote_token_store);
        fungible_asset::transfer(
            protocol_store_signer,
            protocol_token_store,
            primary_fungible_store::primary_store(user_addr, vote_token::token()),
            withdraw_amount,
        );

        event::emit(WithdrawEvent {
            user: user_addr,
            withdraw_amount,
            withdraw_at: timestamp::now_seconds(),
        });
    }

    public entry fun compact_user_locks(user: &signer) acquires VeTokenManagement {
        compact_global_locks();

        let user_addr = signer::address_of(user);
        let current_epoch = current_epoch();
        let config = &mut VeTokenManagement[@voting];
        if (!config.user_locks.contains(user_addr)) {
            return;
        };
        let user_locks = &mut config.user_locks.borrow_mut(user_addr).locks;
        compact_locks(user_locks, current_epoch, config.maximum_lock_epochs);
    }

    public entry fun compact_global_locks() acquires VeTokenManagement {
        let current_epoch = current_epoch();
        let config = &mut VeTokenManagement[@voting];
        let global_locks = &mut config.global_locks.locks;
        if (global_locks.length() == 0) {
            return;
        };
        compact_locks(global_locks, current_epoch, config.maximum_lock_epochs);
    }

    //////////////////////////////////// Private functions ///////////////////////////////////////

    inline fun protocol_token_store(): Object<FungibleStore> {
        object::address_to_object<FungibleStore>(
            object::address_from_extend_ref(&VeTokenManagement[@voting].vote_token_store)
        )
    }

    fun calculate_voting_power(
        locks: &SimpleMap<u64, u64>,
        ending_epoch: u64,
        maximum_lock_epochs: u64,
        voting_multiplier_numerator: u128,
        voting_multiplier_denominator: u128,
    ): u128 {
        let ending_epoch = ending_epoch as u128;
        let maximum_lock_epochs = maximum_lock_epochs as u128;
        let total_voting_power = 0;
        locks.keys().for_each(|epoch| {
            let locked_amount = *locks.borrow(&epoch) as u128;
            let epoch = epoch as u128;
            // Amount added in the ending epoch doesn't count towards voting power.
            if (epoch < ending_epoch) {
                let num_epochs_locked = math128::min(ending_epoch - epoch, maximum_lock_epochs);
                let voting_power_bonus = math128::mul_div(
                    locked_amount,
                    num_epochs_locked * voting_multiplier_numerator,
                    voting_multiplier_denominator
                );
                let epoch_voting_power = locked_amount + voting_power_bonus;
                total_voting_power += epoch_voting_power;
            };
        });
        total_voting_power
    }

    fun create_or_add_to_existing_lock(epoch: u64, lock_records: &mut LockRecords, amount: u64): u64 {
        let locks = &mut lock_records.locks;
        if (locks.contains_key(&epoch)) {
            let locked_amount = locks.borrow_mut(&epoch);
            *locked_amount += amount;
            *locked_amount
        } else {
            locks.add(epoch, amount);
            amount
        }
    }

    fun compact_locks(locks: &mut SimpleMap<u64, u64>, current_epoch: u64, maximum_lock_epochs: u64) {
        if (current_epoch <= maximum_lock_epochs) {
            return;
        };

        // Remove the locks that are older than the maximum lock epochs and track the carry forward amount.
        let carry_forward = 0;
        let min_epoch = current_epoch - maximum_lock_epochs;
        locks.keys().for_each(|epoch| {
            if (epoch < min_epoch) {
                carry_forward += *locks.borrow(&epoch);
                locks.remove(&epoch);
            };
        });

        if (carry_forward > 0) {
            let new_min_epoch = min_epoch;
            // If there's an existing lock with the new minimum epoch, add the carry forward amount to it.
            // Otherwise, create a new lock for the carry forward amount.
            if (locks.contains_key(&new_min_epoch)) {
                *locks.borrow_mut(&new_min_epoch) += carry_forward;
            } else {
                locks.add(new_min_epoch, carry_forward);
            }
        }
    }

    #[test_only]
    public fun init_for_test(deployer: &signer) {
        init_module(deployer);
    }
}
