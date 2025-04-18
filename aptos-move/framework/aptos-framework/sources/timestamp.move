/// This module keeps a global wall clock that stores the current Unix time in microseconds.
/// It interacts with the other modules in the following ways:
/// * genesis: to initialize the timestamp
/// * block: to reach consensus on the global wall clock time
module aptos_framework::timestamp {
    use aptos_framework::system_addresses;
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use std::error;

    friend aptos_framework::genesis;

    /// A singleton resource holding the current Unix time in microseconds
    struct CurrentTimeMicroseconds has key {
        microseconds: u64,
    }

    /// Current Unix time in microseconds represented as aggregator
    struct ConflictFreeTimeMicroseconds has key {
        microseconds: Aggregator<u64>,
    }

    /// Conversion factor between seconds and microseconds
    const MICRO_CONVERSION_FACTOR: u64 = 1000000;

    /// The blockchain is not in an operating state yet
    const ENOT_OPERATING: u64 = 1;
    /// An invalid timestamp was provided
    const EINVALID_TIMESTAMP: u64 = 2;

    /// Marks that time has started. This can only be called from genesis and with the aptos framework account.
    public(friend) fun set_time_has_started(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        let timer = CurrentTimeMicroseconds { microseconds: 0 };
        move_to(aptos_framework, timer);
    }

    /// Initializes the conflict-free timer so it can be used for expiration check. This can only be called with the aptos framework account.
    public(friend) fun initialize_conflict_free_timer(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        let timer = ConflictFreeTimeMicroseconds { microseconds: aggregator_v2::create_unbounded_aggregator_with_value(0) };
        move_to(aptos_framework, timer);
    }

    /// Updates the wall clock time by consensus. Requires VM privilege and will be invoked during block prologue.
    public fun update_global_time(
        account: &signer,
        proposer: address,
        timestamp: u64
    ) acquires CurrentTimeMicroseconds, ConflictFreeTimeMicroseconds {
        // Can only be invoked by AptosVM signer.
        system_addresses::assert_vm(account);

        let global_timer = borrow_global_mut<CurrentTimeMicroseconds>(@aptos_framework);
        let now = global_timer.microseconds;
        if (proposer == @vm_reserved) {
            // NIL block with null address as proposer. Timestamp must be equal.
            assert!(now == timestamp, error::invalid_argument(EINVALID_TIMESTAMP));
        } else {
            // Normal block. Time must advance
            assert!(now < timestamp, error::invalid_argument(EINVALID_TIMESTAMP));
            global_timer.microseconds = timestamp;
        };

        if (exists<ConflictFreeTimeMicroseconds>(@aptos_framework)) {
            let conflict_free_timer = borrow_global_mut<ConflictFreeTimeMicroseconds>(@aptos_framework);
            let now = aggregator_v2::read(&conflict_free_timer.microseconds);
            aggregator_v2::add(&mut conflict_free_timer.microseconds, timestamp - now);
        }
    }

    /// Returns true if the given timestamp is smaller than the current time on-chain;
    public fun is_expired(timestamp_second: u64): bool acquires CurrentTimeMicroseconds, ConflictFreeTimeMicroseconds {
        if (exists<ConflictFreeTimeMicroseconds>(@aptos_framework)) {
            let global_timer = borrow_global_mut<ConflictFreeTimeMicroseconds>(@aptos_framework);
            aggregator_v2::is_at_least(&global_timer.microseconds, timestamp_second * MICRO_CONVERSION_FACTOR)
        } else {
            let global_timer = borrow_global_mut<CurrentTimeMicroseconds>(@aptos_framework);
            global_timer.microseconds >= timestamp_second * MICRO_CONVERSION_FACTOR
        }
    }

    #[test_only]
    public fun set_time_has_started_for_testing(account: &signer) {
        if (!exists<CurrentTimeMicroseconds>(@aptos_framework)) {
            set_time_has_started(account);
        };
    }

    #[view]
    /// Gets the current time in microseconds.
    public fun now_microseconds(): u64 acquires CurrentTimeMicroseconds {
        borrow_global<CurrentTimeMicroseconds>(@aptos_framework).microseconds
    }

    #[view]
    /// Gets the current time in seconds.
    public fun now_seconds(): u64 acquires CurrentTimeMicroseconds {
        now_microseconds() / MICRO_CONVERSION_FACTOR
    }

    #[test_only]
    public fun update_global_time_for_test(timestamp_microsecs: u64) acquires CurrentTimeMicroseconds {
        let global_timer = borrow_global_mut<CurrentTimeMicroseconds>(@aptos_framework);
        let now = global_timer.microseconds;
        assert!(now < timestamp_microsecs, error::invalid_argument(EINVALID_TIMESTAMP));
        global_timer.microseconds = timestamp_microsecs;
    }

    #[test_only]
    public fun update_global_time_for_test_secs(timestamp_seconds: u64) acquires CurrentTimeMicroseconds {
        update_global_time_for_test(timestamp_seconds * MICRO_CONVERSION_FACTOR);
    }

    #[test_only]
    public fun fast_forward_seconds(timestamp_seconds: u64) acquires CurrentTimeMicroseconds {
        update_global_time_for_test_secs(now_seconds() + timestamp_seconds);
    }
}
