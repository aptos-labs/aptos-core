// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native-function implementations for `aptos_experimental::native_collateral`.
//! Writes are staged in [`NativeCollateralContext`]. A collateral value is
//! unpacked field by field into `NativeCollateral`, whose BCS is the Move
//! value's once its snapshot is materialized. Under the delayed field
//! optimization the primary balance is a snapshot identifier, written as its
//! `u64` encoding and replaced with the committed value by the block executor.
//! The field order here must match `native_collateral_types.move`.
//!
//! TODO[native_collateral](metering): these natives charge no gas yet.

use crate::context::NativeCollateralContext;
use aptos_framework_natives::aggregator_natives::NativeAggregatorContext;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    on_chain_config::FeatureFlag,
    state_store::{
        native_collateral::{NativeCollateral, SecondaryBalance},
        state_key::inner::{CollateralScope, TradingNativeKey},
    },
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Matches `EFEATURE_DISABLED` in `native_collateral.move`.
const E_FEATURE_DISABLED: u64 = 1;
/// An enum variant this native does not know, e.g. a variant appended to a
/// collateral type after this native was written.
const E_UNKNOWN_VARIANT: u64 = 2;
/// `AGGREGATOR_V2_DELAYED_FIELDS` is off. The block executor materializes the
/// primary balance's snapshot identifier, which exists only with it on.
const E_DELAYED_FIELDS_DISABLED: u64 = 3;

const V1: u16 = 0;
const SCOPE_CROSS: u16 = 0;
const SCOPE_ISOLATED: u16 = 1;

fn check_features_enabled(context: &SafeNativeContext) -> SafeNativeResult<()> {
    let features = context.get_feature_flags();
    if !features.is_enabled(FeatureFlag::NATIVE_COLLATERAL) {
        return Err(SafeNativeError::Abort {
            abort_code: E_FEATURE_DISABLED,
            abort_message: Some("NATIVE_COLLATERAL feature flag is not enabled".to_string()),
        });
    }
    if !features.is_aggregator_v2_delayed_fields_enabled() {
        return Err(SafeNativeError::Abort {
            abort_code: E_DELAYED_FIELDS_DISABLED,
            abort_message: Some(
                "AGGREGATOR_V2_DELAYED_FIELDS feature flag is not enabled".to_string(),
            ),
        });
    }
    Ok(())
}

/// `(name, NativeFunction)` pairs for the `native_collateral` module.
pub fn all_natives(builder: &SafeNativeBuilder) -> Vec<(String, NativeFunction)> {
    let natives: [(&'static str, RawSafeNative); 2] = [
        ("native_set_collateral", native_set_collateral),
        ("native_delete_collateral", native_delete_collateral),
    ];
    builder.make_named_natives(natives).collect()
}

// -----------------------------------------------------------------------
// Writes
// -----------------------------------------------------------------------

fn native_set_collateral(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_features_enabled(context)?;
    let collateral = args.pop_back().ok_or_else(|| missing_arg("collateral"))?;
    let scope = scope_from_value(args.pop_back().ok_or_else(|| missing_arg("scope"))?)?;
    let account = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    let delayed_fields = context
        .extensions()
        .get::<NativeAggregatorContext>()
        .delayed_field_optimization_enabled();
    let collateral = collateral_from_value(collateral, delayed_fields)?;
    let bytes = collateral
        .serialize()
        .map_err(|e| arg_error(e.to_string()))?;
    context
        .extensions()
        .get::<NativeCollateralContext>()
        .stage_set(
            TradingNativeKey::Collateral {
                exchange,
                account,
                scope,
            },
            bytes.into(),
        );
    Ok(smallvec![])
}

fn native_delete_collateral(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_features_enabled(context)?;
    let scope = scope_from_value(args.pop_back().ok_or_else(|| missing_arg("scope"))?)?;
    let account = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    context
        .extensions()
        .get::<NativeCollateralContext>()
        .stage_delete(TradingNativeKey::Collateral {
            exchange,
            account,
            scope,
        });
    Ok(smallvec![])
}

// -----------------------------------------------------------------------
// Move -> native value conversions
// -----------------------------------------------------------------------

/// Unpacks a Move `CollateralScope`.
pub(crate) fn scope_from_value(value: Value) -> SafeNativeResult<CollateralScope> {
    let (tag, fields) = value.value_as::<Struct>()?.unpack_with_tag()?;
    let mut fields = fields.into_iter();
    let scope = match tag {
        SCOPE_CROSS => CollateralScope::Cross,
        SCOPE_ISOLATED => CollateralScope::Isolated {
            market: fields
                .next()
                .ok_or_else(|| missing_arg("market"))?
                .value_as::<AccountAddress>()?,
        },
        tag => return Err(unknown_variant("CollateralScope", tag)),
    };
    if fields.next().is_some() {
        return Err(arg_error("CollateralScope has extra fields".to_string()));
    }
    Ok(scope)
}

/// Unpacks a Move `Collateral`. With `delayed_fields` the primary balance must
/// be a snapshot identifier, and otherwise a concrete value.
pub(crate) fn collateral_from_value(
    value: Value,
    delayed_fields: bool,
) -> SafeNativeResult<NativeCollateral> {
    let (tag, fields) = value.value_as::<Struct>()?.unpack_with_tag()?;
    if tag != V1 {
        return Err(unknown_variant("Collateral", tag));
    }
    let fields: Vec<Value> = fields.collect();
    if fields.len() != 3 {
        return Err(arg_error(format!(
            "Collateral expected 3 fields, got {}",
            fields.len()
        )));
    }
    let mut fields = fields.into_iter();
    let mut next = |name| fields.next().ok_or_else(|| missing_arg(name));
    let primary_offset = snapshot_value(next("primary")?, delayed_fields)?;
    let secondaries = next("secondaries")?
        .value_as::<Vec<Value>>()?
        .into_iter()
        .map(secondary_balance)
        .collect::<SafeNativeResult<Vec<_>>>()?;
    let reserved = next("reserved")?.value_as::<u64>()?;
    Ok(NativeCollateral::V1 {
        primary_offset,
        secondaries,
        reserved,
    })
}

/// The `u64` an `AggregatorSnapshot<u64>` serializes to: the encoded
/// identifier under the delayed field optimization, else the value.
fn snapshot_value(value: Value, delayed_fields: bool) -> SafeNativeResult<u64> {
    let mut fields = value.value_as::<Struct>()?.unpack()?;
    let inner = match (fields.next(), fields.next()) {
        (Some(inner), None) => inner,
        _ => {
            return Err(arg_error(
                "AggregatorSnapshot expected one field".to_string(),
            ))
        },
    };
    Ok(if delayed_fields {
        inner.value_as::<DelayedFieldID>()?.as_u64()
    } else {
        inner.value_as::<u64>()?
    })
}

fn secondary_balance(value: Value) -> SafeNativeResult<SecondaryBalance> {
    let mut fields = value.value_as::<Struct>()?.unpack()?;
    let mut next = |name| fields.next().ok_or_else(|| missing_arg(name));
    let asset = next("asset")?.value_as::<AccountAddress>()?;
    let amount = next("amount")?.value_as::<u64>()?;
    Ok(SecondaryBalance { asset, amount })
}

fn unknown_variant(name: &str, tag: u16) -> SafeNativeError {
    SafeNativeError::Abort {
        abort_code: E_UNKNOWN_VARIANT,
        abort_message: Some(format!("unknown {name} variant {tag}")),
    }
}

fn arg_error(msg: String) -> SafeNativeError {
    SafeNativeError::InvariantViolation(
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg),
    )
}

