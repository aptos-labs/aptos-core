/// Pricing kernels shared by the four pool backends.
///
/// Everything wider than a `u64` product runs in `u256`. An arithmetic
/// overflow is an abort, and the aggregator's mix transactions are not allowed
/// to abort, so the extra width buys the guarantee outright rather than
/// through a case analysis on reserve sizes.
module bench::dexr_math {
    /// Basis point denominator.
    const BPS: u64 = 10000;

    const U64_MAX: u64 = 18446744073709551615;

    /// Largest reserve a pool may hold. Bounding the input side keeps every
    /// reserve addition inside `u64` however long a run goes on.
    const MAX_RESERVE: u64 = 1_000_000_000_000_000_000;

    /// A hop may move at most this fraction of the reserve it trades into,
    /// expressed as a divisor. Five percent leaves the pool priced sanely for
    /// the next hop without any minimum-output check.
    const MAX_INPUT_DIVISOR: u64 = 20;

    /// Newton iterations for the stableswap invariant. Both loops return their
    /// last iterate instead of asserting convergence.
    const NEWTON_ITERS: u64 = 32;

    public fun bps(): u64 {
        BPS
    }

    /// How much a reserve can still grow before it reaches [`MAX_RESERVE`].
    public fun headroom(reserve: u64): u64 {
        if (reserve >= MAX_RESERVE) { 0 } else { MAX_RESERVE - reserve }
    }

    public fun min(a: u64, b: u64): u64 {
        if (a < b) { a } else { b }
    }

    /// `a * b / c`, zero when `c` is zero and saturating at `u64::MAX`.
    public fun mul_div(a: u64, b: u64, c: u64): u64 {
        if (c == 0) {
            return 0
        };
        let result = (a as u256) * (b as u256) / (c as u256);
        if (result > (U64_MAX as u256)) { U64_MAX } else { (result as u64) }
    }

    /// The largest input this pool will accept: the caller's amount, capped at
    /// a twentieth of the reserve it trades into and at the headroom left
    /// under [`MAX_RESERVE`].
    public fun clamp_in(amount_in: u64, reserve_in: u64): u64 {
        if (reserve_in < MAX_INPUT_DIVISOR || reserve_in >= MAX_RESERVE) {
            return 0
        };
        min(min(amount_in, reserve_in / MAX_INPUT_DIVISOR), MAX_RESERVE - reserve_in)
    }

    /// Constant product output, net of the fee taken off the input.
    public fun cpmm_out(
        amount_in: u64, reserve_in: u64, reserve_out: u64, fee_bps: u64
    ): u64 {
        if (amount_in == 0 || reserve_in == 0 || reserve_out == 0) {
            return 0
        };
        let net = net_of_fee(amount_in, fee_bps);
        let out = net * (reserve_out as u256) / ((reserve_in as u256) + net);
        if (out >= (reserve_out as u256)) { reserve_out - 1 } else { (out as u64) }
    }

    /// Two-coin stableswap output at amplification `amp`, net of the fee.
    public fun stable_out(
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        amp: u64,
        fee_bps: u64,
    ): u64 {
        if (amount_in == 0 || reserve_in == 0 || reserve_out == 0) {
            return 0
        };
        let ann = amp_n_n(amp);
        let x = (reserve_in as u256);
        let y = (reserve_out as u256);
        let d = invariant_d(x, y, ann);
        if (d == 0) {
            return 0
        };
        let y_new = solve_y(x + net_of_fee(amount_in, fee_bps), d, ann);
        if (y_new >= y) {
            return 0
        };
        let out = y - y_new;
        if (out >= (reserve_out as u256)) { reserve_out - 1 } else { (out as u64) }
    }

    /// The stableswap invariant `D` over two balances. Exposed so a test can
    /// pin it against a hand-computed value.
    public fun invariant_d(x: u256, y: u256, ann: u256): u256 {
        if (x == 0 || y == 0) {
            return 0
        };
        let sum = x + y;
        let d = sum;
        let i = 0;
        while (i < NEWTON_ITERS) {
            let d_product = d * d * d / (4 * x * y);
            let denominator = (ann - 1) * d + 3 * d_product;
            if (denominator == 0) {
                break
            };
            let previous = d;
            d = (ann * sum + 2 * d_product) * d / denominator;
            if (converged(d, previous)) {
                break
            };
            i = i + 1;
        };
        d
    }

    /// Amplification times `n^n` for two coins, floored at one so the
    /// invariant's `ann - 1` term cannot underflow.
    public fun amp_n_n(amp: u64): u256 {
        let amp = if (amp == 0) { 1 } else { amp };
        (amp as u256) * 4
    }

    /// The balance of the other coin that holds `d` once this side is
    /// `x_new`. Exposed for the same reason as [`invariant_d`].
    public fun solve_y(x_new: u256, d: u256, ann: u256): u256 {
        if (x_new == 0 || ann == 0) {
            return 0
        };
        let c = d * d * d / (4 * x_new * ann);
        let b = x_new + d / ann;
        let y = d;
        let i = 0;
        while (i < NEWTON_ITERS) {
            let denominator = 2 * y + b;
            if (denominator <= d) {
                break
            };
            let previous = y;
            y = (y * y + c) / (denominator - d);
            if (converged(y, previous)) {
                break
            };
            i = i + 1;
        };
        y
    }

    fun converged(current: u256, previous: u256): bool {
        if (current > previous) {
            current - previous <= 1
        } else {
            previous - current <= 1
        }
    }

    /// The part of `amount` that reaches the curve, with a fee wider than the
    /// denominator pinned one basis point below it.
    fun net_of_fee(amount: u64, fee_bps: u64): u256 {
        let fee_bps = if (fee_bps >= BPS) { BPS - 1 } else { fee_bps };
        (amount as u256) * ((BPS - fee_bps) as u256) / (BPS as u256)
    }
}
