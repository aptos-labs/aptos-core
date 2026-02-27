// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_infallible::Mutex;
use aptos_time_service::{TimeService, TimeServiceTrait};
use std::{
    cmp::min,
    sync::Arc,
    time::{Duration, Instant},
};

/// Type alias for a thread-safe shared token bucket
pub type SharedTokenBucket = Arc<Mutex<TokenBucket>>;

/// A token bucket rate limiter implementing the token bucket algorithm:
/// - The bucket has a maximum capacity of tokens.
/// - Tokens are added to the bucket at a fixed rate (i.e., refill_rate in tokens/second).
/// - Operations consume tokens from the bucket.
/// - When tokens are exhausted, operations are throttled until enough tokens refill.
#[derive(Debug)]
pub struct TokenBucket {
    /// Current number of available tokens
    tokens: u64,
    /// Maximum capacity of the bucket
    capacity: u64,
    /// Refill rate in tokens per second
    refill_rate: u64,
    /// Last time the bucket was refilled
    last_refill_time: Instant,
    /// Time service for getting the current time
    time_service: TimeService,
}

impl TokenBucket {
    /// Creates a new token bucket with the initial tokens set to full capacity
    pub fn new(capacity: u64, refill_rate: u64, time_service: TimeService) -> Self {
        Self::new_with_initial_tokens(capacity, refill_rate, capacity, time_service)
    }

    /// Creates a new token bucket with the given initial token count
    pub fn new_with_initial_tokens(
        capacity: u64,
        refill_rate: u64,
        initial_tokens: u64,
        time_service: TimeService,
    ) -> Self {
        // Verify the given parameters
        assert!(
            capacity > 0 && refill_rate > 0,
            "Capacity and refill rate must be > 0!"
        );
        assert!(
            capacity >= refill_rate,
            "Bucket capacity must be >= refill rate!"
        );
        assert!(
            initial_tokens <= capacity,
            "Initial tokens must be <= capacity!"
        );

        // Create the token bucket
        Self {
            tokens: initial_tokens,
            capacity,
            refill_rate,
            last_refill_time: time_service.now(),
            time_service,
        }
    }

    /// Refills tokens based on the elapsed time since the last refill
    fn refill(&mut self) {
        // Determine how many refill intervals have passed
        let num_refill_intervals = self
            .time_service
            .now()
            .saturating_duration_since(self.last_refill_time)
            .as_secs();

        // Refill the token bucket
        if num_refill_intervals > 0 {
            // Add tokens based on elapsed time
            let num_tokens_to_add = num_refill_intervals.saturating_mul(self.refill_rate);
            self.add_tokens(num_tokens_to_add);

            // Update last refill time (avoid drift by adding intervals to original time)
            self.last_refill_time += Duration::from_secs(num_refill_intervals);
        }
    }

    /// Attempts to acquire all the requested tokens (all-or-nothing).
    ///
    /// Returns successfully if all tokens were acquired, or the time when they
    /// will be ready if not enough tokens are available now. If the request
    /// exceeds the bucket capacity, `Err(None)` is returned to indicate the
    /// request can never succeed.
    pub fn try_acquire_all(&mut self, requested: u64) -> Result<(), Option<Instant>> {
        if requested == 0 {
            return Ok(()); // Return early if no tokens are requested
        }

        // Refill tokens based on elapsed time
        self.refill();

        // Attempt to acquire all tokens
        if self.tokens >= requested {
            // Deduct the tokens
            self.deduct_tokens(requested);

            Ok(())
        } else {
            // Not enough tokens available, return when they will be ready
            Err(self.time_of_tokens_needed(requested))
        }
    }

    /// Adds tokens to the bucket (capped at capacity)
    fn add_tokens(&mut self, count: u64) {
        let new_token_count = self.tokens.saturating_add(count);
        self.tokens = min(self.capacity, new_token_count);
    }

    /// Deducts up to `requested` tokens from the bucket.
    ///
    /// Returns the actual number of tokens deducted (which may be
    /// less than requested if not enough tokens are available).
    fn deduct_tokens(&mut self, requested: u64) -> u64 {
        let tokens_allowed = min(self.tokens, requested);
        self.tokens = self.tokens.saturating_sub(tokens_allowed);
        tokens_allowed
    }

