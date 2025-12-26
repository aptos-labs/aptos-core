/// This module provides native implementations of computationally intensive
/// position contribution calculations for perpetual trading.
/// The native implementations are significantly faster than Move bytecode
/// equivalents for complex mathematical operations.
module aptos_experimental::fast_native_computations {
    use std::math128;

    /// RATE_SIZE_MULTIPLIER constant used in funding cost calculations
    const RATE_SIZE_MULTIPLIER: u128 = 1_000_000_000_000; // 10^12

    // ============================================================================
    // PUBLIC API
    // ============================================================================

    /// Compute contribution from a position state with a specific price and funding index.
    /// This is the main entry point for computing position metrics.
    ///
    /// Parameters:
    /// - `position_size`: The size of the position in base units
    /// - `position_is_long`: Whether the position is long (true) or short (false)
    /// - `entry_px_times_size_sum`: The sum of (entry_price * size) for all entries
    /// - `position_funding_index`: The funding index at the last position update
    /// - `unrealized_funding_before_last_update`: Unrealized funding amount accumulated before last update
    /// - `mark_px`: The current mark price
    /// - `current_funding_index`: The current funding index
    /// - `size_multiplier`: Size multiplier for the position
    /// - `haircut_bps`: Haircut in basis points for PnL (applies only if for_free_collateral is true)
    /// - `margin_leverage`: The leverage for margin calculation
    /// - `for_free_collateral`: Whether to apply haircut to PnL
    ///
    /// Returns: (unrealized_pnl, initial_margin, total_notional_value)
    public fun compute_position_contribution_at_price_and_funding(
        position_size: u64,
        position_is_long: bool,
        entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        haircut_bps: u64,
        margin_leverage: u8,
        for_free_collateral: bool
    ): (i64, u64, u64) {
        // Use Move implementation for testing, native for production
        if (__COMPILE_FOR_TESTING__) {
            compute_position_contribution_move_impl(
                position_size,
                position_is_long,
                entry_px_times_size_sum,
                position_funding_index,
                unrealized_funding_before_last_update,
                mark_px,
                current_funding_index,
                size_multiplier,
                haircut_bps,
                margin_leverage,
                for_free_collateral
            )
        } else {
            compute_position_contribution_internal(
                position_size,
                position_is_long,
                entry_px_times_size_sum,
                position_funding_index,
                unrealized_funding_before_last_update,
                mark_px,
                current_funding_index,
                size_multiplier,
                haircut_bps,
                margin_leverage,
                for_free_collateral
            )
        }
    }

    // ============================================================================
    // NATIVE IMPLEMENTATION
    // ============================================================================

    /// Native implementation of position contribution calculation.
    /// This function performs all the complex math operations natively in Rust
    /// for maximum performance.
    ///
    /// Returns: (unrealized_pnl: i64, initial_margin: u64, total_notional_value: u64)
    native fun compute_position_contribution_internal(
        position_size: u64,
        position_is_long: bool,
        entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        haircut_bps: u64,
        margin_leverage: u8,
        for_free_collateral: bool
    ): (i64, u64, u64);

    // ============================================================================
    // MOVE IMPLEMENTATION (for testing)
    // ============================================================================

    /// Move-based implementation for testing when native is not available.
    /// This mirrors the Rust implementation logic.
    fun compute_position_contribution_move_impl(
        position_size: u64,
        position_is_long: bool,
        entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        haircut_bps: u64,
        margin_leverage: u8,
        for_free_collateral: bool
    ): (i64, u64, u64) {
        // If position size is 0, return zeros
        if (position_size == 0) {
            return (0, 0, 0)
        };

        // Compute PnL with funding
        let pnl = pnl_with_funding_impl(
            position_size,
            position_is_long,
            entry_px_times_size_sum,
            position_funding_index,
            unrealized_funding_before_last_update,
            mark_px,
            current_funding_index,
            size_multiplier
        );

        // Apply haircut if for_free_collateral is true
        let final_pnl = if (for_free_collateral) {
            apply_pnl_haircut(pnl, haircut_bps)
        } else {
            pnl
        };

        // Calculate margin required
        let margin = margin_required_formula(position_size, mark_px, size_multiplier, margin_leverage);

        // Calculate notional value
        let notional = ((position_size as u128) * (mark_px as u128) / (size_multiplier as u128) as u64);

        (final_pnl, margin, notional)
    }

