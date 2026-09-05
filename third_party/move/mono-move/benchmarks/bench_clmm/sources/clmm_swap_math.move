/// One step of a swap: how far the price moves inside a single liquidity
/// range, and what that costs.
///
/// A step never crosses `sqrt_target`. The pool loop calls this once per tick
/// range and handles the crossing itself.
module bench::clmm_swap_math {
    use bench::clmm_full_math;
    use bench::clmm_liquidity_math;

    /// Price moved the wrong way for the requested direction.
    const EINVALID_DIRECTION: u64 = 1;
    /// Fee rate at or above 100%.
    const EINVALID_FEE_RATE: u64 = 2;
    /// Intermediate product left `u256`; liquidity is too large for this price.
    const EPRICE_OVERFLOW: u64 = 3;
    /// Output would drive the price to zero or below.
    const EPRICE_UNDERFLOW: u64 = 4;
    /// A token amount left `u64`.
    const EAMOUNT_OVERFLOW: u64 = 5;

    /// Fee rates are millionths, so 3000 is 0.30%.
    const FEE_RATE_DENOMINATOR: u64 = 1000000;

    const MAX_U64: u256 = 18446744073709551615;
    const MAX_U128: u256 = 340282366920938463463374607431768211455;

    public fun fee_rate_denominator(): u64 {
        FEE_RATE_DENOMINATOR
    }

    /// Advance the price toward `sqrt_target` for at most `amount`.
    ///
    /// Returns `(amount_in, amount_out, next_sqrt_price, fee_amount)`, where
    /// `amount_in` excludes the fee. A trader pays `amount_in + fee_amount`.
    public fun compute_swap_step(
        sqrt_current: u128,
        sqrt_target: u128,
        liquidity: u128,
        amount: u64,
        fee_rate: u64,
        a_to_b: bool,
        exact_in: bool
    ): (u64, u64, u128, u64) {
        assert!(fee_rate < FEE_RATE_DENOMINATOR, EINVALID_FEE_RATE);
        if (a_to_b) {
            assert!(sqrt_current >= sqrt_target, EINVALID_DIRECTION);
        } else {
            assert!(sqrt_current <= sqrt_target, EINVALID_DIRECTION);
        };
        // An empty range costs nothing and is skipped whole, which is how the
        // pool loop walks the gap between two initialized ticks.
        if (liquidity == 0) {
            return (0, 0, sqrt_target, 0)
        };
        if (amount == 0) {
            return (0, 0, sqrt_current, 0)
        };

        let denominator = (FEE_RATE_DENOMINATOR as u256);
        let net_rate = denominator - (fee_rate as u256);
        let amount_in;
        let amount_out;
        let fee_amount;
        let next_sqrt_price;

        if (exact_in) {
            // The fee comes off the top, so only this much reaches the curve.
            let budget = clmm_full_math::mul_div((amount as u256), net_rate, denominator);
            let max_in = max_amount_in(sqrt_current, sqrt_target, liquidity, a_to_b);
            if (max_in > budget) {
                amount_in = budget;
                fee_amount = (amount as u256) - budget;
                next_sqrt_price =
                    get_next_sqrt_price_from_input(sqrt_current, liquidity, budget, a_to_b);
            } else {
                amount_in = max_in;
                fee_amount = clmm_full_math::mul_div_rounding_up(amount_in, (fee_rate as u256), net_rate);
                next_sqrt_price = sqrt_target;
            };
            amount_out = max_amount_out(sqrt_current, next_sqrt_price, liquidity, a_to_b);
        } else {
            let max_out = max_amount_out(sqrt_current, sqrt_target, liquidity, a_to_b);
            if (max_out > (amount as u256)) {
                amount_out = (amount as u256);
                next_sqrt_price =
                    get_next_sqrt_price_from_output(sqrt_current, liquidity, amount_out, a_to_b);
            } else {
                amount_out = max_out;
                next_sqrt_price = sqrt_target;
            };
            amount_in = max_amount_in(sqrt_current, next_sqrt_price, liquidity, a_to_b);
            fee_amount = clmm_full_math::mul_div_rounding_up(amount_in, (fee_rate as u256), net_rate);
        };

        assert!(amount_in <= MAX_U64, EAMOUNT_OVERFLOW);
        assert!(amount_out <= MAX_U64, EAMOUNT_OVERFLOW);
        assert!(fee_amount <= MAX_U64, EAMOUNT_OVERFLOW);
        ((amount_in as u64), (amount_out as u64), next_sqrt_price, (fee_amount as u64))
    }

