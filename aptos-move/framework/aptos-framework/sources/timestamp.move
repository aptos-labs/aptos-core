/// This module keeps a global wall clock that stores the current Unix time in microseconds.
/// It interacts with the other modules in the following ways:
/// * genesis: to initialize the timestamp
/// * block: to reach consensus on the global wall clock time
module aptos_framework::timestamp {
    use aptos_framework::system_addresses;
    use std::error;
    use std::vector;

    friend aptos_framework::genesis;

    /// A singleton resource holding the current Unix time in microseconds
    struct CurrentTimeMicroseconds has key {
        microseconds: u64,
    }

    /// Conversion factor between seconds and microseconds
    const MICRO_CONVERSION_FACTOR: u64 = 1000000;
    /// Bits per `u8`
    const BITS_PER_BYTE: u8 = 8;
    /// Bytes in a `u64`
    const BYTES_PER_TIMESTAMP: u8 = 8;
    /// Bitmask for the least-significant byte of a `u64`
    const LEAST_SIGNIFICANT_BYTE_MASK: u64 = 0xff;

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

    /// Updates the wall clock time by consensus. Requires VM privilege and will be invoked during block prologue.
    public fun update_global_time(
        account: &signer,
        proposer: address,
        timestamp: u64
    ) acquires CurrentTimeMicroseconds {
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
        };
        global_timer.microseconds = timestamp;
    }

    #[test_only]
    public fun set_time_has_started_for_testing(account: &signer) {
        set_time_has_started(account);
    }

    /// Gets the current time in microseconds.
    public fun now_microseconds(): u64 acquires CurrentTimeMicroseconds {
        borrow_global<CurrentTimeMicroseconds>(@aptos_framework).microseconds
    }

    /// Return the little-endian `u8`-vectorized time in microseconds,
    /// useful as a form of quasi-nondeterminism.
    public fun now_microseconds_vectorized():
    vector<u8>
    acquires CurrentTimeMicroseconds {
        let timestamp = now_microseconds(); // Get time in microseconds
        let b = 0; // Initialize loop variable for byte under review
        let vectorized = vector::empty(); // Declare empty vector
        while (b < BYTES_PER_TIMESTAMP) { // Loop over all bytes
            // Get the least significant byte from the timestamp value
            let byte = (timestamp & LEAST_SIGNIFICANT_BYTE_MASK as u8);
            // Push back byte to end of vector
            vector::push_back(&mut vectorized, byte);
            // Bitshift out the byte just extracted
            timestamp = timestamp >> BITS_PER_BYTE;
            b = b + 1; // Increment byte counter
        }; // All bytes have been extracted
        vectorized // Return little-endian vectorized time
    }

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
    public fun fast_forward_seconds(timestamp_seconds: u64) acquires CurrentTimeMicroseconds {
        update_global_time_for_test(now_microseconds() + timestamp_seconds * 1000000);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Verify little-endian vectorization
    fun test_now_microseconds_vectorized(
        aptos_framework: &signer
    ) acquires CurrentTimeMicroseconds {
        // Initialize timetamp resource
        set_time_has_started_for_testing(aptos_framework);
        // Update time to easily-inspected value
        update_global_time_for_test(0x0123456789abcdef);
        // Get little-endian vectorized Unit time in microseconds
        let vectorized = now_microseconds_vectorized();
        // Assert vectorized byte values
        assert!(*vector::borrow(&vectorized, 0) == 0xef, 0);
        assert!(*vector::borrow(&vectorized, 1) == 0xcd, 0);
        assert!(*vector::borrow(&vectorized, 2) == 0xab, 0);
        assert!(*vector::borrow(&vectorized, 3) == 0x89, 0);
        assert!(*vector::borrow(&vectorized, 4) == 0x67, 0);
        assert!(*vector::borrow(&vectorized, 5) == 0x45, 0);
        assert!(*vector::borrow(&vectorized, 6) == 0x23, 0);
        assert!(*vector::borrow(&vectorized, 7) == 0x01, 0);
    }
}
