spec aptos_framework::rate_limiter {
    spec initialize(capacity: u64, refill_interval: u64): RateLimiter {
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        ensures result == RateLimiter::TokenBucket {
            capacity,
            current_amount: capacity,
            refill_interval,
            last_refill_timestamp: timestamp::spec_now_seconds(),
            fractional_accumulated: 0,
        };
    }

    spec request(limiter: &mut RateLimiter, num_token_requested: u64): bool {
        include RefillAbortsIf;
        let refilled = spec_refill(limiter, timestamp::spec_now_seconds());
        ensures result == (refilled.current_amount >= num_token_requested);
        ensures limiter == if (result) {
            update_field(refilled, current_amount, refilled.current_amount - num_token_requested)
        } else {
            refilled
        };
    }

    spec refill(limiter: &mut RateLimiter) {
        include RefillAbortsIf;
        ensures limiter == spec_refill(old(limiter), timestamp::spec_now_seconds());
    }

    spec schema RefillAbortsIf {
        limiter: RateLimiter;
        let now = timestamp::spec_now_seconds();
        let accumulated = (now - limiter.last_refill_timestamp) * limiter.capacity
            + limiter.fractional_accumulated;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if now < limiter.last_refill_timestamp;
        aborts_if accumulated > MAX_U64;
        aborts_if limiter.refill_interval == 0;
        aborts_if limiter.current_amount + accumulated / limiter.refill_interval > MAX_U64;
    }

    /// The limiter after adding the tokens accrued since its last refill at time `now`, capped at capacity.
    spec fun spec_refill(limiter: RateLimiter, now: u64): RateLimiter {
        let accumulated = (now - limiter.last_refill_timestamp) * limiter.capacity
            + limiter.fractional_accumulated;
        let new_tokens = accumulated / limiter.refill_interval;
        let full = limiter.current_amount + new_tokens >= limiter.capacity;
        RateLimiter::TokenBucket {
            capacity: limiter.capacity,
            current_amount: if (full) { limiter.capacity } else { limiter.current_amount + new_tokens },
            refill_interval: limiter.refill_interval,
            last_refill_timestamp: now,
            fractional_accumulated: if (full) { 0 } else { accumulated % limiter.refill_interval },
        }
    }
}