    /// Input needed to move the price from `from` to `to`, rounded up.
    public fun max_amount_in(from: u128, to: u128, liquidity: u128, a_to_b: bool): u256 {
        if (a_to_b) {
            clmm_liquidity_math::get_amount_a_delta(from, to, liquidity, true)
        } else {
            clmm_liquidity_math::get_amount_b_delta(from, to, liquidity, true)
        }
    }

    /// Output produced by moving the price from `from` to `to`, rounded down.
    public fun max_amount_out(from: u128, to: u128, liquidity: u128, a_to_b: bool): u256 {
        if (a_to_b) {
            clmm_liquidity_math::get_amount_b_delta(from, to, liquidity, false)
        } else {
            clmm_liquidity_math::get_amount_a_delta(from, to, liquidity, false)
        }
    }

    public fun get_next_sqrt_price_from_input(
        sqrt_price: u128, liquidity: u128, amount: u256, a_to_b: bool
    ): u128 {
        if (a_to_b) {
            get_next_sqrt_price_a_up(sqrt_price, liquidity, amount, true)
        } else {
            get_next_sqrt_price_b_down(sqrt_price, liquidity, amount, true)
        }
    }

    public fun get_next_sqrt_price_from_output(
        sqrt_price: u128, liquidity: u128, amount: u256, a_to_b: bool
    ): u128 {
        if (a_to_b) {
            get_next_sqrt_price_b_down(sqrt_price, liquidity, amount, false)
        } else {
            get_next_sqrt_price_a_up(sqrt_price, liquidity, amount, false)
        }
    }

    /// Next price after `amount` of token A enters (`add`) or leaves the range.
    ///
    /// From `L / sqrt(p') = L / sqrt(p) + dA`, so
    /// `sqrt(p') = L * sqrt(p) / (L + dA * sqrt(p))`. Rounding up keeps the
    /// price from overshooting, which would let a trader take too much out.
    public fun get_next_sqrt_price_a_up(
        sqrt_price: u128, liquidity: u128, amount: u256, add: bool
    ): u128 {
        if (amount == 0) {
            return sqrt_price
        };
        let numerator_1 = (liquidity as u256) << 96;
        let price = (sqrt_price as u256);
        let product = amount * price;
        let next = if (add) {
            let denominator = numerator_1 + product;
            if (numerator_1 <= clmm_full_math::max_u256() / price) {
                clmm_full_math::mul_div_rounding_up(numerator_1, price, denominator)
            } else {
                // `L * sqrt(p)` would not fit, so divide first and lose a unit
                // of precision instead of the whole result.
                clmm_full_math::div_rounding_up(numerator_1, numerator_1 / price + amount)
            }
        } else {
            assert!(product < numerator_1, EPRICE_UNDERFLOW);
            assert!(numerator_1 <= clmm_full_math::max_u256() / price, EPRICE_OVERFLOW);
            clmm_full_math::mul_div_rounding_up(numerator_1, price, numerator_1 - product)
        };
        assert!(next <= MAX_U128, EPRICE_OVERFLOW);
        (next as u128)
    }

    /// Next price after `amount` of token B enters (`add`) or leaves the range.
    ///
    /// From `L * sqrt(p') = L * sqrt(p) + dB`, so `sqrt(p') = sqrt(p) + dB / L`.
    /// Rounding is toward the current price for the same reason as above.
    public fun get_next_sqrt_price_b_down(
        sqrt_price: u128, liquidity: u128, amount: u256, add: bool
    ): u128 {
        let price = (sqrt_price as u256);
        let next = if (add) {
            price + (amount << 96) / (liquidity as u256)
        } else {
            let quotient = clmm_full_math::div_rounding_up(amount << 96, (liquidity as u256));
            assert!(price > quotient, EPRICE_UNDERFLOW);
            price - quotient
        };
        assert!(next <= MAX_U128, EPRICE_OVERFLOW);
        (next as u128)
    }

    //
    // Tests.
    //

    #[test_only]
    use bench::clmm_tick_math;

    #[test_only]
    const TEST_LIQUIDITY: u128 = 1000000000000000;

    #[test_only]
    const TEST_FEE: u64 = 3000;

