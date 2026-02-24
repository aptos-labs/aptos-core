
<a id="0x7_fast_native_computations"></a>

# Module `0x7::fast_native_computations`

This module provides native implementations of computationally intensive
position contribution calculations for perpetual trading.
The native implementations are significantly faster than Move bytecode
equivalents for complex mathematical operations.


-  [Function `compute_position_contribution_at_price_and_funding`](#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding)
-  [Function `compute_position_contribution_internal`](#0x7_fast_native_computations_compute_position_contribution_internal)


<pre><code></code></pre>



<a id="0x7_fast_native_computations_compute_position_contribution_at_price_and_funding"></a>

## Function `compute_position_contribution_at_price_and_funding`

Compute contribution from a position state with a specific price and funding index.
Entry point for calling the native function.

This function computes position contribution for cross-margin positions.
If the position is isolated, it returns all zeros.

Parameters:
- <code>position_size</code>: The size of the position in base units
- <code>position_is_long</code>: Whether the position is long (true) or short (false)
- <code>position_is_isolated</code>: Whether the position is isolated margin (returns zeros if true)
- <code>position_entry_px_times_size_sum</code>: The sum of (entry_price * size) for all entries
- <code>position_funding_index</code>: The funding index at the last position update
- <code>unrealized_funding_before_last_update</code>: Unrealized funding amount accumulated before last update
- <code>position_user_leverage</code>: The user's selected leverage for this position
- <code>mark_px</code>: The current mark price
- <code>current_funding_index</code>: The current funding index
- <code>size_multiplier</code>: Size multiplier for the position
- <code>rate_size_multiplier</code>: Multiplier for funding rate calculations
- <code>haircut_bps</code>: Haircut in basis points for PnL (percentage to keep, e.g., 9000 = 90%)
- <code>withdrawable_margin_leverage</code>: The withdrawable margin leverage from market
- <code>market_max_leverage</code>: The maximum leverage allowed by the market

Returns: (positions_pnl, pnl_haircutted, margin_for_max_leverage, margin_for_free_collateral, notional)


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(position_size: u64, position_is_long: bool, position_is_isolated: bool, position_entry_px_times_size_sum: u128, position_funding_index: i128, position_unrealized_funding_before_last_update: i64, position_user_leverage: u8, mark_px: u64, current_funding_index: i128, size_multiplier: u64, rate_size_multiplier: u64, haircut_bps: u64, withdrawable_margin_leverage: u8, market_max_leverage: u8): (i64, i64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(
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
    <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>

<a id="0x7_fast_native_computations_compute_position_contribution_internal"></a>

## Function `compute_position_contribution_internal`

Native implementation of position contribution calculation.
This function performs all the complex math operations natively in Rust
for maximum performance.

Returns:
- positions_pnl: i64 - Raw PnL with funding
- pnl_haircutted: i64 - PnL after applying haircut (pnl * haircut_bps / 10000 if positive, else 0)
- margin_for_max_leverage: u64 - Margin required at market max leverage
- margin_for_free_collateral: u64 - Margin required at min(user_leverage, withdrawable_margin_leverage)
- notional: u64 - Total notional value (size * mark_px / size_multiplier)


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(position_size: u64, position_is_long: bool, position_is_isolated: bool, position_entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, position_user_leverage: u8, mark_px: u64, current_funding_index: i128, size_multiplier: u64, rate_size_multiplier: u64, haircut_bps: u64, withdrawable_margin_leverage: u8, market_max_leverage: u8): (i64, i64, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