fn missing_arg(name: &str) -> SafeNativeError {
    arg_error(format!("missing argument: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::value::{MoveStruct, MoveValue};

    fn addr(byte: u8) -> AccountAddress {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(bytes)
    }

    fn ok<T>(result: SafeNativeResult<T>) -> T {
        result.unwrap_or_else(|_| panic!("native returned an error"))
    }

    fn vm_struct(fields: Vec<Value>) -> Value {
        Value::struct_(Struct::pack(fields))
    }

    fn vm_variant(tag: u16, fields: Vec<Value>) -> Value {
        Value::struct_(Struct::pack_variant(tag, fields))
    }

    fn vm_collateral(primary: Value) -> Value {
        vm_variant(V1, vec![
            vm_struct(vec![primary]),
            Value::vector_unchecked(vec![vm_struct(vec![
                Value::address(addr(0x42)),
                Value::u64(7),
            ])])
            .unwrap(),
            Value::u64(3),
        ])
    }

    /// BCS of the Move `Collateral::V1` with `primary` as its snapshot's u64.
    fn move_bytes(primary: u64) -> Vec<u8> {
        bcs::to_bytes(&MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(primary)])),
            MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
                MoveValue::Address(addr(0x42)),
                MoveValue::U64(7),
            ]))]),
            MoveValue::U64(3),
        ])))
        .unwrap()
    }

    #[test]
    fn concrete_primary_matches_move_bcs() {
        let collateral = ok(collateral_from_value(
            vm_collateral(Value::u64(1 << 63)),
            false,
        ));
        assert_eq!(collateral.primary(), 0);
        assert_eq!(collateral.serialize().unwrap(), move_bytes(1 << 63));
    }

    #[test]
    fn delayed_primary_is_written_as_the_identifier() {
        let id = DelayedFieldID::new_for_test_for_u64(5);
        let collateral = ok(collateral_from_value(
            vm_collateral(Value::delayed_value(id)),
            true,
        ));
        assert_eq!(collateral.serialize().unwrap(), move_bytes(id.as_u64()));
    }

    #[test]
    fn primary_must_match_the_delayed_field_mode() {
        let id = DelayedFieldID::new_for_test_for_u64(5);
        assert!(collateral_from_value(vm_collateral(Value::delayed_value(id)), false).is_err());
        assert!(collateral_from_value(vm_collateral(Value::u64(1 << 63)), true).is_err());
    }

    #[test]
    fn unknown_collateral_variant_aborts() {
        let value = vm_variant(1, vec![]);
        assert!(matches!(
            collateral_from_value(value, false),
            Err(SafeNativeError::Abort {
                abort_code: E_UNKNOWN_VARIANT,
                ..
            })
        ));
    }

    #[test]
    fn scopes_unpack() {
        assert_eq!(
            ok(scope_from_value(vm_variant(SCOPE_CROSS, vec![]))),
            CollateralScope::Cross
        );
        assert_eq!(
            ok(scope_from_value(vm_variant(SCOPE_ISOLATED, vec![
                Value::address(addr(9))
            ]))),
            CollateralScope::Isolated { market: addr(9) }
        );
        assert!(matches!(
            scope_from_value(vm_variant(2, vec![])),
            Err(SafeNativeError::Abort {
                abort_code: E_UNKNOWN_VARIANT,
                ..
            })
        ));
    }
}