    /// Computes PnL with funding for a position
    fun pnl_with_funding_impl(
        position_size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        mark_price: u64,
        current_funding_index: i128,
        size_multiplier: u64
    ): i64 {
        let current_px_times_size = (mark_price as u128) * (position_size as u128);

        // Calculate price difference and direction
        let (is_positive, price_diff) = if (current_px_times_size >= entry_px_times_size_sum) {
            (is_long, current_px_times_size - entry_px_times_size_sum)
        } else {
            (!is_long, entry_px_times_size_sum - current_px_times_size)
        };

        // Calculate absolute PnL with directional rounding
        let absolute_pnl = div_direction_128(price_diff, (size_multiplier as u128), !is_positive);

        let pnl = if (is_positive) {
            (absolute_pnl as i64)
        } else {
            -((absolute_pnl as i64))
        };

        // Calculate funding cost
        let unrealized_funding_cost = get_funding_cost(
            position_funding_index,
            current_funding_index,
            position_size,
            size_multiplier,
            is_long
        );

        let total_funding_cost = unrealized_funding_before_last_update + unrealized_funding_cost;

        pnl - total_funding_cost
    }

    /// Applies haircut to positive PnL
    inline fun apply_pnl_haircut(pnl: i64, haircut_bps: u64): i64 {
        if (pnl > 0) {
            pnl * ((10000 - haircut_bps) as i64) / 10000
        } else {
            pnl
        }
    }

    /// Calculates margin required for a position
    inline fun margin_required_formula(size: u64, price: u64, size_multiplier: u64, leverage: u8): u64 {
        let divisor = (size_multiplier as u128) * (leverage as u128);
        (math128::ceil_div((size as u128) * (price as u128), divisor) as u64)
    }

    /// Calculates funding cost between two funding indices
    fun get_funding_cost(
        entry_index: i128,
        exit_index: i128,
        position_size: u64,
        position_size_multiplier: u64,
        for_long: bool
    ): i64 {
        let index_delta = exit_index - entry_index;
        let index_delta = if (!for_long) { -index_delta } else { index_delta };

        let (is_positive, delta_abs) = into_sign_and_amount_i128(index_delta);
        let divisor = (position_size_multiplier as u128) * RATE_SIZE_MULTIPLIER;
        let cost_abs = div_direction_128(delta_abs * (position_size as u128), divisor, is_positive);

        from_sign_and_amount(is_positive, (cost_abs as i64))
    }

    /// Division with directional rounding (ceil if `ceil` is true, floor otherwise)
    inline fun div_direction_128(a: u128, b: u128, ceil: bool): u128 {
        if (ceil) {
            math128::ceil_div(a, b)
        } else {
            a / b
        }
    }

    /// Extracts sign and absolute value from i128
    inline fun into_sign_and_amount_i128(value: i128): (bool, u128) {
        if (value >= 0) {
            (true, (value as u128))
        } else {
            (false, ((-value) as u128))
        }
    }

    /// Reconstructs i64 from sign and absolute value
    inline fun from_sign_and_amount(is_positive: bool, amount: i64): i64 {
        if (is_positive) { amount } else { -amount }
    }

    // ============================================================================
    // TESTS
    // ============================================================================

