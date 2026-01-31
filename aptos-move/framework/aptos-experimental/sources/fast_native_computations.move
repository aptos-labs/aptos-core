/// This module provides native implementations of computationally intensive
/// position contribution calculations for perpetual trading.
/// The native implementations are significantly faster than Move bytecode
/// equivalents for complex mathematical operations.
module aptos_experimental::fast_native_computations {
    // ============================================================================
    // PUBLIC API
    // ============================================================================

    /// Compute contribution from a position state with a specific price and funding index.
    /// Entry point for calling the native function.
    ///
    /// This function computes position contribution for cross-margin positions.
    /// If the position is isolated, it returns all zeros.
    ///
    /// Parameters:
    /// - `position_size`: The size of the position in base units
    /// - `position_is_long`: Whether the position is long (true) or short (false)
    /// - `position_is_isolated`: Whether the position is isolated margin (returns zeros if true)
    /// - `position_entry_px_times_size_sum`: The sum of (entry_price * size) for all entries
    /// - `position_funding_index`: The funding index at the last position update
    /// - `unrealized_funding_before_last_update`: Unrealized funding amount accumulated before last update
    /// - `position_user_leverage`: The user's selected leverage for this position
    /// - `mark_px`: The current mark price
    /// - `current_funding_index`: The current funding index
    /// - `size_multiplier`: Size multiplier for the position
    /// - `rate_size_multiplier`: Multiplier for funding rate calculations
    /// - `haircut_bps`: Haircut in basis points for PnL (percentage to keep, e.g., 9000 = 90%)
    /// - `withdrawable_margin_leverage`: The withdrawable margin leverage from market
    /// - `market_max_leverage`: The maximum leverage allowed by the market
    ///
    /// Returns: (positions_pnl, pnl_haircutted, margin_for_max_leverage, margin_for_free_collateral, notional)
    public fun compute_position_contribution_at_price_and_funding(
        position_size: u64,
        position_is_long: bool,
        position_is_isolated: bool,
        position_entry_px_times_size_sum: u128,
        position_funding_index: i128,
        position_unrealized_funding_before_last_update: i64,
        position_user_leverage: u8,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        rate_size_multiplier: u64,
        haircut_bps: u64,
        withdrawable_margin_leverage: u8,
        market_max_leverage: u8
    ): (i64, i64, u64, u64, u64) {
        compute_position_contribution_internal(
            position_size,
            position_is_long,
            position_is_isolated,
            position_entry_px_times_size_sum,
            position_funding_index,
            position_unrealized_funding_before_last_update,
            position_user_leverage,
            mark_px,
            current_funding_index,
            size_multiplier,
            rate_size_multiplier,
            haircut_bps,
            withdrawable_margin_leverage,
            market_max_leverage
        )
    }

    // ============================================================================
    // NATIVE IMPLEMENTATION
    // ============================================================================

    /// Native implementation of position contribution calculation.
    /// This function performs all the complex math operations natively in Rust
    /// for maximum performance.
    ///
    /// Returns:
    /// - positions_pnl: i64 - Raw PnL with funding
    /// - pnl_haircutted: i64 - PnL after applying haircut (pnl * haircut_bps / 10000 if positive, else 0)
    /// - margin_for_max_leverage: u64 - Margin required at market max leverage
    /// - margin_for_free_collateral: u64 - Margin required at min(user_leverage, withdrawable_margin_leverage)
    /// - notional: u64 - Total notional value (size * mark_px / size_multiplier)
    public native fun compute_position_contribution_internal(
        position_size: u64,
        position_is_long: bool,
        position_is_isolated: bool,
        position_entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        position_user_leverage: u8,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        rate_size_multiplier: u64,
        haircut_bps: u64,
        withdrawable_margin_leverage: u8,
        market_max_leverage: u8
    ): (i64, i64, u64, u64, u64);
}
