module aptos_framework::token_bucket {
    use aptos_std::math64;
    use aptos_framework::timestamp;

    // Struct to represent a Token Bucket that refills every minute
    struct Bucket has key, store, copy, drop {
        // Maximum number of transactions allowed per minute
        capacity: u64,
        // Current number of transactions remaining in this minute
        tokens: u64,
        // Transactions added per minute
        refill_rate_per_minute: u64,
        // Last time the bucket was refilled (in seconds)
        last_refill_timestamp: u64,
        // Time accumulated that hasn't yet added up to a full token
        fractional_time_accumulated: u64,
    }

    // Public entry function to initialize a Token Bucket for transactions per minute
    public fun initialize_bucket(capacity: u64): Bucket {
        let bucket = Bucket {
            capacity,
            tokens: capacity, // Start with a full bucket (full capacity of transactions allowed)
            refill_rate_per_minute: capacity,
            last_refill_timestamp: timestamp::now_seconds(),
            fractional_time_accumulated: 0, // Start with no fractional time accumulated
        };
        bucket
    }

    // Public function to request a transaction from the bucket
    public fun request(bucket: &mut Bucket, num_token_requested: u64): bool {
        refill(bucket);
        if (bucket.tokens >= num_token_requested) {
            bucket.tokens = bucket.tokens - num_token_requested;
            true
        } else {
            false
        }
    }

    // Function to refill the transactions in the bucket based on time passed (in minutes)
    fun refill(bucket: &mut Bucket) {
        let current_time = timestamp::now_seconds();
        let time_passed = current_time - bucket.last_refill_timestamp;

        // Total time passed including fractional accumulated time
        let total_time = time_passed + bucket.fractional_time_accumulated;

        // Calculate the full tokens that can be added
        let new_tokens = total_time * bucket.refill_rate_per_minute / 60;

        // Calculate the remaining fractional time
        let remaining_fractional_time = total_time % 60;

        // Refill the bucket with the full tokens
        if (new_tokens > 0) {
            bucket.tokens = math64::min(bucket.tokens + new_tokens, bucket.capacity);
            bucket.last_refill_timestamp = current_time;
        };

        // Update the fractional time accumulated for the next refill cycle
        bucket.fractional_time_accumulated = remaining_fractional_time;
    }

    #[test(aptos_framework = @0x1)]
    fun test_initialize_bucket(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);
        assert!(bucket.capacity == 10, 100);
        assert!(bucket.tokens == 10, 101);
        assert!(bucket.refill_rate_per_minute == 10, 102);
    }

    #[test(aptos_framework = @0x1)]
    fun test_request_success(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);
        let success = request(&mut bucket, 5);
        assert!(success, 200); // Should succeed since 5 <= 10
        assert!(bucket.tokens == 5, 201); // Remaining tokens should be 5
    }

    #[test(aptos_framework = @0x1)]
    fun test_request_failure(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);
        let success = request(&mut bucket, 15);
        assert!(!success, 300); // Should fail since 15 > 10
        assert!(bucket.tokens == 10, 301); // Tokens should remain unchanged
    }

    #[test(aptos_framework = @0x1)]
    fun test_refill(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);

        // Simulate a passage of 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill the bucket
        refill(&mut bucket);

        // Should have refilled 5 tokens (half of the capacity)
        assert!(bucket.tokens == 10, 400); // Bucket was already full, so should remain full

        // Request 5 tokens
        let success = request(&mut bucket, 5);
        assert!(success, 401); // Request should succeed
        assert!(bucket.tokens == 5, 402); // Remaining tokens should be 5

        // Simulate another passage of 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill again
        refill(&mut bucket);

        // Should refill 5 tokens, bringing back to full
        assert!(bucket.tokens == 10, 403); // Should now be full again
    }

    #[test(aptos_framework= @0x1)]
    fun test_fractional_accumulation(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);

        // Simulate 10 seconds passing
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 10);

        // Refill the bucket
        refill(&mut bucket);

        // Should add 1/6th of the tokens (because 10 seconds is 1/6th of a minute)
        assert!(bucket.tokens == 10, 500); // No token will be added since it rounds down
        assert!(bucket.fractional_time_accumulated == 10, 501); // Accumulate the 10 seconds of fractional time

        // Simulate another 50 seconds passing (total 60 seconds)
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 50);

        // Refill the bucket again
        refill(&mut bucket);

        // Should refill 10 tokens (full minute passed)
        assert!(bucket.tokens == 10, 502); // Should be full now
        assert!(bucket.fractional_time_accumulated == 0, 503); // Fractional time should reset
    }

    #[test(aptos_framework= @0x1)]
    fun test_multiple_refills(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let bucket = initialize_bucket(10);

        // Request 8 tokens
        let success = request(&mut bucket, 8);
        assert!(success, 600); // Should succeed
        assert!(bucket.tokens == 2, 601); // Remaining tokens should be 2

        // Simulate a passage of 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill the bucket
        refill(&mut bucket);
        assert!(bucket.tokens == 7, 602); // Should add 5 tokens (half of the refill rate)

        // Simulate another 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill the bucket again
        refill(&mut bucket);
        assert!(bucket.tokens == 10, 603); // Should be full again
    }
}
