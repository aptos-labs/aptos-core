/// StableSwap invariant math, following the shape of Curve's `StableSwap`
/// contracts.
///
/// `get_D` solves the invariant for the pool's total virtual balance and
/// `get_y` solves it for one coin, both by Newton iteration in u256. How many
/// rounds either takes depends on how far the pool is from balance, which is
/// the data-dependent iteration count this benchmark exists to measure.
///
/// Curve aborts when its 255-round bound is reached, and relies on real pool
/// balances to keep its products inside a u256. Neither is available here: the
/// benchmark harness treats any abort as a failed run. So the loops return
/// their last iterate at the bound, clamp every balance they read to
/// `D_LIMIT` before it enters a product, and bail out on any intermediate
/// large enough that the next product would not fit.
module bench::ss_math {
    use std::vector;

    /// Denominator the amplification coefficient is expressed in.
    const A_PRECISION: u256 = 100;

    /// Common precision every coin balance is scaled up to. Ten places keeps
    /// a scaled balance near 1e21 at the package's largest reserve, which is
    /// six orders of magnitude under `D_LIMIT`.
    const PRECISION_DECIMALS: u8 = 10;

    /// Newton bound, matching Curve's.
    const MAX_ITERS: u64 = 255;

    /// Ceilings the Newton loops stop at. A pool this package can build stays
    /// far below all of them; a degenerate one would otherwise overflow the
    /// next multiplication and abort the whole transaction.
    const D_LIMIT: u256 = 1000000000000000000000000000;
    const DP_LIMIT: u256 = 1000000000000000000000000000000000000000000000;
    const C_LIMIT: u256 = 1000000000000000000000000000000000000000000000;
    const Y_LIMIT: u256 = 10000000000000000000000000000000000000;
    const ANN_LIMIT: u256 = 10000000000;

    /// Synthetic pools `bench_math_only` refuses to run past, so a
    /// caller-supplied count cannot exceed the per-transaction execution
    /// limit, and the largest `A` it will solve against.
    const MAX_SYNTHETIC_POOLS: u64 = 64;
    const MAX_SYNTHETIC_A: u64 = 1000000;

    /// Reference scaled balance the synthetic pools are built around: a
    /// million six-decimal tokens.
    const BASE_XP: u256 = 10000000000000000;

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    /// `bench_math_only` produced an accumulator no reachable input can
    /// produce.
    const E_IMPOSSIBLE_ACCUMULATOR: u64 = 1;

    public fun max_iters(): u64 {
        MAX_ITERS
    }

    public fun a_precision(): u256 {
        A_PRECISION
    }

    /// Multiplier that lifts a balance with `decimals` places to the common
    /// precision.
    public fun rate_for_decimals(decimals: u8): u256 {
        if (decimals >= PRECISION_DECIMALS) {
            1
        } else {
            pow10(PRECISION_DECIMALS - decimals)
        }
    }

    /// Smallest unit of a token with `decimals` places, capped so a
    /// whole-token amount still fits a u64.
    public fun unit_for_decimals(decimals: u8): u64 {
        if (decimals > 12) {
            1000000000000
        } else {
            (pow10(decimals) as u64)
        }
    }

    fun pow10(e: u8): u256 {
        let r = 1u256;
        let i = 0;
        while (i < e) {
            r = r * 10;
            i = i + 1;
        };
        r
    }

    /// Balances lifted to the common precision. A zero balance becomes one, so
    /// the Newton loops cannot divide by zero on an empty pool.
    public fun xp_mem(
        balances: &vector<u64>, rates: &vector<u256>
    ): vector<u256> {
        let n = vector::length(balances);
        let xp = vector::empty<u256>();
        let i = 0;
        while (i < n) {
            let x = (*vector::borrow(balances, i) as u256)
                * *vector::borrow(rates, i);
            if (x == 0) {
                x = 1;
            };
            vector::push_back(&mut xp, x);
            i = i + 1;
        };
        xp
    }

    public fun sum(xp: &vector<u256>): u256 {
        let total = 0u256;
        let i = 0;
        let n = vector::length(xp);
        while (i < n) {
            total = total + clamp(*vector::borrow(xp, i), D_LIMIT);
            if (total > D_LIMIT) {
                return D_LIMIT
            };
            i = i + 1;
        };
        total
    }

    public fun get_D(xp: &vector<u256>, amp: u256): u256 {
        let (d, _) = get_D_with_iters(xp, amp);
        d
    }

    /// `get_D` plus the rounds it took. A pool's distance from balance is what
    /// sets that count, so a test has to be able to observe it.
    public fun get_D_with_iters(xp: &vector<u256>, amp: u256): (u256, u64) {
        let n_coins = vector::length(xp);
        let s = sum(xp);
        if (s == 0 || n_coins == 0) {
            return (0, 0)
        };
        let n = (n_coins as u256);
        let ann = clamp(amp * n, ANN_LIMIT);
        // Below one unit of A the update's denominator underflows.
        if (ann < A_PRECISION) {
            ann = A_PRECISION;
        };
        let d = s;
        let i = 0;
        while (i < MAX_ITERS) {
            if (d > D_LIMIT) {
                return (d, i)
            };
            let d_p = d;
            let k = 0;
            while (k < n_coins) {
                if (d_p > DP_LIMIT) {
                    return (d, i + 1)
                };
                let x = clamp(*vector::borrow(xp, k), D_LIMIT);
                if (x == 0) {
                    x = 1;
                };
                d_p = d_p * d / (x * n);
                k = k + 1;
            };
            if (d_p > DP_LIMIT) {
                return (d, i + 1)
            };
            let d_prev = d;
            let denominator = (ann - A_PRECISION) * d / A_PRECISION
                + (n + 1) * d_p;
            i = i + 1;
            if (denominator == 0) {
                return (d, i)
            };
            d = (ann * s / A_PRECISION + d_p * n) * d / denominator;
            if (diff(d, d_prev) <= 1) {
                return (d, i)
            };
        };
        (d, MAX_ITERS)
    }