    /// Returns unused tokens to the bucket (e.g., if the operation used fewer tokens than expected)
    pub fn return_tokens(&mut self, count: u64) {
        self.add_tokens(count);
    }

    /// Returns when the requested number of tokens will be available.
    ///
    /// Returns `Some(instant)` when the tokens will be ready, or `None`
    /// if the request can never succeed (e.g., it exceeds bucket capacity).
    fn time_of_tokens_needed(&self, requested: u64) -> Option<Instant> {
        if self.capacity < requested {
            // Request exceeds bucket capacity, can never succeed
            None
        } else {
            let tokens_needed = requested.saturating_sub(self.tokens);
            let intervals = tokens_needed.div_ceil(self.refill_rate);
            self.last_refill_time
                .checked_add(Duration::from_secs(intervals))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_time_service::MockTimeService;

    #[test]
    fn test_add_tokens_overflow() {
        // Create a bucket with very high capacity
        let capacity = 100;
        let refill_rate = 10;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Use most tokens
        assert!(bucket.try_acquire_all(50).is_ok());
        assert_eq!(bucket.tokens, 50);

        // Return a huge number of tokens (should saturate at capacity)
        bucket.return_tokens(u64::MAX);
        assert_eq!(bucket.tokens, capacity);
    }

    #[test]
    fn test_deduct_tokens() {
        let capacity = 10;
        let refill_rate = 2;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Deduct fewer tokens than available: exact amount is deducted
        let deducted = bucket.deduct_tokens(3);
        assert_eq!(deducted, 3);
        assert_eq!(bucket.tokens, 7);

        // Deduct more tokens than available: only available tokens are deducted
        let deducted = bucket.deduct_tokens(10);
        assert_eq!(deducted, 7);
        assert_eq!(bucket.tokens, 0);

        // Deduct from empty bucket: nothing is deducted
        let deducted = bucket.deduct_tokens(5);
        assert_eq!(deducted, 0);
        assert_eq!(bucket.tokens, 0);

        // Deduct exactly the available amount
        bucket.return_tokens(4);
        let deducted = bucket.deduct_tokens(4);
        assert_eq!(deducted, 4);
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_deduct_tokens_underflow() {
        // Create a bucket
        let capacity = 10;
        let refill_rate = 2;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Acquire all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // The deduct operation should handle underflow gracefully
        let result = bucket.try_acquire_all(1);
        assert!(result.is_err());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_exact_wait_time_after_time_advance() {
        // Create a new token bucket with mock time
        let capacity = 10;
        let refill_rate = 5;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());
        let bucket_creation_time = time_service.now();

        // Drain all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Advance 2 seconds (last_refill_time becomes creation_time + 2s)
        let mock_time = time_service.into_mock();
        mock_time.advance_secs(2);
        assert!(bucket.try_acquire_all(10).is_ok()); // consume the 10 refilled tokens
        assert_eq!(bucket.tokens, 0);

        // Request 7 tokens (2 more seconds required)
        let wait_until = bucket.try_acquire_all(7).unwrap_err().unwrap();
        assert_eq!(wait_until, bucket_creation_time + Duration::from_secs(4));
    }

    #[test]
    fn test_exact_wait_time_empty_bucket() {
        // Create a new token bucket with mock time
        let capacity = 10;
        let refill_rate = 5;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());
        let creation_time = time_service.now();

        // Drain all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Request 5 tokens (1 second required)
        let wait = bucket.try_acquire_all(5).unwrap_err().unwrap();
        assert_eq!(wait, creation_time + Duration::from_secs(1));

        // Request 6 tokens (2 seconds required)
        let wait = bucket.try_acquire_all(6).unwrap_err().unwrap();
        assert_eq!(wait, creation_time + Duration::from_secs(2));

        // Request 10 tokens  (2 seconds required)
        let wait = bucket.try_acquire_all(10).unwrap_err().unwrap();
        assert_eq!(wait, creation_time + Duration::from_secs(2));
    }

    #[test]
    fn test_exact_wait_time_partial_tokens() {
        // Create a new token bucket with mock time
        let capacity = 10;
        let refill_rate = 5;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());
        let creation_time = time_service.now();

        // Use 7 tokens, leaving 3
        assert!(bucket.try_acquire_all(7).is_ok());
        assert_eq!(bucket.tokens, 3);

        // Request 8 tokens (1 second required)
        let wait = bucket.try_acquire_all(8).unwrap_err().unwrap();
        assert_eq!(wait, creation_time + Duration::from_secs(1));

        // Request 4 tokens (1 second required)
        let wait = bucket.try_acquire_all(4).unwrap_err().unwrap();
        assert_eq!(wait, creation_time + Duration::from_secs(1));
    }

    #[test]
    fn test_new_constructor_fields() {
        // Create a new token bucket
        let capacity = 42;
        let refill_rate = 7;
        let bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Verify that the fields are initialized correctly
        assert_eq!(bucket.tokens, capacity);
        assert_eq!(bucket.capacity, capacity);
        assert_eq!(bucket.refill_rate, refill_rate);
    }

    #[test]
    #[should_panic(expected = "Bucket capacity must be >= refill rate!")]
    fn test_new_with_capacity_less_than_refill_rate() {
        // Verify creating a bucket with capacity < refill_rate fails
        TokenBucket::new(5, 10, TimeService::mock());
    }

    #[test]
    #[should_panic(expected = "Initial tokens must be <= capacity!")]
    fn test_new_with_initial_exceeds_capacity() {
        // Verify creating a bucket with initial_tokens > capacity fails
        TokenBucket::new_with_initial_tokens(100, 10, 101, TimeService::mock());
    }

    #[test]
    fn test_new_with_initial_tokens_zero() {
        // Create a new token bucket with 0 initial tokens
        let capacity = 10;
        let refill_rate = 5;
        let bucket =
            TokenBucket::new_with_initial_tokens(capacity, refill_rate, 0, TimeService::mock());

        // Verify that the bucket is created with 0 tokens and correct capacity and refill rate
        assert_eq!(bucket.tokens, 0);
        assert_eq!(bucket.capacity, capacity);
        assert_eq!(bucket.refill_rate, refill_rate);
    }

    #[test]
    #[should_panic(expected = "Capacity and refill rate must be > 0!")]
    fn test_new_with_zero_capacity() {
        // Verify creating a bucket with zero capacity fails
        TokenBucket::new(0, 10, TimeService::mock());
    }

    #[test]
    #[should_panic(expected = "Capacity and refill rate must be > 0!")]
    fn test_new_with_zero_refill_rate() {
        // Verify creating a bucket with zero refill rate fails
        TokenBucket::new(10, 0, TimeService::mock());
    }

    #[test]
    fn test_refill_after_time_elapsed() {
        // Create a new token bucket with mock time
        let capacity = 100;
        let refill_rate = 10; // 10 tokens per second
        let mock_time = MockTimeService::new();
        let time_service = TimeService::from_mock(mock_time.clone());
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service);

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // No tokens should be available immediately
        assert!(bucket.try_acquire_all(1).is_err());
        assert_eq!(bucket.tokens, 0);

        // Advance time by 1 second (should refill 10 tokens)
        mock_time.advance_secs(1);
        assert!(bucket.try_acquire_all(10).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance time by 5 seconds (should refill 50 tokens)
        mock_time.advance_secs(5);
        assert!(bucket.try_acquire_all(50).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance time by 10 seconds (should refill to full capacity, 100 tokens)
        mock_time.advance_secs(10);
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_refill_called_directly() {
        // Create a new token bucket with mock time
        let capacity = 50;
        let refill_rate = 10;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Drain all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance 3 seconds and call refill() directly
        let mock_time = time_service.into_mock();
        mock_time.advance_secs(3);
        bucket.refill();
        assert_eq!(bucket.tokens, 30); // 3 intervals * 10 tokens/s

        // Calling refill() again without advancing time adds nothing
        bucket.refill();
        assert_eq!(bucket.tokens, 30);
    }

    #[test]
    fn test_refill_does_not_exceed_capacity() {
        // Create a new token bucket with mock time
        let capacity = 20;
        let refill_rate = 5; // 5 tokens per second
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Use some tokens
        assert!(bucket.try_acquire_all(10).is_ok());
        assert_eq!(bucket.tokens, 10);

        // Advance time by 100 seconds (way more than needed to fill)
        let mock_time = time_service.into_mock();
        mock_time.advance_secs(100);

        // Verify that tokens are capped at capacity
        assert!(bucket.try_acquire_all(1).is_ok());
        assert_eq!(bucket.tokens, capacity - 1);
    }

    #[test]
    fn test_refill_drift_prevention() {
        // Create a new token bucket with mock time
        let capacity = 100;
        let refill_rate = 10;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Drain all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Advance 2.7 seconds (2 whole intervals)
        let mock_time = time_service.into_mock();
        mock_time.advance_secs(2);
        mock_time.advance_ms(700);
        assert!(bucket.try_acquire_all(20).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance 0.3 more seconds (1 whole interval)
        mock_time.advance_ms(300);
        assert!(bucket.try_acquire_all(10).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_refill_multiple_small_intervals() {
        // Create a new token bucket with mock time
        let capacity = 50;
        let refill_rate = 5; // 5 tokens per second
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance in small increments and verify refill
        let mock_time = time_service.into_mock();
        for _ in 0..10 {
            mock_time.advance_secs(1);
            assert!(bucket.try_acquire_all(5).is_ok());
            assert_eq!(bucket.tokens, 0);
        }
    }

    #[test]
    fn test_refill_partial_second() {
        // Create a new token bucket with mock time
        let capacity = 100;
        let refill_rate = 10; // 10 tokens per second
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Advance time by 500ms (less than 1 second)
        let mock_time = time_service.into_mock();
        mock_time.advance_ms(500);

        // No refill should occur (refill only happens at 1 second intervals)
        assert!(bucket.try_acquire_all(1).is_err());
        assert_eq!(bucket.tokens, 0);

        // Advance time by another 500ms (total 1 second)
        mock_time.advance_ms(500);

        // Now 10 tokens should be available
        assert!(bucket.try_acquire_all(10).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_refill_with_return_tokens() {
        // Create a new token bucket with mock time
        let capacity = 50;
        let refill_rate = 10; // 10 tokens per second
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Use most tokens
        assert!(bucket.try_acquire_all(40).is_ok());
        assert_eq!(bucket.tokens, 10);

        // Advance time by 2 seconds (should add 20 tokens, but capped at capacity)
        let mock_time = time_service.into_mock();
        mock_time.advance_secs(2);
        assert!(bucket.try_acquire_all(1).is_ok());
        assert_eq!(bucket.tokens, 29);

        // Return some tokens
        bucket.return_tokens(21);
        assert_eq!(bucket.tokens, capacity); // Should be capped at capacity

        // Advance time (should not overflow capacity)
        mock_time.advance_secs(5);
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_return_tokens_zero() {
        // Create a new token bucket
        let capacity = 11;
        let refill_rate = 4;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Use some tokens
        assert!(bucket.try_acquire_all(5).is_ok());
        assert_eq!(bucket.tokens, 6);

        // Return zero tokens
        bucket.return_tokens(0);
        assert_eq!(bucket.tokens, 6);
    }

    #[test]
    fn test_saturating_add_when_returning_tokens() {
        // Create a bucket with some tokens already used
        let capacity = 50;
        let refill_rate = 5;
        let mut bucket =
            TokenBucket::new_with_initial_tokens(capacity, refill_rate, 30, TimeService::mock());

        // Return enough tokens to match capacity
        bucket.return_tokens(20);
        assert_eq!(bucket.tokens, capacity);

        // Return more tokens (should cap at capacity)
        bucket.return_tokens(10);
        assert_eq!(bucket.tokens, capacity);
    }

    #[test]
    fn test_shared_token_bucket() {
        // Create a new shared token bucket
        let capacity = 10;
        let refill_rate = 2;
        let bucket: SharedTokenBucket = Arc::new(Mutex::new(TokenBucket::new(
            capacity,
            refill_rate,
            TimeService::mock(),
        )));

        // Test through the shared bucket interface
        assert!(bucket.lock().try_acquire_all(5).is_ok());
        assert!(bucket.lock().try_acquire_all(5).is_ok());
        assert!(bucket.lock().try_acquire_all(1).is_err());

        // Return tokens
        bucket.lock().return_tokens(3);
        assert!(bucket.lock().try_acquire_all(3).is_ok());
    }

    #[test]
    fn test_time_calculation_edge_cases() {
        // Create a bucket with specific parameters
        let capacity = 100;
        let refill_rate = 7; // Not a divisor of capacity
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Request 50 tokens (needs ceil(50/7) = 8 intervals)
        let result = bucket.try_acquire_all(50);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_some());

        // Request exactly capacity (needs ceil(100/7) = 15 intervals)
        let result = bucket.try_acquire_all(capacity);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_some());
    }

    #[test]
    fn test_time_of_tokens_needed_multiple_intervals() {
        // Create a bucket and use all tokens
        let capacity = 100;
        let refill_rate = 10;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service.clone());

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Request tokens that require multiple refill intervals
        let result = bucket.try_acquire_all(50);
        assert!(result.is_err());

        // Should have a wait time (not None, since 50 <= capacity)
        let wait_time = result.unwrap_err();
        assert!(wait_time.is_some());

        // The wait time should be in the future
        let instant = wait_time.unwrap();
        assert!(instant > time_service.now());
    }

    #[test]
    fn test_token_bucket_basic() {
        // Create a new token bucket
        let capacity = 10;
        let refill_rate = 2;
        let time_service = TimeService::mock();
        let mut bucket = TokenBucket::new(capacity, refill_rate, time_service);

        // We should be able to acquire all initial tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Ensure the bucket is empty
        assert!(bucket.try_acquire_all(1).is_err());

        // Return some tokens
        bucket.return_tokens(5);
        assert_eq!(bucket.tokens, 5);

        // Verify we can acquire them again
        assert!(bucket.try_acquire_all(5).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_token_bucket_capacity_limit() {
        // Create a new token bucket
        let capacity = 10;
        let refill_rate = 2;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Use all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());

        // Return more than capacity
        bucket.return_tokens(capacity + 5);

        // Verify we only have capacity tokens
        assert_eq!(bucket.tokens, capacity);
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert!(bucket.try_acquire_all(1).is_err());
    }

    #[test]
    fn test_try_acquire_all_exceeds_capacity() {
        // Create a new token bucket
        let capacity = 100;
        let refill_rate = 25;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Verify requesting more than capacity fails
        let result = bucket.try_acquire_all(capacity + 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_none());

        // The tokens should remain unchanged
        assert_eq!(bucket.tokens, capacity);
    }

    #[test]
    fn test_try_acquire_all_failure() {
        // Create a new token bucket
        let capacity = 10;
        let refill_rate = 3;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Acquire most tokens
        assert!(bucket.try_acquire_all(8).is_ok());
        assert_eq!(bucket.tokens, 2);

        // Request more tokens than available
        let result = bucket.try_acquire_all(5);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_some()); // Should have wait time

        // The tokens should remain unchanged
        assert_eq!(bucket.tokens, 2);
    }

    #[test]
    fn test_try_acquire_all_success() {
        // Create a new token bucket
        let capacity = 10;
        let refill_rate = 2;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Acquire some tokens
        assert!(bucket.try_acquire_all(5).is_ok());
        assert_eq!(bucket.tokens, 5);

        // Acquire the remaining tokens
        assert!(bucket.try_acquire_all(5).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_try_acquire_all_zero() {
        // Create a new token bucket
        let capacity = 8;
        let refill_rate = 1;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Acquire zero tokens (should succeed)
        assert!(bucket.try_acquire_all(0).is_ok());
        assert_eq!(bucket.tokens, capacity);
    }

    #[test]
    fn test_try_acquire_all_zero_on_empty_bucket() {
        // try_acquire_all(0) must succeed even when the bucket is drained,
        // because the early return fires before refill() is called.
        let capacity = 10;
        let refill_rate = 5;
        let mut bucket = TokenBucket::new(capacity, refill_rate, TimeService::mock());

        // Drain all tokens
        assert!(bucket.try_acquire_all(capacity).is_ok());
        assert_eq!(bucket.tokens, 0);

        // Zero-token request must still succeed
        assert!(bucket.try_acquire_all(0).is_ok());
        assert_eq!(bucket.tokens, 0);
    }

    #[test]
    fn test_with_initial() {
        // Create a new token bucket with initial tokens
        let capacity = 100;
        let refill_rate = 10;
        let initial = 50;
        let bucket = TokenBucket::new_with_initial_tokens(
            capacity,
            refill_rate,
            initial,
            TimeService::mock(),
        );

        // Verify initial state
        assert_eq!(bucket.tokens, 50);
        assert_eq!(bucket.capacity, 100);
        assert_eq!(bucket.refill_rate, 10);
    }
}
