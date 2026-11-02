/// Amplification coefficients and their ramps, following Curve's
/// `ramp_A` / `_A` pair.
///
/// A ramp moves `A` linearly between two wall-clock instants. A harness that
/// holds the block timestamp still leaves the interpolation on its starting
/// value, so what a ramp is worth there is the write rather than the motion:
/// it is the only flow that updates an entry every swap on the pool reads.
module bench::ss_amp {
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::timestamp;

    friend bench::ss_pool;

    /// Only the package address configures a ramp through the admin entry.
    const E_NOT_BENCH: u64 = 1;

    /// Denominator `A` is stored in, matching `ss_math`.
    const A_PRECISION: u64 = 100;

    /// Bounds Curve keeps `A` inside. An unpermissioned ramp clamps into them,
    /// so no caller can push a pool into a degenerate invariant.
    const MIN_A: u64 = 1;
    const MAX_A: u64 = 1000000;

    /// Bounds on how long a ramp may take, so a zero duration cannot make the
    /// interpolation divide by zero and a huge one cannot freeze `A`.
    const MIN_RAMP_SECS: u64 = 1;
    const MAX_RAMP_SECS: u64 = 31536000;

    /// Amplification a pool that was never registered runs at.
    const DEFAULT_A: u64 = 100;

    struct Ramp has store, copy, drop {
        /// Both endpoints are in `A_PRECISION` units.
        a0: u64,
        a1: u64,
        t0: u64,
        t1: u64,
    }

    struct Ramps has key {
        ramps: Table<u64, Ramp>,
    }

    public(friend) fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Ramps>(@bench)) {
            move_to(admin, Ramps { ramps: table::new() });
        }
    }

    /// Start `pool_id` at a flat `a`, replacing any ramp it already had.
    public(friend) fun register(pool_id: u64, a: u64) acquires Ramps {
        if (!exists<Ramps>(@bench)) {
            return
        };
        let now = timestamp::now_seconds();
        let scaled = clamp_a(a) * A_PRECISION;
        let ramp = Ramp { a0: scaled, a1: scaled, t0: now, t1: now };
        let ramps = &mut borrow_global_mut<Ramps>(@bench).ramps;
        if (table::contains(ramps, pool_id)) {
            *table::borrow_mut(ramps, pool_id) = ramp;
        } else {
            table::add(ramps, pool_id, ramp);
        }
    }

    /// Current amplification in `A_PRECISION` units.
    public fun amp(pool_id: u64): u256 acquires Ramps {
        (amp_u64(pool_id) as u256)
    }

    public fun amp_u64(pool_id: u64): u64 acquires Ramps {
        if (!exists<Ramps>(@bench)) {
            return DEFAULT_A * A_PRECISION
        };
        let ramps = &borrow_global<Ramps>(@bench).ramps;
        if (!table::contains(ramps, pool_id)) {
            return DEFAULT_A * A_PRECISION
        };
        interpolate(table::borrow(ramps, pool_id), timestamp::now_seconds())
    }

    /// Admin ramp, used while the publisher is still configuring the package.
    public entry fun set_ramp(
        admin: &signer, pool_id: u64, future_a: u64, duration_secs: u64
    ) acquires Ramps {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        ramp_to(pool_id, future_a, duration_secs);
    }

    /// Unpermissioned ramp. The mix reaches this from a benchmark account,
    /// which never holds the publisher's signer.
    public entry fun bench_ramp(
        _user: &signer, pool_id: u64, future_a: u64, duration_secs: u64
    ) acquires Ramps {
        ramp_to(pool_id, future_a, duration_secs);
    }

    fun ramp_to(pool_id: u64, future_a: u64, duration_secs: u64) acquires Ramps {
        if (!exists<Ramps>(@bench)) {
            return
        };
        let now = timestamp::now_seconds();
        let ramps = &mut borrow_global_mut<Ramps>(@bench).ramps;
        if (!table::contains(ramps, pool_id)) {
            return
        };
        let ramp = table::borrow_mut(ramps, pool_id);
        let from = interpolate(ramp, now);
        ramp.a0 = from;
        ramp.a1 = clamp_a(future_a) * A_PRECISION;
        ramp.t0 = now;
        ramp.t1 = now + clamp_duration(duration_secs);
    }

    fun interpolate(ramp: &Ramp, now: u64): u64 {
        if (ramp.t1 <= ramp.t0 || now >= ramp.t1) {
            return ramp.a1
        };
        if (now <= ramp.t0) {
            return ramp.a0
        };
        let elapsed = now - ramp.t0;
        let span = ramp.t1 - ramp.t0;
        if (ramp.a1 > ramp.a0) {
            ramp.a0 + (ramp.a1 - ramp.a0) * elapsed / span
        } else {
            ramp.a0 - (ramp.a0 - ramp.a1) * elapsed / span
        }
    }

    fun clamp_a(a: u64): u64 {
        if (a < MIN_A) { MIN_A } else if (a > MAX_A) { MAX_A } else { a }
    }

    fun clamp_duration(secs: u64): u64 {
        if (secs < MIN_RAMP_SECS) {
            MIN_RAMP_SECS
        } else if (secs > MAX_RAMP_SECS) {
            MAX_RAMP_SECS
        } else {
            secs
        }
    }

    #[test_only]
    public fun initialize_for_test(admin: &signer) {
        initialize(admin);
    }

    #[test_only]
    public fun register_for_test(pool_id: u64, a: u64) acquires Ramps {
        register(pool_id, a);
    }
}