    /// Balance coin `j` settles at when coin `i` holds `x`, with every other
    /// coin left where `xp` has it.
    public fun get_y(
        i: u64, j: u64, x: u256, xp: &vector<u256>, amp: u256
    ): u256 {
        let (y, _) = get_y_with_iters(i, j, x, xp, amp);
        y
    }

    public fun get_y_with_iters(
        i: u64, j: u64, x: u256, xp: &vector<u256>, amp: u256
    ): (u256, u64) {
        let d = clamp(get_D(xp, amp), D_LIMIT);
        if (d == 0) {
            return (0, 0)
        };
        let (b, c) = coefficients(j, i, clamp(x, D_LIMIT), xp, amp, d);
        newton_y(d, b, c)
    }

    /// Balance coin `j` settles at when the invariant is driven to `d`. This
    /// is the single-coin withdrawal side of `get_y`.
    public fun get_y_D(
        j: u64, xp: &vector<u256>, amp: u256, d: u256
    ): u256 {
        let d = clamp(d, D_LIMIT);
        if (d == 0) {
            return 0
        };
        // No coin is overridden, so the override index is out of range.
        let (b, c) = coefficients(j, vector::length(xp), 0, xp, amp, d);
        let (y, _) = newton_y(d, b, c);
        y
    }

    /// The `b` and `c` of `y^2 + (b - d) y = c`, over every coin but `j`, with
    /// coin `override_i` held at `override_x`.
    fun coefficients(
        j: u64,
        override_i: u64,
        override_x: u256,
        xp: &vector<u256>,
        amp: u256,
        d: u256,
    ): (u256, u256) {
        let n_coins = vector::length(xp);
        if (n_coins == 0) {
            return (d, d)
        };
        let n = (n_coins as u256);
        let ann = clamp(amp * n, ANN_LIMIT);
        if (ann < A_PRECISION) {
            ann = A_PRECISION;
        };
        let c = d;
        let s = 0u256;
        let k = 0;
        while (k < n_coins) {
            if (k != j) {
                let cur = if (k == override_i) {
                    override_x
                } else {
                    clamp(*vector::borrow(xp, k), D_LIMIT)
                };
                if (cur == 0) {
                    cur = 1;
                };
                s = s + cur;
                c = clamp(c, C_LIMIT) * d / (cur * n);
            };
            k = k + 1;
        };
        c = clamp(c, C_LIMIT) * d * A_PRECISION / (ann * n);
        (s + d * A_PRECISION / ann, c)
    }

    /// Solve `y^2 + (b - d) y = c` by Newton from `y = d`.
    fun newton_y(d: u256, b: u256, c: u256): (u256, u64) {
        let y = d;
        let i = 0;
        while (i < MAX_ITERS) {
            if (y > Y_LIMIT) {
                return (y, i)
            };
            let y_prev = y;
            let denominator = 2 * y + b;
            i = i + 1;
            // `b` carries `d / ann` on top of the other balances, so this is
            // positive for every pool the package builds. A degenerate pool
            // must still return rather than abort the transaction.
            if (denominator <= d) {
                return (y, i)
            };
            y = (y * y + c) / (denominator - d);
            if (diff(y, y_prev) <= 1) {
                return (y, i)
            };
        };
        (y, MAX_ITERS)
    }

    fun clamp(v: u256, limit: u256): u256 {
        if (v > limit) { limit } else { v }
    }

    fun diff(a: u256, b: u256): u256 {
        if (a > b) { a - b } else { b - a }
    }

    /// Run the Newton kernel over synthetic balances. Nothing here reads or
    /// writes global state, so the mix can attribute compute to the math
    /// rather than to storage.
    public entry fun bench_math_only(
        _user: &signer, n_pools: u64, a: u64, seed: u64
    ) {
        let pools = if (n_pools > MAX_SYNTHETIC_POOLS) {
            MAX_SYNTHETIC_POOLS
        } else {
            n_pools
        };
        let amp = (if (a > MAX_SYNTHETIC_A) { MAX_SYNTHETIC_A } else { a } as u256)
            * A_PRECISION;
        let x = seed % LCG_MOD;
        let acc = 0u256;
        let p = 0;
        while (p < pools) {
            // Alternating widths keep both pool shapes on the hot path.
            let n_coins = 2 + (p % 2);
            let xp = vector::empty<u256>();
            let k = 0;
            while (k < n_coins) {
                x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
                // A tenth to twice the reference balance, so the pools sit at
                // a spread of distances from balance.
                let balance = BASE_XP * (1 + ((x % 20) as u256)) / 10;
                vector::push_back(&mut xp, balance);
                k = k + 1;
            };
            acc = acc + get_D(&xp, amp);
            acc = acc + get_y(0, 1, *vector::borrow(&xp, 0) * 11 / 10, &xp, amp);
            p = p + 1;
        };
        // The accumulator has to be observed or the loop is dead code. Every
        // term is either zero or far larger than one.
        assert!(acc != 1, E_IMPOSSIBLE_ACCUMULATOR);
    }
}
