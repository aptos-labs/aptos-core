// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native-function implementations for `aptos_experimental::native_position`.
//!
//! Writes are staged in the [`NativePositionContext`] session extension
//! at session-end time. The session finalize step (in the VM wiring)
//! drains the context's staged map and routes each write into the
//! `position_write_set` bucket of `VMChangeSet`. Move-side reads from
//! the in-memory store are deferred to milestone 2.
//!
//! Move-value decoding:
//! - `Position` is a Move enum with two variants (`PerpV1` / `SpotV1`).
//!   The native unpacks it via `Struct::unpack_with_tag()`; the
//!   variant tags below must match the declaration order in
//!   `aptos_experimental::native_position::Position`.

use crate::{
    context::NativePositionContext,
    position::{NativePosition, PositionKey},
};
use aptos_gas_schedule::gas_params::natives::position::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const POSITION_VARIANT_PERP_V1: u16 = 0;
const POSITION_VARIANT_SPOT_V1: u16 = 1;

/// Move-level abort code matching `EFEATURE_DISABLED` in
/// `native_position.move`.
const E_FEATURE_DISABLED: u64 = 1;

/// Assert that the `NATIVE_POSITION` feature flag is enabled, aborting
/// the native with the same error code the Move module uses.
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

/// Return the full list of `(name, NativeFunction)` pairs to be
/// registered under the `aptos_experimental::native_position` module.
///
/// Lifecycle (`register` / `deny` / `reenable`) is now pure Move,
/// authoritative state lives in the `ExchangeRegistry` resource at
/// `@aptos_framework`. Only the write-staging natives remain here.
pub fn all_natives(builder: &SafeNativeBuilder) -> Vec<(String, NativeFunction)> {
    let natives: [(&'static str, RawSafeNative); 3] = [
        ("native_create_position", native_create_position),
        ("native_update_position", native_update_position),
        ("native_remove_position", native_remove_position),
    ];
    builder.make_named_natives(natives).collect()
}

// -----------------------------------------------------------------------
// Writes
// -----------------------------------------------------------------------

fn native_create_position(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    context.charge(CREATE_POSITION_BASE)?;
    let position_val = args.pop_back().ok_or_else(|| missing_arg("position"))?;
    let market: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let account: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let exchange: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let native_pos = move_value_to_position(position_val)?;
    context
        .charge(CREATE_POSITION_PER_BYTE * NumBytes::new(native_pos.serialize().len() as u64))?;
    let ctx = context.extensions().get::<NativePositionContext>();
    let key = PositionKey {
        exchange,
        account,
        market,
    };
    ctx.stage_create(key, native_pos);
    Ok(smallvec![])
}

fn native_update_position(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    context.charge(UPDATE_POSITION_BASE)?;
    let position_val = args.pop_back().ok_or_else(|| missing_arg("position"))?;
    let market: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let account: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let exchange: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let native_pos = move_value_to_position(position_val)?;
    context
        .charge(UPDATE_POSITION_PER_BYTE * NumBytes::new(native_pos.serialize().len() as u64))?;
    let ctx = context.extensions().get::<NativePositionContext>();
    let key = PositionKey {
        exchange,
        account,
        market,
    };
    ctx.stage_update(key, native_pos);
    Ok(smallvec![])
}

fn native_remove_position(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    context.charge(REMOVE_POSITION_BASE)?;
    let market: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let account: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let exchange: AccountAddress = safely_pop_arg!(args, AccountAddress);
    let ctx = context.extensions().get::<NativePositionContext>();
    let key = PositionKey {
        exchange,
        account,
        market,
    };
    ctx.stage_remove(key);
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
                avg_entry_px: pop_u64(&mut iter)?,
                user_leverage: pop_u8(&mut iter)?,
                is_isolated: pop_bool(&mut iter)?,
                funding_index: pop_i128(&mut iter)?,
                unrealized_funding_before: pop_i64(&mut iter)?,
                timestamp: pop_u64(&mut iter)?,
            })
        },
        POSITION_VARIANT_SPOT_V1 => {
            if fields.len() != 5 {
                return Err(arg_error(format!(
                    "SpotV1 expected 5 fields, got {}",
                    fields.len()
                )));
            }
            let mut iter = fields.into_iter();
            Ok(NativePosition::SpotV1 {
                size: pop_u64(&mut iter)?,
                is_long: pop_bool(&mut iter)?,
                entry_px_times_size_sum: pop_u128(&mut iter)?,
                avg_entry_px: pop_u64(&mut iter)?,
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
