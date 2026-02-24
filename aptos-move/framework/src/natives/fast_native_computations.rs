// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementations for fast position contribution calculations.
//! This module provides high-performance native functions for perpetual
//! trading computations that would be too expensive in Move bytecode.

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun compute_position_contribution_internal
 *
 * Computes the position contribution (unrealized PnL, initial margin, total notional value)
 * for a perpetual position given the current market conditions.
 *
 * This is a performance-critical function that consolidates multiple calculations:
 * 1. PnL calculation with funding
 * 2. Optional haircut application
 * 3. Margin requirement calculation
 * 4. Notional value calculation
 *
 * gas cost: base_cost
 *
 **************************************************************************************************/
fn native_compute_position_contribution_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 14);
    context.charge(FAST_NATIVE_COMPUTATIONS_COMPUTE_POSITION_CONTRIBUTION_BASE)?;

    // Pop arguments in reverse order (last argument first)
    let market_max_leverage = safely_pop_arg!(args, u8);
    let withdrawable_margin_leverage = safely_pop_arg!(args, u8);
    let haircut_bps = safely_pop_arg!(args, u64);
    let rate_size_multiplier = safely_pop_arg!(args, u64);
    let size_multiplier = safely_pop_arg!(args, u64);
    let current_funding_index = safely_pop_arg!(args, i128);
    let mark_px = safely_pop_arg!(args, u64);
    let position_user_leverage = safely_pop_arg!(args, u8);
    let unrealized_funding_before_last_update = safely_pop_arg!(args, i64);
    let position_funding_index = safely_pop_arg!(args, i128);
    let position_entry_px_times_size_sum = safely_pop_arg!(args, u128);
    let position_is_isolated = safely_pop_arg!(args, bool);
    let position_is_long = safely_pop_arg!(args, bool);
    let position_size = safely_pop_arg!(args, u64);

    // If position is isolated or size is 0, return zeros
    if position_is_isolated || position_size == 0 {
        return Ok(smallvec![
            Value::i64(0),  // positions_pnl
            Value::i64(0),  // pnl_haircutted
            Value::u64(0),  // margin_for_max_leverage
            Value::u64(0),  // margin_for_free_collateral
            Value::u64(0)   // notional
        ]);
    }

    // Compute PnL with funding
    let positions_pnl = compute_pnl_with_funding(
        position_size,
        position_is_long,
        position_entry_px_times_size_sum,
        position_funding_index,
        unrealized_funding_before_last_update,
        mark_px,
        current_funding_index,
        size_multiplier,
        rate_size_multiplier,
    );

    // Apply haircut to get haircutted PnL
    let pnl_haircutted = apply_upnl_haircut(positions_pnl, haircut_bps);

    // Calculate free_collateral_max_leverage = min(user_leverage, withdrawable_margin_leverage)
    let free_collateral_max_leverage =
        std::cmp::min(position_user_leverage, withdrawable_margin_leverage);

    // Calculate margin required for max leverage
    let margin_for_max_leverage =
        margin_required_formula(position_size, mark_px, size_multiplier, market_max_leverage);

    // Calculate margin required for free collateral
    let margin_for_free_collateral = margin_required_formula(
        position_size,
        mark_px,
        size_multiplier,
        free_collateral_max_leverage,
    );

    // Calculate notional value: (position_size * mark_px) / size_multiplier
    let notional = mul_div(position_size, mark_px, size_multiplier);

    // Return the results as a tuple
    Ok(smallvec![
        Value::i64(positions_pnl),
        Value::i64(pnl_haircutted),
        Value::u64(margin_for_max_leverage),
        Value::u64(margin_for_free_collateral),
        Value::u64(notional)
    ])
}

fn compute_pnl_with_funding(
    position_size: u64,
    position_is_long: bool,
    position_entry_px_times_size_sum: u128,
    position_funding_index: i128,
    unrealized_funding_before_last_update: i64,
    mark_price: u64,
    current_funding_index: i128,
    size_multiplier: u64,
    rate_size_multiplier: u64,
) -> i64 {
    let current_px_times_size = (mark_price as u128) * (position_size as u128);

    // Calculate price difference and direction
    let (is_positive, price_diff) = if current_px_times_size >= position_entry_px_times_size_sum {
        (position_is_long, current_px_times_size - position_entry_px_times_size_sum)
    } else {
        (!position_is_long, position_entry_px_times_size_sum - current_px_times_size)
    };

    // Calculate absolute PnL with directional rounding
    let absolute_pnl = div_direction_128(price_diff, size_multiplier as u128, !is_positive);

    let pnl = if is_positive {
        absolute_pnl as i64
    } else {
        -(absolute_pnl as i64)
    };

    // Calculate funding cost
    let unrealized_funding_cost = get_funding_cost(
        position_funding_index,
        current_funding_index,
        position_size,
        size_multiplier,
        rate_size_multiplier,
        position_is_long,
    );

    let total_funding_cost = unrealized_funding_before_last_update + unrealized_funding_cost;

    pnl - total_funding_cost
}

/// Apply unrealized PnL haircut.
/// Returns pnl * haircut_bps / 10000 if pnl > 0, otherwise 0.
fn apply_upnl_haircut(pnl: i64, haircut_bps: u64) -> i64 {
    if pnl > 0 {
        ((pnl as i128) * (haircut_bps as i128) / 10000) as i64
    } else {
        0
    }
}

fn margin_required_formula(size: u64, price: u64, size_multiplier: u64, leverage: u8) -> u64 {
    let divisor = size_multiplier as u128 * leverage as u128;
    ceil_div_128((size as u128) * (price as u128), divisor) as u64
}

