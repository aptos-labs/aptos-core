// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native-function implementations for `aptos_trading::native_position`.
//! Writes are staged in [`NativePositionContext`] and drained at session
//! finalize. The `Position` Move value is unpacked positionally via
//! `Struct::unpack_with_tag()`, so the field order here must match the
//! Move `Position` declaration.

use crate::context::NativePositionContext;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    on_chain_config::FeatureFlag,
    state_store::{native_position::NativePosition, state_key::inner::TradingNativeKey},
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const POSITION_VARIANT_PERP_V1: u16 = 0;

/// Matches `EFEATURE_DISABLED` in `native_position.move`.
const E_FEATURE_DISABLED: u64 = 1;

fn check_feature_enabled(context: &SafeNativeContext) -> SafeNativeResult<()> {
    if context
        .get_feature_flags()
        .is_enabled(FeatureFlag::NATIVE_POSITION)
    {
        Ok(())
    } else {
        Err(SafeNativeError::Abort {
            abort_code: E_FEATURE_DISABLED,
            abort_message: Some("NATIVE_POSITION feature flag is not enabled".to_string()),
        })
    }
}

/// `(name, NativeFunction)` pairs for the `native_position` module.
pub fn all_natives(builder: &SafeNativeBuilder) -> Vec<(String, NativeFunction)> {
    let natives: [(&'static str, RawSafeNative); 2] = [
        ("native_set_position", native_set_position),
        ("native_delete_position", native_delete_position),
    ];
    builder.make_named_natives(natives).collect()
}

// -----------------------------------------------------------------------
// Writes
// -----------------------------------------------------------------------

fn native_set_position(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let position_val = args.pop_back().ok_or_else(|| missing_arg("position"))?;
    let account: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let market: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let exchange: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let native_pos = move_value_to_position(position_val)?;
    let ctx = context.extensions().get::<NativePositionContext>();
    let key = TradingNativeKey::Position {
        exchange,
        account,
        market,
    };
    ctx.stage_set(key, native_pos);
    Ok(smallvec![])
}

fn native_delete_position(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let account: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let market: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let exchange: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let ctx = context.extensions().get::<NativePositionContext>();
    let key = TradingNativeKey::Position {
        exchange,
        account,
        market,
    };
    ctx.stage_delete(key);
    Ok(smallvec![])
}

// -----------------------------------------------------------------------
// Move <-> Native Position conversions
// -----------------------------------------------------------------------

fn move_value_to_position(value: Value) -> SafeNativeResult<NativePosition> {
    let s = value.value_as::<Struct>().map_err(into_safe_error)?;
    let (variant, fields_iter) = s.unpack_with_tag().map_err(into_safe_error)?;
    let fields: Vec<Value> = fields_iter.collect();
    match variant {
        POSITION_VARIANT_PERP_V1 => {
            if fields.len() != 9 {
                return Err(arg_error(format!(
                    "PerpV1 expected 9 fields, got {}",
                    fields.len()
                )));
            }
            let mut iter = fields.into_iter();
            Ok(NativePosition::PerpV1 {
                size: pop_u64(&mut iter)?,
                is_long: pop_bool(&mut iter)?,
                entry_px_times_size_sum: pop_u128(&mut iter)?,
                avg_acquire_entry_px: pop_u64(&mut iter)?,
                user_leverage: pop_u8(&mut iter)?,
                is_isolated: pop_bool(&mut iter)?,
                funding_index_at_last_update: pop_accumulative_index(&mut iter)?,
                unrealized_funding_amount_before_last_update: pop_i64(&mut iter)?,
                timestamp: pop_u64(&mut iter)?,
            })
        },
        other => Err(arg_error(format!("unknown Position variant {}", other))),
    }
}

fn pop_u8(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<u8> {
    iter.next()
        .ok_or_else(|| arg_error("missing u8 field".into()))?
        .value_as::<u8>()
        .map_err(into_safe_error)
}

fn pop_u64(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<u64> {
    iter.next()
        .ok_or_else(|| arg_error("missing u64 field".into()))?
        .value_as::<u64>()
        .map_err(into_safe_error)
}

fn pop_u128(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<u128> {
    iter.next()
        .ok_or_else(|| arg_error("missing u128 field".into()))?
        .value_as::<u128>()
        .map_err(into_safe_error)
}

fn pop_i64(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<i64> {
    iter.next()
        .ok_or_else(|| arg_error("missing i64 field".into()))?
        .value_as::<i64>()
        .map_err(into_safe_error)
}

fn pop_i128(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<i128> {
    iter.next()
        .ok_or_else(|| arg_error("missing i128 field".into()))?
        .value_as::<i128>()
        .map_err(into_safe_error)
}

fn pop_accumulative_index(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<i128> {
    let val = iter
        .next()
        .ok_or_else(|| arg_error("missing AccumulativeIndex field".into()))?;
    let s = val.value_as::<Struct>().map_err(into_safe_error)?;
    let fields: Vec<Value> = s.unpack().map_err(into_safe_error)?.collect();
    if fields.len() != 1 {
        return Err(arg_error(format!(
            "AccumulativeIndex expected 1 field, got {}",
            fields.len()
        )));
    }
    let mut inner = fields.into_iter();
    pop_i128(&mut inner)
}

fn pop_bool(iter: &mut impl Iterator<Item = Value>) -> SafeNativeResult<bool> {
    iter.next()
        .ok_or_else(|| arg_error("missing bool field".into()))?
        .value_as::<bool>()
        .map_err(into_safe_error)
}

fn into_safe_error(e: PartialVMError) -> SafeNativeError {
    SafeNativeError::InvariantViolation(e)
}

fn arg_error(msg: String) -> SafeNativeError {
    SafeNativeError::InvariantViolation(
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg),
    )
}

fn missing_arg(name: &str) -> SafeNativeError {
    arg_error(format!("missing argument: {}", name))
}
