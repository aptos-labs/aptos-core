
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

Position state is passed as arguments.

Parameters:
- <code>position_size</code>: The size of the position in base units
- <code>position_is_long</code>: Whether the position is long (true) or short (false)
- <code>position_entry_px_times_size_sum</code>: The sum of (entry_price * size) for all entries
- <code>position_funding_index</code>: The funding index at the last position update
- <code>unrealized_funding_before_last_update</code>: Unrealized funding amount accumulated before last update
- <code>mark_px</code>: The current mark price
- <code>current_funding_index</code>: The current funding index
- <code>size_multiplier</code>: Size multiplier for the position
- <code>rate_size_multiplier</code>: Multiplier for funding rate calculations
- <code>haircut_bps</code>: Haircut in basis points for PnL (applies only if for_free_collateral is true)
- <code>margin_leverage</code>: The leverage for margin calculation
- <code>for_free_collateral</code>: Whether to apply haircut to PnL

Returns: (unrealized_pnl, initial_margin, total_notional_value)


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(position_size: u64, position_is_long: bool, position_entry_px_times_size_sum: u128, position_funding_index: i128, position_unrealized_funding_before_last_update: i64, mark_px: u64, current_funding_index: i128, size_multiplier: u64, rate_size_multiplier: u64, haircut_bps: u64, margin_leverage: u8, for_free_collateral: bool): (i64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(
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
    <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>

<a id="0x7_fast_native_computations_compute_position_contribution_internal"></a>

## Function `compute_position_contribution_internal`

Native implementation of position contribution calculation.
This function performs all the complex math operations natively in Rust
for maximum performance.

Returns: (unrealized_pnl: i64, initial_margin: u64, total_notional_value: u64)


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(position_size: u64, position_is_long: bool, position_entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, mark_px: u64, current_funding_index: i128, size_multiplier: u64, rate_size_multiplier: u64, haircut_bps: u64, margin_leverage: u8, for_free_collateral: bool): (i64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
