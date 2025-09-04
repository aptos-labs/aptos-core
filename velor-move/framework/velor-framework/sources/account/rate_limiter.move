module velor_framework::rate_limiter {
    use velor_framework::timestamp;

    enum RateLimiter has key, store, copy, drop {
        // Struct to represent a Token Bucket that refills every minute
        TokenBucket {
            // Maximum number of tokens allowed at any time.
            capacity: u64,
            // Current number of tokens remaining in this interval.
            current_amount: u64,
            // refill `capacity` number of tokens every `refill_interval` in seconds.
            refill_interval: u64,
            // Last time the bucket was refilled (in seconds)
            last_refill_timestamp: u64,
            // accumulated amount that hasn't yet added up to a full token
            fractional_accumulated: u64,
        }
    }

    // Public entry function to initialize a Token Bucket based rate limiter.
    public fun initialize(capacity: u64, refill_interval: u64): RateLimiter {
        RateLimiter::TokenBucket {
            capacity,
            current_amount: capacity, // Start with a full bucket (full capacity of transactions allowed)
            refill_interval,
            last_refill_timestamp: timestamp::now_seconds(),
            fractional_accumulated: 0, // Start with no fractional accumulated
        }
    }

    // Public function to request a transaction from the bucket
    public fun request(limiter: &mut RateLimiter, num_token_requested: u64): bool {
        refill(limiter);
        if (limiter.current_amount >= num_token_requested) {
            limiter.current_amount = limiter.current_amount - num_token_requested;
            true
        } else {
            false
        }
    }

    // Function to refill the transactions in the bucket based on time passed
    fun refill(limiter: &mut RateLimiter) {
        let current_time = timestamp::now_seconds();
        let time_passed = current_time - limiter.last_refill_timestamp;
        // Calculate the full tokens that can be added
        let accumulated_amount = time_passed * limiter.capacity + limiter.fractional_accumulated;
        let new_tokens = accumulated_amount / limiter.refill_interval;
        if (limiter.current_amount + new_tokens >= limiter.capacity) {
            limiter.current_amount = limiter.capacity;
            limiter.fractional_accumulated = 0;
        } else {
            limiter.current_amount = limiter.current_amount + new_tokens;
            // Update the fractional amount accumulated for the next refill cycle
            limiter.fractional_accumulated = accumulated_amount % limiter.refill_interval;
        };
        limiter.last_refill_timestamp = current_time;
    }

    #[test(velor_framework = @0x1)]
    fun test_initialize_bucket(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 60);
        assert!(bucket.capacity == 10, 100);
        assert!(bucket.current_amount == 10, 101);
        assert!(bucket.refill_interval == 60, 102);
    }

    #[test(velor_framework = @0x1)]
    fun test_request_success(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 30);
        let success = request(&mut bucket, 5);
        assert!(success, 200); // Should succeed since 5 <= 10
        assert!(bucket.current_amount == 5, 201); // Remaining tokens should be 5
    }

    #[test(velor_framework = @0x1)]
    fun test_request_failure(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 30);
        let success = request(&mut bucket, 15);
        assert!(!success, 300); // Should fail since 15 > 10
        assert!(bucket.current_amount == 10, 301); // Tokens should remain unchanged
    }

    #[test(velor_framework = @0x1)]
    fun test_refill(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 60);

        // Simulate a passage of 31 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 31);

        // Refill the bucket
        refill(&mut bucket);

        // Should have refilled 5 tokens (half of the capacity),
        // but bucket was already full, so should remain full
        assert!(bucket.current_amount == 10, 400);
        assert!(bucket.fractional_accumulated == 0, 401);

        // Request 5 tokens
        let success = request(&mut bucket, 5);
        assert!(success, 401); // Request should succeed
        assert!(bucket.current_amount == 5, 402); // Remaining tokens should be 5
        assert!(bucket.fractional_accumulated == 0, 403);

        // Simulate another passage of 23 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 23);

        // Refill again
        refill(&mut bucket);

        // Should refill 3 tokens
        assert!(bucket.current_amount == 8, 403);
        // and have 230-180 leftover
        assert!(bucket.fractional_accumulated == 50, 404);
    }

    #[test(velor_framework= @0x1)]
    fun test_fractional_accumulation(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 60);
        assert!(request(&mut bucket, 10), 1); // Request should succeed

        assert!(bucket.current_amount == 0, 500); // No token will be added since it rounds down

        // Simulate 10 seconds passing
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 10);

        // Refill the bucket
        refill(&mut bucket);
        // Should add 1/6th of the tokens (because 10 seconds is 1/6th of a minute)
        assert!(bucket.current_amount == 1, 500); // 1 token will be added since it rounds down
        assert!(bucket.fractional_accumulated == 40, 501); // Accumulate the 4 seconds of fractional amount

        // Simulate another 50 seconds passing (total 60 seconds)
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 50);

        // Refill the bucket again
        refill(&mut bucket);

        assert!(bucket.current_amount == 10, 502); // Should be full now
        assert!(bucket.fractional_accumulated == 0, 503); // Fractional time should reset
    }

    #[test(velor_framework= @0x1)]
    fun test_multiple_refills(velor_framework: &signer) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let bucket = initialize(10, 60);

        // Request 8 tokens
        let success = request(&mut bucket, 8);
        assert!(success, 600); // Should succeed
        assert!(bucket.current_amount == 2, 601); // Remaining tokens should be 2

        // Simulate a passage of 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill the bucket
        refill(&mut bucket);
        assert!(bucket.current_amount == 7, 602); // Should add 5 tokens (half of the refill rate)

        // Simulate another 30 seconds
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + 30);

        // Refill the bucket again
        refill(&mut bucket);
        assert!(bucket.current_amount == 10, 603); // Should be full again
    }
}