    #[test]
    fun test_zero_liquidity_skips_the_range() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-600);
        let (ain, aout, next, fee) = compute_swap_step(from, to, 0, 1000000, TEST_FEE, true, true);
        assert!(ain == 0 && aout == 0 && fee == 0, 0);
        assert!(next == to, 0);
    }

    #[test]
    fun test_zero_amount_leaves_the_price_alone() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-600);
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, 0, TEST_FEE, true, true);
        assert!(ain == 0 && aout == 0 && fee == 0, 0);
        assert!(next == from, 0);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_DIRECTION, location = Self)]
    fun test_a_to_b_rejects_a_rising_target() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(600);
        compute_swap_step(from, to, TEST_LIQUIDITY, 1000, TEST_FEE, true, true);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_DIRECTION, location = Self)]
    fun test_b_to_a_rejects_a_falling_target() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-600);
        compute_swap_step(from, to, TEST_LIQUIDITY, 1000, TEST_FEE, false, true);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FEE_RATE, location = Self)]
    fun test_fee_rate_must_be_below_one() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-600);
        compute_swap_step(from, to, TEST_LIQUIDITY, 1000, FEE_RATE_DENOMINATOR, true, true);
    }

    // An amount too small to reach the target is consumed to the last unit:
    // the fee is exactly the part that did not reach the curve.
    #[test]
    fun test_exact_in_partial_fill_consumes_everything() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60000);
        let amount = 1000000000u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, true, true);
        assert!((ain as u256) + (fee as u256) == (amount as u256), 0);
        assert!(next < from && next > to, 0);
        assert!(aout > 0, 0);
    }

    #[test]
    fun test_exact_in_partial_fill_consumes_everything_upward() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(60000);
        let amount = 1000000000u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, false, true);
        assert!((ain as u256) + (fee as u256) == (amount as u256), 0);
        assert!(next > from && next < to, 0);
        assert!(aout > 0, 0);
    }

    // An amount more than enough stops at the target and charges only for what
    // the curve took.
    #[test]
    fun test_exact_in_full_step_stops_at_the_target() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60);
        let amount = 18446744073709551615u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, true, true);
        assert!(next == to, 0);
        assert!((ain as u256) + (fee as u256) <= (amount as u256), 0);
        assert!(ain > 0 && aout > 0 && fee > 0, 0);
    }

    #[test]
    fun test_exact_in_full_step_stops_at_the_target_upward() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(60);
        let amount = 18446744073709551615u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, false, true);
        assert!(next == to, 0);
        assert!((ain as u256) + (fee as u256) <= (amount as u256), 0);
        assert!(ain > 0 && aout > 0 && fee > 0, 0);
    }

    // The fee is the stated fraction of the gross input, so netting it back out
    // returns `amount_in`.
    #[test]
    fun test_fee_is_the_stated_fraction_of_gross_input() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60);
        let (ain, _, _, fee) = compute_swap_step(
            from, to, TEST_LIQUIDITY, 18446744073709551615u64, TEST_FEE, true, true
        );
        let gross = (ain as u256) + (fee as u256);
        let net = clmm_full_math::mul_div(
            gross, ((FEE_RATE_DENOMINATOR - TEST_FEE) as u256), (FEE_RATE_DENOMINATOR as u256)
        );
        assert!(net <= (ain as u256) && (ain as u256) <= net + 1, 0);
    }

    #[test]
    fun test_exact_out_partial_fill_delivers_the_request() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60000);
        let amount = 1000000000u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, true, false);
        assert!(aout == amount, 0);
        assert!(next < from && next > to, 0);
        assert!(ain > 0 && fee > 0, 0);
        let gross = (ain as u256) + (fee as u256);
        let net = clmm_full_math::mul_div(
            gross, ((FEE_RATE_DENOMINATOR - TEST_FEE) as u256), (FEE_RATE_DENOMINATOR as u256)
        );
        assert!(net <= (ain as u256) && (ain as u256) <= net + 1, 0);
    }

    #[test]
    fun test_exact_out_partial_fill_delivers_the_request_upward() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(60000);
        let amount = 1000000000u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, false, false);
        assert!(aout == amount, 0);
        assert!(next > from && next < to, 0);
        assert!(ain > 0 && fee > 0, 0);
        let gross = (ain as u256) + (fee as u256);
        let net = clmm_full_math::mul_div(
            gross, ((FEE_RATE_DENOMINATOR - TEST_FEE) as u256), (FEE_RATE_DENOMINATOR as u256)
        );
        assert!(net <= (ain as u256) && (ain as u256) <= net + 1, 0);
    }

    // Asking for more than the range holds caps at the target and delivers
    // less than requested.
    #[test]
    fun test_exact_out_capped_at_the_target() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60);
        let amount = 18446744073709551615u64;
        let (ain, aout, next, fee) =
            compute_swap_step(from, to, TEST_LIQUIDITY, amount, TEST_FEE, true, false);
        assert!(next == to, 0);
        assert!((aout as u256) < (amount as u256), 0);
        assert!(ain > 0 && fee > 0, 0);
    }

    // A step never passes the target, in either direction and either mode.
    #[test]
    fun test_step_never_passes_the_target() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let down = clmm_tick_math::get_sqrt_price_at_tick(-600);
        let up = clmm_tick_math::get_sqrt_price_at_tick(600);
        let amounts = vector[1u64, 1000, 1000000, 1000000000, 18446744073709551615];
        let i = 0;
        while (i < 5) {
            let amount = *std::vector::borrow(&amounts, i);
            let (_, _, next, _) =
                compute_swap_step(from, down, TEST_LIQUIDITY, amount, TEST_FEE, true, true);
            assert!(next >= down && next <= from, 0);
            let (_, _, next, _) =
                compute_swap_step(from, down, TEST_LIQUIDITY, amount, TEST_FEE, true, false);
            assert!(next >= down && next <= from, 0);
            let (_, _, next, _) =
                compute_swap_step(from, up, TEST_LIQUIDITY, amount, TEST_FEE, false, true);
            assert!(next <= up && next >= from, 0);
            let (_, _, next, _) =
                compute_swap_step(from, up, TEST_LIQUIDITY, amount, TEST_FEE, false, false);
            assert!(next <= up && next >= from, 0);
            i = i + 1;
        };
    }

    // A zero fee rate charges nothing, and exact-in then spends the whole
    // budget on the curve.
    #[test]
    fun test_zero_fee_charges_nothing() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60000);
        let amount = 1000000000u64;
        let (ain, _, _, fee) = compute_swap_step(from, to, TEST_LIQUIDITY, amount, 0, true, true);
        assert!(fee == 0, 0);
        assert!(ain == amount, 0);
    }

    // Selling into the pool and buying the same amount back out cost the same
    // input, since both land on the same price.
    #[test]
    fun test_exact_in_and_exact_out_agree_on_the_same_step() {
        let from = clmm_tick_math::get_sqrt_price_at_tick(0);
        let to = clmm_tick_math::get_sqrt_price_at_tick(-60000);
        let (ain, aout, next, _) =
            compute_swap_step(from, to, TEST_LIQUIDITY, 1000000000, 0, true, true);
        let (ain2, aout2, next2, _) =
            compute_swap_step(from, to, TEST_LIQUIDITY, aout, 0, true, false);
        assert!(aout2 == aout, 0);
        // Exact-out rounds the input up to reach the same output, so it can ask
        // for a unit more than exact-in consumed.
        assert!(ain2 <= ain && ain2 + 2 >= ain, 0);
        assert!(next2 >= next, 0);
    }

    #[test]
    fun test_next_price_from_input_moves_the_right_way() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        let down = get_next_sqrt_price_from_input(p, TEST_LIQUIDITY, 1000000000, true);
        assert!(down < p, 0);
        let up = get_next_sqrt_price_from_input(p, TEST_LIQUIDITY, 1000000000, false);
        assert!(up > p, 0);
    }

    #[test]
    fun test_next_price_from_output_moves_the_right_way() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        let down = get_next_sqrt_price_from_output(p, TEST_LIQUIDITY, 1000000000, true);
        assert!(down < p, 0);
        let up = get_next_sqrt_price_from_output(p, TEST_LIQUIDITY, 1000000000, false);
        assert!(up > p, 0);
    }

    #[test]
    fun test_next_price_is_unchanged_by_a_zero_amount() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        assert!(get_next_sqrt_price_a_up(p, TEST_LIQUIDITY, 0, true) == p, 0);
        assert!(get_next_sqrt_price_a_up(p, TEST_LIQUIDITY, 0, false) == p, 0);
        assert!(get_next_sqrt_price_b_down(p, TEST_LIQUIDITY, 0, true) == p, 0);
        assert!(get_next_sqrt_price_b_down(p, TEST_LIQUIDITY, 0, false) == p, 0);
    }

    #[test]
    #[expected_failure(abort_code = EPRICE_UNDERFLOW, location = Self)]
    fun test_taking_all_of_token_b_underflows() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        get_next_sqrt_price_b_down(p, TEST_LIQUIDITY, 1u256 << 100, false);
    }

    #[test]
    #[expected_failure(abort_code = EPRICE_UNDERFLOW, location = Self)]
    fun test_taking_all_of_token_a_underflows() {
        let p = clmm_tick_math::get_sqrt_price_at_tick(0);
        get_next_sqrt_price_a_up(p, TEST_LIQUIDITY, 1u256 << 100, false);
    }
}
