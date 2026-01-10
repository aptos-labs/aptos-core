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
    /// Position state is passed as arguments.
    ///
    /// Parameters:
    /// - `position_size`: The size of the position in base units
    /// - `position_is_long`: Whether the position is long (true) or short (false)
    /// - `position_entry_px_times_size_sum`: The sum of (entry_price * size) for all entries
    /// - `position_funding_index`: The funding index at the last position update
    /// - `unrealized_funding_before_last_update`: Unrealized funding amount accumulated before last update
    /// - `mark_px`: The current mark price
    /// - `current_funding_index`: The current funding index
    /// - `size_multiplier`: Size multiplier for the position
    /// - `rate_size_multiplier`: Multiplier for funding rate calculations
    /// - `haircut_bps`: Haircut in basis points for PnL (applies only if for_free_collateral is true)
    /// - `margin_leverage`: The leverage for margin calculation
    /// - `for_free_collateral`: Whether to apply haircut to PnL
    ///
    /// Returns: (unrealized_pnl, initial_margin, total_notional_value)
    public fun compute_position_contribution_at_price_and_funding(
        position_size: u64,
        position_is_long: bool,
        position_entry_px_times_size_sum: u128,
        position_funding_index: i128,
        position_unrealized_funding_before_last_update: i64,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        rate_size_multiplier: u64,
        haircut_bps: u64,
        margin_leverage: u8,
        for_free_collateral: bool
    ): (i64, u64, u64) {
        compute_position_contribution_internal(
            position_size,
            position_is_long,
            position_entry_px_times_size_sum,
            position_funding_index,
            position_unrealized_funding_before_last_update,
            mark_px,
            current_funding_index,
            size_multiplier,
            rate_size_multiplier,
            haircut_bps,
            margin_leverage,
            for_free_collateral
        )
    }

    // ============================================================================
    // NATIVE IMPLEMENTATION
    // ============================================================================

    /// Native implementation of position contribution calculation.
    /// This function performs all the complex math operations natively in Rust
    /// for maximum performance.
    ///
    /// Returns: (unrealized_pnl: i64, initial_margin: u64, total_notional_value: u64)
    public native fun compute_position_contribution_internal(
        position_size: u64,
        position_is_long: bool,
        position_entry_px_times_size_sum: u128,
        position_funding_index: i128,
        unrealized_funding_before_last_update: i64,
        mark_px: u64,
        current_funding_index: i128,
        size_multiplier: u64,
        rate_size_multiplier: u64,
        haircut_bps: u64,
        margin_leverage: u8,
        for_free_collateral: bool
    ): (i64, u64, u64);
}
