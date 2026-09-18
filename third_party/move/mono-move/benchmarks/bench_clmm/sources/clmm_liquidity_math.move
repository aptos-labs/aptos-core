/// Converting between a liquidity amount and the token amounts it represents
/// over a price range.
///
/// The two identities are `amount_a = L * (1/sqrt(lo) - 1/sqrt(hi))` and
/// `amount_b = L * (sqrt(hi) - sqrt(lo))`, both from the Uniswap V3 whitepaper.
module bench::clmm_liquidity_math {
    use bench::clmm_full_math;

    /// Removing more liquidity than is there.
    const ELIQUIDITY_UNDERFLOW: u64 = 1;
    /// A price of zero has no reciprocal.
    const EZERO_PRICE: u64 = 2;

    const Q96: u256 = 79228162514264337593543950336;

    /// Apply a signed change to a liquidity total.
    public fun add_delta(liquidity: u128, delta: i128): u128 {
        if (delta < 0) {
            let magnitude = (-delta) as u128;
            assert!(liquidity >= magnitude, ELIQUIDITY_UNDERFLOW);
            liquidity - magnitude
        } else {
            liquidity + (delta as u128)
        }
    }

    /// Token A held by `liquidity` between the two prices.
    ///
    /// Rounding up is for amounts owed to the pool and rounding down for
    /// amounts paid out, so the pool never gives away a rounding unit.
    public fun get_amount_a_delta(
        sqrt_price_0: u128, sqrt_price_1: u128, liquidity: u128, round_up: bool
    ): u256 {
        let (lo, hi) = sort(sqrt_price_0, sqrt_price_1);
        if (liquidity == 0 || lo == hi) {
            return 0
        };
        assert!(lo > 0, EZERO_PRICE);
        let numerator_1 = (liquidity as u256) << 96;
        let numerator_2 = (hi as u256) - (lo as u256);
        if (round_up) {
            clmm_full_math::div_rounding_up(
                clmm_full_math::mul_div_rounding_up(numerator_1, numerator_2, (hi as u256)),
                (lo as u256)
            )
        } else {
            clmm_full_math::mul_div(numerator_1, numerator_2, (hi as u256)) / (lo as u256)
        }
    }

    /// Token B held by `liquidity` between the two prices.
    public fun get_amount_b_delta(
        sqrt_price_0: u128, sqrt_price_1: u128, liquidity: u128, round_up: bool
    ): u256 {
        let (lo, hi) = sort(sqrt_price_0, sqrt_price_1);
        if (liquidity == 0 || lo == hi) {
            return 0
        };
        let difference = (hi as u256) - (lo as u256);
        if (round_up) {
            clmm_full_math::mul_div_rounding_up((liquidity as u256), difference, Q96)
        } else {
            clmm_full_math::mul_div((liquidity as u256), difference, Q96)
        }
    }

    fun sort(a: u128, b: u128): (u128, u128) {
        if (a > b) { (b, a) } else { (a, b) }
    }

    //
    // Tests.
    //

    #[test_only]
    use bench::clmm_tick_math;

    #[test]
    fun test_add_delta() {
        assert!(add_delta(0, 0i128) == 0, 0);
        assert!(add_delta(100, 50i128) == 150, 0);
        assert!(add_delta(100, -50i128) == 50, 0);
        assert!(add_delta(100, -100i128) == 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = ELIQUIDITY_UNDERFLOW, location = Self)]
    fun test_add_delta_underflows() {
        add_delta(100, -101i128);
    }

    #[test]
    fun test_deltas_are_zero_on_a_degenerate_range() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        assert!(get_amount_a_delta(p, p, 1000000, true) == 0, 0);
        assert!(get_amount_b_delta(p, p, 1000000, true) == 0, 0);
        let q = clmm_tick_math::get_sqrt_price_at_tick(60);
        assert!(get_amount_a_delta(p, q, 0, true) == 0, 0);
        assert!(get_amount_b_delta(p, q, 0, true) == 0, 0);
    }

    // The price arguments commute: the function sorts them itself.
    #[test]
    fun test_deltas_are_symmetric_in_their_prices() {
        let lo = clmm_tick_math::get_sqrt_price_at_tick(-600);
        let hi = clmm_tick_math::get_sqrt_price_at_tick(600);
        let l = 1000000000000u128;
        assert!(get_amount_a_delta(lo, hi, l, true) == get_amount_a_delta(hi, lo, l, true), 0);
        assert!(get_amount_b_delta(lo, hi, l, false) == get_amount_b_delta(hi, lo, l, false), 0);
    }

    #[test]
    fun test_rounding_up_is_never_below_rounding_down() {
        let lo = clmm_tick_math::get_sqrt_price_at_tick(-887);
        let hi = clmm_tick_math::get_sqrt_price_at_tick(1013);
        let l = 123456789012345u128;
        let a_up = get_amount_a_delta(lo, hi, l, true);
        let a_down = get_amount_a_delta(lo, hi, l, false);
        assert!(a_up >= a_down, 0);
        assert!(a_up <= a_down + 1, 0);
        let b_up = get_amount_b_delta(lo, hi, l, true);
        let b_down = get_amount_b_delta(lo, hi, l, false);
        assert!(b_up >= b_down, 0);
        assert!(b_up <= b_down + 1, 0);
    }

    // At tick 0 the price is 1, so a symmetric range holds the same amount of
    // each token. The two identities are mirror images there.
    #[test]
    fun test_symmetric_range_holds_both_tokens_alike() {
        let lo = clmm_tick_math::get_sqrt_price_at_tick(-1000);
        let mid = clmm_tick_math::get_sqrt_price_at_tick(0);
        let hi = clmm_tick_math::get_sqrt_price_at_tick(1000);
        let l = 1000000000000000u128;
        let a = get_amount_a_delta(mid, hi, l, false);
        let b = get_amount_b_delta(lo, mid, l, false);
        let difference = if (a > b) { a - b } else { b - a };
        // Within a part in a million of each other, the rest being the price
        // asymmetry of 1.0001^1000 against 1.0001^-1000.
        assert!(difference * 1000000 < a, 0);
    }

    // Doubling liquidity doubles both amounts, up to the rounding unit.
    #[test]
    fun test_amounts_are_linear_in_liquidity() {
        let lo = clmm_tick_math::get_sqrt_price_at_tick(120);
        let hi = clmm_tick_math::get_sqrt_price_at_tick(3000);
        let a1 = get_amount_a_delta(lo, hi, 500000000, false);
        let a2 = get_amount_a_delta(lo, hi, 1000000000, false);
        assert!(a2 >= 2 * a1 && a2 <= 2 * a1 + 2, 0);
        let b1 = get_amount_b_delta(lo, hi, 500000000, false);
        let b2 = get_amount_b_delta(lo, hi, 1000000000, false);
        assert!(b2 >= 2 * b1 && b2 <= 2 * b1 + 2, 0);
    }

    // A wider range holds more of both tokens at fixed liquidity.
    #[test]
    fun test_wider_range_holds_more() {
        let mid = clmm_tick_math::get_sqrt_price_at_tick(0);
        let near = clmm_tick_math::get_sqrt_price_at_tick(600);
        let far = clmm_tick_math::get_sqrt_price_at_tick(6000);
        let l = 1000000000000u128;
        assert!(get_amount_a_delta(mid, far, l, false) > get_amount_a_delta(mid, near, l, false), 0);
        assert!(get_amount_b_delta(mid, far, l, false) > get_amount_b_delta(mid, near, l, false), 0);
    }
}
