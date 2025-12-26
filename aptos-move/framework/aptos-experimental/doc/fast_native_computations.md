
<a id="0x7_fast_native_computations"></a>

# Module `0x7::fast_native_computations`

This module provides native implementations of computationally intensive
position contribution calculations for perpetual trading.
The native implementations are significantly faster than Move bytecode
equivalents for complex mathematical operations.


-  [Constants](#@Constants_0)
-  [Function `compute_position_contribution_at_price_and_funding`](#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding)
-  [Function `compute_position_contribution_internal`](#0x7_fast_native_computations_compute_position_contribution_internal)
-  [Function `compute_position_contribution_move_impl`](#0x7_fast_native_computations_compute_position_contribution_move_impl)
-  [Function `pnl_with_funding_impl`](#0x7_fast_native_computations_pnl_with_funding_impl)
-  [Function `apply_pnl_haircut`](#0x7_fast_native_computations_apply_pnl_haircut)
-  [Function `margin_required_formula`](#0x7_fast_native_computations_margin_required_formula)
-  [Function `get_funding_cost`](#0x7_fast_native_computations_get_funding_cost)
-  [Function `div_direction_128`](#0x7_fast_native_computations_div_direction_128)
-  [Function `into_sign_and_amount_i128`](#0x7_fast_native_computations_into_sign_and_amount_i128)
-  [Function `from_sign_and_amount`](#0x7_fast_native_computations_from_sign_and_amount)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_fast_native_computations_RATE_SIZE_MULTIPLIER"></a>

RATE_SIZE_MULTIPLIER constant used in funding cost calculations


<pre><code><b>const</b> <a href="fast_native_computations.md#0x7_fast_native_computations_RATE_SIZE_MULTIPLIER">RATE_SIZE_MULTIPLIER</a>: u128 = 1000000000000;
</code></pre>



<a id="0x7_fast_native_computations_compute_position_contribution_at_price_and_funding"></a>

## Function `compute_position_contribution_at_price_and_funding`

Compute contribution from a position state with a specific price and funding index.
This is the main entry point for computing position metrics.

Parameters:
- <code>position_size</code>: The size of the position in base units
- <code>position_is_long</code>: Whether the position is long (true) or short (false)
- <code>entry_px_times_size_sum</code>: The sum of (entry_price * size) for all entries
- <code>position_funding_index</code>: The funding index at the last position update
- <code>unrealized_funding_before_last_update</code>: Unrealized funding amount accumulated before last update
- <code>mark_px</code>: The current mark price
- <code>current_funding_index</code>: The current funding index
- <code>size_multiplier</code>: Size multiplier for the position
- <code>haircut_bps</code>: Haircut in basis points for PnL (applies only if for_free_collateral is true)
- <code>margin_leverage</code>: The leverage for margin calculation
- <code>for_free_collateral</code>: Whether to apply haircut to PnL

Returns: (unrealized_pnl, initial_margin, total_notional_value)


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(position_size: u64, position_is_long: bool, entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, mark_px: u64, current_funding_index: i128, size_multiplier: u64, haircut_bps: u64, margin_leverage: u8, for_free_collateral: bool): (i64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_at_price_and_funding">compute_position_contribution_at_price_and_funding</a>(
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
    // Use Move implementation for testing, <b>native</b> for production
    <b>if</b> (__COMPILE_FOR_TESTING__) {
        <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_move_impl">compute_position_contribution_move_impl</a>(
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
    } <b>else</b> {
        <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>

<a id="0x7_fast_native_computations_compute_position_contribution_internal"></a>

## Function `compute_position_contribution_internal`

Native implementation of position contribution calculation.
This function performs all the complex math operations natively in Rust
for maximum performance.

Returns: (unrealized_pnl: i64, initial_margin: u64, total_notional_value: u64)


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(position_size: u64, position_is_long: bool, entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, mark_px: u64, current_funding_index: i128, size_multiplier: u64, haircut_bps: u64, margin_leverage: u8, for_free_collateral: bool): (i64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_internal">compute_position_contribution_internal</a>(
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
</code></pre>



</details>

<a id="0x7_fast_native_computations_compute_position_contribution_move_impl"></a>

## Function `compute_position_contribution_move_impl`

Move-based implementation for testing when native is not available.
This mirrors the Rust implementation logic.


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_move_impl">compute_position_contribution_move_impl</a>(position_size: u64, position_is_long: bool, entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, mark_px: u64, current_funding_index: i128, size_multiplier: u64, haircut_bps: u64, margin_leverage: u8, for_free_collateral: bool): (i64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_compute_position_contribution_move_impl">compute_position_contribution_move_impl</a>(
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
    // If position size is 0, <b>return</b> zeros
    <b>if</b> (position_size == 0) {
        <b>return</b> (0, 0, 0)
    };

    // Compute PnL <b>with</b> funding
    <b>let</b> pnl = <a href="fast_native_computations.md#0x7_fast_native_computations_pnl_with_funding_impl">pnl_with_funding_impl</a>(
        position_size,
        position_is_long,
        entry_px_times_size_sum,
        position_funding_index,
        unrealized_funding_before_last_update,
        mark_px,
        current_funding_index,
        size_multiplier
    );

    // Apply haircut <b>if</b> for_free_collateral is <b>true</b>
    <b>let</b> final_pnl = <b>if</b> (for_free_collateral) {
        <a href="fast_native_computations.md#0x7_fast_native_computations_apply_pnl_haircut">apply_pnl_haircut</a>(pnl, haircut_bps)
    } <b>else</b> {
        pnl
    };

    // Calculate margin required
    <b>let</b> margin = <a href="fast_native_computations.md#0x7_fast_native_computations_margin_required_formula">margin_required_formula</a>(position_size, mark_px, size_multiplier, margin_leverage);

    // Calculate notional value
    <b>let</b> notional = ((position_size <b>as</b> u128) * (mark_px <b>as</b> u128) / (size_multiplier <b>as</b> u128) <b>as</b> u64);

    (final_pnl, margin, notional)
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_pnl_with_funding_impl"></a>

## Function `pnl_with_funding_impl`

Computes PnL with funding for a position


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_pnl_with_funding_impl">pnl_with_funding_impl</a>(position_size: u64, is_long: bool, entry_px_times_size_sum: u128, position_funding_index: i128, unrealized_funding_before_last_update: i64, mark_price: u64, current_funding_index: i128, size_multiplier: u64): i64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_pnl_with_funding_impl">pnl_with_funding_impl</a>(
    position_size: u64,
    is_long: bool,
    entry_px_times_size_sum: u128,
    position_funding_index: i128,
    unrealized_funding_before_last_update: i64,
    mark_price: u64,
    current_funding_index: i128,
    size_multiplier: u64
): i64 {
    <b>let</b> current_px_times_size = (mark_price <b>as</b> u128) * (position_size <b>as</b> u128);

    // Calculate price difference and direction
    <b>let</b> (is_positive, price_diff) = <b>if</b> (current_px_times_size &gt;= entry_px_times_size_sum) {
        (is_long, current_px_times_size - entry_px_times_size_sum)
    } <b>else</b> {
        (!is_long, entry_px_times_size_sum - current_px_times_size)
    };

    // Calculate absolute PnL <b>with</b> directional rounding
    <b>let</b> absolute_pnl = <a href="fast_native_computations.md#0x7_fast_native_computations_div_direction_128">div_direction_128</a>(price_diff, (size_multiplier <b>as</b> u128), !is_positive);

    <b>let</b> pnl = <b>if</b> (is_positive) {
        (absolute_pnl <b>as</b> i64)
    } <b>else</b> {
        -((absolute_pnl <b>as</b> i64))
    };

    // Calculate funding cost
    <b>let</b> unrealized_funding_cost = <a href="fast_native_computations.md#0x7_fast_native_computations_get_funding_cost">get_funding_cost</a>(
        position_funding_index,
        current_funding_index,
        position_size,
        size_multiplier,
        is_long
    );

    <b>let</b> total_funding_cost = unrealized_funding_before_last_update + unrealized_funding_cost;

    pnl - total_funding_cost
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_apply_pnl_haircut"></a>

## Function `apply_pnl_haircut`

Applies haircut to positive PnL


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_apply_pnl_haircut">apply_pnl_haircut</a>(pnl: i64, haircut_bps: u64): i64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_apply_pnl_haircut">apply_pnl_haircut</a>(pnl: i64, haircut_bps: u64): i64 {
    <b>if</b> (pnl &gt; 0) {
        pnl * ((10000 - haircut_bps) <b>as</b> i64) / 10000
    } <b>else</b> {
        pnl
    }
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_margin_required_formula"></a>

## Function `margin_required_formula`

Calculates margin required for a position


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_margin_required_formula">margin_required_formula</a>(size: u64, price: u64, size_multiplier: u64, leverage: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_margin_required_formula">margin_required_formula</a>(size: u64, price: u64, size_multiplier: u64, leverage: u8): u64 {
    <b>let</b> divisor = (size_multiplier <b>as</b> u128) * (leverage <b>as</b> u128);
    (<a href="../../aptos-framework/../aptos-stdlib/doc/math128.md#0x1_math128_ceil_div">math128::ceil_div</a>((size <b>as</b> u128) * (price <b>as</b> u128), divisor) <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_get_funding_cost"></a>

## Function `get_funding_cost`

Calculates funding cost between two funding indices


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_get_funding_cost">get_funding_cost</a>(entry_index: i128, exit_index: i128, position_size: u64, position_size_multiplier: u64, for_long: bool): i64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_get_funding_cost">get_funding_cost</a>(
    entry_index: i128,
    exit_index: i128,
    position_size: u64,
    position_size_multiplier: u64,
    for_long: bool
): i64 {
    <b>let</b> index_delta = exit_index - entry_index;
    <b>let</b> index_delta = <b>if</b> (!for_long) { -index_delta } <b>else</b> { index_delta };

    <b>let</b> (is_positive, delta_abs) = <a href="fast_native_computations.md#0x7_fast_native_computations_into_sign_and_amount_i128">into_sign_and_amount_i128</a>(index_delta);
    <b>let</b> divisor = (position_size_multiplier <b>as</b> u128) * <a href="fast_native_computations.md#0x7_fast_native_computations_RATE_SIZE_MULTIPLIER">RATE_SIZE_MULTIPLIER</a>;
    <b>let</b> cost_abs = <a href="fast_native_computations.md#0x7_fast_native_computations_div_direction_128">div_direction_128</a>(delta_abs * (position_size <b>as</b> u128), divisor, is_positive);

    <a href="fast_native_computations.md#0x7_fast_native_computations_from_sign_and_amount">from_sign_and_amount</a>(is_positive, (cost_abs <b>as</b> i64))
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_div_direction_128"></a>

## Function `div_direction_128`

Division with directional rounding (ceil if <code>ceil</code> is true, floor otherwise)


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_div_direction_128">div_direction_128</a>(a: u128, b: u128, ceil: bool): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_div_direction_128">div_direction_128</a>(a: u128, b: u128, ceil: bool): u128 {
    <b>if</b> (ceil) {
        <a href="../../aptos-framework/../aptos-stdlib/doc/math128.md#0x1_math128_ceil_div">math128::ceil_div</a>(a, b)
    } <b>else</b> {
        a / b
    }
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_into_sign_and_amount_i128"></a>

## Function `into_sign_and_amount_i128`

Extracts sign and absolute value from i128


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_into_sign_and_amount_i128">into_sign_and_amount_i128</a>(value: i128): (bool, u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_into_sign_and_amount_i128">into_sign_and_amount_i128</a>(value: i128): (bool, u128) {
    <b>if</b> (value &gt;= 0) {
        (<b>true</b>, (value <b>as</b> u128))
    } <b>else</b> {
        (<b>false</b>, ((-value) <b>as</b> u128))
    }
}
</code></pre>



</details>

<a id="0x7_fast_native_computations_from_sign_and_amount"></a>

## Function `from_sign_and_amount`

Reconstructs i64 from sign and absolute value


<pre><code><b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_from_sign_and_amount">from_sign_and_amount</a>(is_positive: bool, amount: i64): i64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fast_native_computations.md#0x7_fast_native_computations_from_sign_and_amount">from_sign_and_amount</a>(is_positive: bool, amount: i64): i64 {
    <b>if</b> (is_positive) { amount } <b>else</b> { -amount }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