    #[test]
    fun test_zero_position() {
        let (pnl, margin, notional) = compute_position_contribution_at_price_and_funding(
            0,                                          // position_size
            true,                                       // position_is_long
            0,                                          // entry_px_times_size_sum
            0,                                          // position_funding_index
            0,                                          // unrealized_funding_before_last_update
            1000,                                       // mark_px
            100,                                        // current_funding_index
            1000,                                       // size_multiplier
            100,                                        // haircut_bps (1%)
            10,                                         // margin_leverage
            true                                        // for_free_collateral
        );

        assert!(pnl == 0, 0);
        assert!(margin == 0, 1);
        assert!(notional == 0, 2);
    }

    #[test]
    fun test_long_position_profit() {
        // Long position entered at price 1000, current price 1100 (10% profit)
        let (pnl, margin, notional) = compute_position_contribution_at_price_and_funding(
            100,                                        // position_size
            true,                                       // position_is_long
            100000,                                     // entry_px_times_size_sum (100 * 1000)
            0,                                          // position_funding_index
            0,                                          // unrealized_funding_before_last_update
            1100,                                       // mark_px (10% higher)
            0,                                          // current_funding_index (no funding)
            1,                                          // size_multiplier
            0,                                          // haircut_bps
            10,                                         // margin_leverage
            false                                       // for_free_collateral
        );

        // PnL should be 100 * 1100 - 100000 = 110000 - 100000 = 10000
        assert!(pnl == 10000, 0);
        // Margin = ceil(100 * 1100 / (1 * 10)) = ceil(11000) = 11000
        assert!(margin == 11000, 1);
        // Notional = 100 * 1100 / 1 = 110000
        assert!(notional == 110000, 2);
    }

    #[test]
    fun test_short_position_profit() {
        // Short position entered at price 1000, current price 900 (10% profit for short)
        let (pnl, _margin, _notional) = compute_position_contribution_at_price_and_funding(
            100,                                        // position_size
            false,                                      // position_is_long (short)
            100000,                                     // entry_px_times_size_sum (100 * 1000)
            0,                                          // position_funding_index
            0,                                          // unrealized_funding_before_last_update
            900,                                        // mark_px (10% lower - profit for short)
            0,                                          // current_funding_index (no funding)
            1,                                          // size_multiplier
            0,                                          // haircut_bps
            10,                                         // margin_leverage
            false                                       // for_free_collateral
        );

        // For short: current_px_times_size = 900 * 100 = 90000 < entry 100000
        // price_diff = 100000 - 90000 = 10000
        // is_positive = !is_long = true (profit for short when price goes down)
        assert!(pnl == 10000, 0);
    }

    #[test]
    fun test_haircut_application() {
        // Long position with profit, applying haircut
        let (pnl_no_haircut, _, _) = compute_position_contribution_at_price_and_funding(
            100,                                        // position_size
            true,                                       // position_is_long
            100000,                                     // entry_px_times_size_sum
            0,                                          // position_funding_index
            0,                                          // unrealized_funding_before_last_update
            1100,                                       // mark_px
            0,                                          // current_funding_index
            1,                                          // size_multiplier
            1000,                                       // haircut_bps (10%)
            10,                                         // margin_leverage
            false                                       // no haircut applied
        );

        let (pnl_with_haircut, _, _) = compute_position_contribution_at_price_and_funding(
            100,                                        // position_size
            true,                                       // position_is_long
            100000,                                     // entry_px_times_size_sum
            0,                                          // position_funding_index
            0,                                          // unrealized_funding_before_last_update
            1100,                                       // mark_px
            0,                                          // current_funding_index
            1,                                          // size_multiplier
            1000,                                       // haircut_bps (10%)
            10,                                         // margin_leverage
            true                                        // haircut applied for free collateral
        );

        // PnL without haircut = 10000
        assert!(pnl_no_haircut == 10000, 0);
        // PnL with 10% haircut = 10000 * (10000 - 1000) / 10000 = 10000 * 0.9 = 9000
        assert!(pnl_with_haircut == 9000, 1);
    }
}