fn get_funding_cost(
    entry_index: i128,
    exit_index: i128,
    position_size: u64,
    position_size_multiplier: u64,
    rate_size_multiplier: u64,
    for_long: bool,
) -> i64 {
    let mut index_delta = exit_index - entry_index;

    if !for_long {
        index_delta = -index_delta;
    }

    let (is_positive, delta_abs) = into_sign_and_amount_i128(index_delta);
    let divisor = (position_size_multiplier as u128) * (rate_size_multiplier as u128);
    let cost_abs = div_direction_128(delta_abs * (position_size as u128), divisor, is_positive);

    from_sign_and_amount(is_positive, cost_abs as i64)
}

#[inline]
fn div_direction_128(a: u128, b: u128, ceil: bool) -> u128 {
    if ceil {
        ceil_div_128(a, b)
    } else {
        if b == 0 {
            panic!("Division by zero");
        }
        a / b
    }
}

#[inline]
fn ceil_div_128(a: u128, b: u128) -> u128 {
    if b == 0 {
        panic!("Division by zero");
    } else if a == 0 {
        0
    } else {
        (a - 1) / b + 1
    }
}

#[inline]
fn into_sign_and_amount_i128(value: i128) -> (bool, u128) {
    if value >= 0 {
        (true, value as u128)
    } else {
        (false, (-value) as u128)
    }
}

#[inline]
fn from_sign_and_amount(is_positive: bool, amount: i64) -> i64 {
    if is_positive {
        amount
    } else {
        -amount
    }
}

#[inline]
fn mul_div(a: u64, b: u64, c: u64) -> u64 {
    debug_assert!(c != 0, "Division by zero");
    ((a as u128) * (b as u128) / (c as u128)) as u64
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [(
        "compute_position_contribution_internal",
        native_compute_position_contribution_internal as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_direction_128() {
        // Floor division
        assert_eq!(div_direction_128(10, 3, false), 3);
        // Ceiling division
        assert_eq!(div_direction_128(10, 3, true), 4);
        // Exact division
        assert_eq!(div_direction_128(9, 3, false), 3);
        assert_eq!(div_direction_128(9, 3, true), 3);
    }

    #[test]
    fn test_into_sign_and_amount() {
        assert_eq!(into_sign_and_amount_i128(100), (true, 100));
        assert_eq!(into_sign_and_amount_i128(-100), (false, 100));
        assert_eq!(into_sign_and_amount_i128(0), (true, 0));
    }

    #[test]
    fn test_from_sign_and_amount() {
        assert_eq!(from_sign_and_amount(true, 100), 100);
        assert_eq!(from_sign_and_amount(false, 100), -100);
    }

    #[test]
    fn test_apply_upnl_haircut() {
        // Positive PnL with 90% haircut (9000 bps = 90% kept)
        // Returns pnl * haircut_bps / 10000 = 10000 * 9000 / 10000 = 9000
        assert_eq!(apply_upnl_haircut(10000, 9000), 9000);
        // Positive PnL with 10% haircut (1000 bps = 10% kept)
        assert_eq!(apply_upnl_haircut(10000, 1000), 1000);
        // Negative PnL - returns 0
        assert_eq!(apply_upnl_haircut(-10000, 9000), 0);
        // Zero PnL - returns 0
        assert_eq!(apply_upnl_haircut(0, 9000), 0);
    }

    #[test]
    fn test_margin_required_formula() {
        // size=100, price=1100, size_multiplier=1, leverage=10
        // margin = ceil(100 * 1100 / (1 * 10)) = ceil(11000) = 11000
        assert_eq!(margin_required_formula(100, 1100, 1, 10), 11000);

        // Test with rounding
        // size=100, price=1000, size_multiplier=3, leverage=10
        // margin = ceil(100 * 1000 / (3 * 10)) = ceil(100000 / 30) = ceil(3333.33) = 3334
        assert_eq!(margin_required_formula(100, 1000, 3, 10), 3334);
    }

    #[test]
    fn test_compute_pnl_with_funding_long_profit() {
        // Long position: entered at avg price 1000 (100 * 1000 = 100000), current price 1100
        let pnl = compute_pnl_with_funding(
            100,                   // position_size
            true,                  // is_long
            100000,                // entry_px_times_size_sum (100 * 1000)
            0,                     // position_funding_index
            0,                     // unrealized_funding_before_last_update
            1100,                  // mark_price
            0,                     // current_funding_index
            1,                     // size_multiplier
            1_000_000_000_000,     // rate_size_multiplier (10^12)
        );
        // current_px_times_size = 100 * 1100 = 110000
        // price_diff = 110000 - 100000 = 10000
        // is_long and current > entry, so is_positive = true
        // pnl = 10000
        assert_eq!(pnl, 10000);
    }

    #[test]
    fn test_compute_pnl_with_funding_short_profit() {
        // Short position: entered at avg price 1000, current price 900 (profit for short)
        let pnl = compute_pnl_with_funding(
            100,                   // position_size
            false,                 // is_long (short)
            100000,                // entry_px_times_size_sum (100 * 1000)
            0,                     // position_funding_index
            0,                     // unrealized_funding_before_last_update
            900,                   // mark_price
            0,                     // current_funding_index
            1,                     // size_multiplier
            1_000_000_000_000,     // rate_size_multiplier (10^12)
        );
        // current_px_times_size = 100 * 900 = 90000
        // price_diff = 100000 - 90000 = 10000
        // !is_long and current < entry, so is_positive = !false = true
        // pnl = 10000
        assert_eq!(pnl, 10000);
    }
}
