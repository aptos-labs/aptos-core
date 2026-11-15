// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native-function implementations for `aptos_experimental::native_order_book`.
//! Writes are staged in [`NativeOrderContext`]. An order value is unpacked
//! field by field into its `aptos-types` order type, whose writer produces the
//! BCS of the Move value; only the metadata `M` is serialized through its
//! runtime layout. The field order here must match `native_order_types.move`.
//!
//! TODO[native_order](metering): these natives charge no gas yet.

use crate::context::NativeOrderContext;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    on_chain_config::FeatureFlag,
    state_store::{
        native_order::{
            BulkOrderV1, NativeBulkOrder, NativeSingleOrder, SingleOrderV1, TimeInForce,
            TriggerCondition,
        },
        state_key::inner::TradingNativeKey,
    },
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{OPTION_NONE_TAG, OPTION_SOME_TAG},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    value_serde::ValueSerDeContext,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Matches `EFEATURE_DISABLED` in `native_order_book.move`.
const E_FEATURE_DISABLED: u64 = 1;
/// An enum variant this native does not know, e.g. a variant appended to an
/// order type after this native was written.
const E_UNKNOWN_VARIANT: u64 = 2;
/// The metadata type contains delayed fields or function values.
const E_UNSUPPORTED_METADATA: u64 = 3;

const V1: u16 = 0;

fn check_feature_enabled(context: &SafeNativeContext) -> SafeNativeResult<()> {
    if context
        .get_feature_flags()
        .is_enabled(FeatureFlag::NATIVE_ORDERBOOK)
    {
        Ok(())
    } else {
        Err(SafeNativeError::Abort {
            abort_code: E_FEATURE_DISABLED,
            abort_message: Some("NATIVE_ORDERBOOK feature flag is not enabled".to_string()),
        })
    }
}

/// `(name, NativeFunction)` pairs for the `native_order_book` module.
pub fn all_natives(builder: &SafeNativeBuilder) -> Vec<(String, NativeFunction)> {
    let natives: [(&'static str, RawSafeNative); 4] = [
        ("native_set_single_order", native_set_single_order),
        ("native_delete_single_order", native_delete_single_order),
        ("native_set_bulk_order", native_set_bulk_order),
        ("native_delete_bulk_order", native_delete_bulk_order),
    ];
    builder.make_named_natives(natives).collect()
}

// -----------------------------------------------------------------------
// Writes
// -----------------------------------------------------------------------

fn native_set_single_order(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let order = args.pop_back().ok_or_else(|| missing_arg("order"))?;
    let market = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    let enum_option = context.get_feature_flags().is_enum_option_enabled();
    let order = single_order_from_value(order, enum_option, |metadata| {
        serialize_metadata(context, &ty_args[0], &metadata)
    })?;
    let NativeSingleOrder::V1(header) = &order;
    let key = TradingNativeKey::Order {
        exchange,
        market,
        order_id: header.order_id,
    };
    let bytes = order.serialize().map_err(|e| arg_error(e.to_string()))?;
    context
        .extensions()
        .get::<NativeOrderContext>()
        .stage_set(key, bytes.into());
    Ok(smallvec![])
}

fn native_delete_single_order(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let order_id = args.pop_back().ok_or_else(|| missing_arg("order_id"))?;
    let market = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    let key = TradingNativeKey::Order {
        exchange,
        market,
        order_id: wrapped_u128(order_id, "order_id")?,
    };
    context
        .extensions()
        .get::<NativeOrderContext>()
        .stage_delete(key);
    Ok(smallvec![])
}

fn native_set_bulk_order(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let order = args.pop_back().ok_or_else(|| missing_arg("order"))?;
    let market = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    let order = bulk_order_from_value(order, |metadata| {
        serialize_metadata(context, &ty_args[0], &metadata)
    })?;
    let NativeBulkOrder::V1(header) = &order;
    let key = TradingNativeKey::BulkOrder {
        exchange,
        market,
        account: header.account,
    };
    let bytes = order.serialize().map_err(|e| arg_error(e.to_string()))?;
    context
        .extensions()
        .get::<NativeOrderContext>()
        .stage_set(key, bytes.into());
    Ok(smallvec![])
}

fn native_delete_bulk_order(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    check_feature_enabled(context)?;
    let account = safely_pop_arg!(args, AccountAddress);
    let market = safely_pop_arg!(args, AccountAddress);
    let exchange = safely_pop_arg!(args, AccountAddress);
    context
        .extensions()
        .get::<NativeOrderContext>()
        .stage_delete(TradingNativeKey::BulkOrder {
            exchange,
            market,
            account,
        });
    Ok(smallvec![])
}

/// BCS of the metadata `M`, through its runtime layout. Delayed fields and
/// function values are rejected.
fn serialize_metadata(
    context: &mut SafeNativeContext,
    ty: &Type,
    metadata: &Value,
) -> SafeNativeResult<Bytes> {
    let layout = context
        .type_to_type_layout_with_delayed_fields(ty)?
        .into_layout_when_has_no_delayed_fields()
        .ok_or_else(unsupported_metadata)?;
    ValueSerDeContext::new(context.max_value_nest_depth())
        .serialize(metadata, &layout)?
        .map(Bytes::from)
        .ok_or_else(unsupported_metadata)
}

// -----------------------------------------------------------------------
// Move values -> order types
// -----------------------------------------------------------------------

/// Unpacks a Move `NativeSingleOrder<M>`; `metadata` serializes the `M` field.
pub(crate) fn single_order_from_value(
    value: Value,
    enum_option: bool,
    metadata: impl FnOnce(Value) -> SafeNativeResult<Bytes>,
) -> SafeNativeResult<NativeSingleOrder> {
    let mut fields = variant_fields(value, V1, 12, "NativeSingleOrder")?.into_iter();
    let mut next = |name| fields.next().ok_or_else(|| missing_arg(name));
    let account = next("account")?.value_as::<AccountAddress>()?;
    let order_id = wrapped_u128(next("order_id")?, "order_id")?;
    let client_order_id = option(next("client_order_id")?, enum_option)?
        .map(string)
        .transpose()?;
    let price = next("price")?.value_as::<u64>()?;
    let orig_size = next("orig_size")?.value_as::<u64>()?;
    let remaining_size = next("remaining_size")?.value_as::<u64>()?;
    let is_bid = next("is_bid")?.value_as::<bool>()?;
    let trigger_condition = option(next("trigger_condition")?, enum_option)?
        .map(trigger_condition)
        .transpose()?;
    let time_in_force = time_in_force(next("time_in_force")?)?;
    let creation_time_micros = next("creation_time_micros")?.value_as::<u64>()?;
    let unique_priority_idx = wrapped_u128(next("unique_priority_idx")?, "unique_priority_idx")?;
    let metadata = metadata(next("metadata")?)?;
    Ok(NativeSingleOrder::V1(SingleOrderV1 {
        account,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        creation_time_micros,
        unique_priority_idx,
        metadata,
    }))
}

/// Unpacks a Move `NativeBulkOrder<M>`; `metadata` serializes the `M` field.
pub(crate) fn bulk_order_from_value(
    value: Value,
    metadata: impl FnOnce(Value) -> SafeNativeResult<Bytes>,
) -> SafeNativeResult<NativeBulkOrder> {
    let mut fields = variant_fields(value, V1, 10, "NativeBulkOrder")?.into_iter();
    let mut next = |name| fields.next().ok_or_else(|| missing_arg(name));
    let account = next("account")?.value_as::<AccountAddress>()?;
    let order_id = wrapped_u128(next("order_id")?, "order_id")?;
    let sequence_number = next("sequence_number")?.value_as::<u64>()?;
    let bid_prices = next("bid_prices")?.value_as::<Vec<u64>>()?;
    let bid_sizes = next("bid_sizes")?.value_as::<Vec<u64>>()?;
    let ask_prices = next("ask_prices")?.value_as::<Vec<u64>>()?;
    let ask_sizes = next("ask_sizes")?.value_as::<Vec<u64>>()?;
    let unique_priority_idx = wrapped_u128(next("unique_priority_idx")?, "unique_priority_idx")?;
    let creation_time_micros = next("creation_time_micros")?.value_as::<u64>()?;
    let metadata = metadata(next("metadata")?)?;
    Ok(NativeBulkOrder::V1(BulkOrderV1 {
        account,
        order_id,
        sequence_number,
        bid_prices,
        bid_sizes,
        ask_prices,
        ask_sizes,
        unique_priority_idx,
        creation_time_micros,
        metadata,
    }))
}

/// Fields of the enum `value` if it is `variant`; an unknown variant aborts.
fn variant_fields(
    value: Value,
    variant: u16,
    num_fields: usize,
    name: &str,
) -> SafeNativeResult<Vec<Value>> {
    let (tag, fields) = value.value_as::<Struct>()?.unpack_with_tag()?;
    if tag != variant {
        return Err(unknown_variant(name, tag));
    }
    let fields: Vec<Value> = fields.collect();
    if fields.len() != num_fields {
        return Err(arg_error(format!(
            "{name} expected {num_fields} fields, got {}",
            fields.len()
        )));
    }
    Ok(fields)
}

/// A one-field struct wrapping a `u128` (`OrderId`, `IncreasingIdx`).
fn wrapped_u128(value: Value, name: &str) -> SafeNativeResult<u128> {
    let mut fields = value.value_as::<Struct>()?.unpack()?;
    match (fields.next(), fields.next()) {
        (Some(inner), None) => Ok(inner.value_as::<u128>()?),
        _ => Err(arg_error(format!("{name} expected a single u128 field"))),
    }
}

/// Move `Option<T>` is an enum (`None` / `Some { e }`) when
/// `ENABLE_ENUM_OPTION` is on, and otherwise a struct holding a vector of at
/// most one element.
fn option(value: Value, enum_option: bool) -> SafeNativeResult<Option<Value>> {
    if enum_option {
        let (tag, mut fields) = value.value_as::<Struct>()?.unpack_with_tag()?;
        return match (tag, fields.next(), fields.next()) {
            (OPTION_NONE_TAG, None, _) => Ok(None),
            (OPTION_SOME_TAG, Some(element), None) => Ok(Some(element)),
            _ => Err(arg_error(format!(
                "Option has unexpected shape (tag {tag})"
            ))),
        };
    }
    let mut fields = value.value_as::<Struct>()?.unpack()?;
    let vec = match (fields.next(), fields.next()) {
        (Some(vec), None) => vec.value_as::<Vec<Value>>()?,
        _ => return Err(arg_error("Option expected a single vector field".into())),
    };
    let mut elements = vec.into_iter();
    match (elements.next(), elements.next()) {
        (element, None) => Ok(element),
        _ => Err(arg_error("Option holds more than one element".into())),
    }
}

/// Move `String` is a struct holding UTF-8 bytes.
fn string(value: Value) -> SafeNativeResult<String> {
    let mut fields = value.value_as::<Struct>()?.unpack()?;
    let bytes = match (fields.next(), fields.next()) {
        (Some(bytes), None) => bytes.value_as::<Vec<u8>>()?,
        _ => return Err(arg_error("String expected a single bytes field".into())),
    };
    String::from_utf8(bytes).map_err(|_| arg_error("String is not valid UTF-8".into()))
}

fn trigger_condition(value: Value) -> SafeNativeResult<TriggerCondition> {
    let (tag, fields) = value.value_as::<Struct>()?.unpack_with_tag()?;
    let mut fields = fields;
    let mut field = || -> SafeNativeResult<u64> {
        Ok(fields
            .next()
            .ok_or_else(|| missing_arg("trigger_condition"))?
            .value_as::<u64>()?)
    };
    match tag {
        0 => Ok(TriggerCondition::PriceMoveAbove(field()?)),
        1 => Ok(TriggerCondition::PriceMoveBelow(field()?)),
        2 => Ok(TriggerCondition::TimeBased(field()?)),
        tag => Err(unknown_variant("TriggerCondition", tag)),
    }
}

fn time_in_force(value: Value) -> SafeNativeResult<TimeInForce> {
    let (tag, _) = value.value_as::<Struct>()?.unpack_with_tag()?;
    match tag {
        0 => Ok(TimeInForce::Gtc),
        1 => Ok(TimeInForce::PostOnly),
        2 => Ok(TimeInForce::Ioc),
        tag => Err(unknown_variant("TimeInForce", tag)),
    }
}

fn unknown_variant(name: &str, tag: u16) -> SafeNativeError {
    SafeNativeError::Abort {
        abort_code: E_UNKNOWN_VARIANT,
        abort_message: Some(format!("unknown {name} variant {tag}")),
    }
}

fn unsupported_metadata() -> SafeNativeError {
    SafeNativeError::Abort {
        abort_code: E_UNSUPPORTED_METADATA,
        abort_message: Some(
            "order metadata must not contain delayed fields or function values".to_string(),
        ),
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
    use move_core_types::value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue};
    use std::sync::Arc;

    fn addr(byte: u8) -> AccountAddress {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(bytes)
    }

    fn vm_struct(fields: Vec<Value>) -> Value {
        Value::struct_(Struct::pack(fields))
    }

    fn vm_variant(tag: u16, fields: Vec<Value>) -> Value {
        Value::struct_(Struct::pack_variant(tag, fields))
    }

    /// Both runtime representations of `Option`; see [`option`].
    fn vm_option(enum_option: bool, element: Option<Value>) -> Value {
        match (enum_option, element) {
            (true, None) => vm_variant(OPTION_NONE_TAG, vec![]),
            (true, Some(element)) => vm_variant(OPTION_SOME_TAG, vec![element]),
            (false, element) => vm_struct(vec![Value::vector_unchecked(
                element.into_iter().collect::<Vec<_>>(),
            )
            .unwrap()]),
        }
    }

    fn vm_string(s: &str) -> Value {
        vm_struct(vec![Value::vector_u8(s.bytes())])
    }

    /// Metadata `M = struct { u64, vector<u8> }`.
    fn metadata_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(Arc::new(MoveStructLayout::new(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
        ])))
    }

    fn vm_metadata() -> Value {
        vm_struct(vec![Value::u64(42), Value::vector_u8([1u8, 2])])
    }

    fn move_metadata() -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(42),
            MoveValue::Vector(vec![MoveValue::U8(1), MoveValue::U8(2)]),
        ]))
    }

    fn serialize_with_layout(value: Value) -> SafeNativeResult<Bytes> {
        Ok(ValueSerDeContext::new(None)
            .serialize(&value, &metadata_layout())?
            .expect("metadata serializes")
            .into())
    }

    fn move_wrapped_u128(v: u128) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U128(v)]))
    }

    fn move_option(v: Option<MoveValue>) -> MoveValue {
        MoveValue::Struct(match v {
            None => MoveStruct::RuntimeVariant(OPTION_NONE_TAG, vec![]),
            Some(v) => MoveStruct::RuntimeVariant(OPTION_SOME_TAG, vec![v]),
        })
    }

    fn vm_single_order(enum_option: bool, trigger_tag: u16) -> Value {
        vm_variant(V1, vec![
            Value::address(addr(7)),
            vm_struct(vec![Value::u128(u128::MAX - 1)]),
            vm_option(enum_option, Some(vm_string("coid-1"))),
            Value::u64(50_000),
            Value::u64(10),
            Value::u64(4),
            Value::bool(true),
            vm_option(
                enum_option,
                Some(vm_variant(trigger_tag, vec![Value::u64(49_000)])),
            ),
            vm_variant(1, vec![]),
            Value::u64(1_700_000_000_000_000),
            vm_struct(vec![Value::u128(123_456_789)]),
            vm_metadata(),
        ])
    }

    /// The unpacker reads every field in Move declaration order: its output
    /// bytes equal the Move BCS of the same value. Distinct values in the
    /// same-typed runs (`price`/`orig_size`/`remaining_size`,
    /// `order_id`/`unique_priority_idx`) catch a swapped read.
    #[test]
    fn single_order_matches_move_bcs() {
        let move_value = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::Address(addr(7)),
            move_wrapped_u128(u128::MAX - 1),
            move_option(Some(MoveValue::Vector(
                b"coid-1".iter().map(|b| MoveValue::U8(*b)).collect(),
            ))),
            MoveValue::U64(50_000),
            MoveValue::U64(10),
            MoveValue::U64(4),
            MoveValue::Bool(true),
            move_option(Some(MoveValue::Struct(MoveStruct::RuntimeVariant(
                1,
                vec![MoveValue::U64(49_000)],
            )))),
            MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![])),
            MoveValue::U64(1_700_000_000_000_000),
            move_wrapped_u128(123_456_789),
            move_metadata(),
        ]));
        for enum_option in [true, false] {
            let order = ok(single_order_from_value(
                vm_single_order(enum_option, 1),
                enum_option,
                serialize_with_layout,
            ));
            assert_eq!(
                order.serialize().unwrap(),
                bcs::to_bytes(&move_value).unwrap()
            );
        }
    }

    #[test]
    fn single_order_without_optional_fields() {
        for enum_option in [true, false] {
            single_order_without_optional_fields_impl(enum_option);
        }
    }

    fn single_order_without_optional_fields_impl(enum_option: bool) {
        let value = vm_variant(V1, vec![
            Value::address(addr(7)),
            vm_struct(vec![Value::u128(1)]),
            vm_option(enum_option, None),
            Value::u64(1),
            Value::u64(2),
            Value::u64(3),
            Value::bool(false),
            vm_option(enum_option, None),
            vm_variant(0, vec![]),
            Value::u64(4),
            vm_struct(vec![Value::u128(5)]),
            vm_metadata(),
        ]);
        let NativeSingleOrder::V1(order) = ok(single_order_from_value(
            value,
            enum_option,
            serialize_with_layout,
        ));
        assert_eq!(order.client_order_id, None);
        assert_eq!(order.trigger_condition, None);
        assert_eq!(order.time_in_force, TimeInForce::Gtc);
    }

    #[test]
    fn bulk_order_matches_move_bcs() {
        let value = vm_variant(V1, vec![
            Value::address(addr(9)),
            vm_struct(vec![Value::u128(77)]),
            Value::u64(3),
            Value::vector_u64([100, 99, 98]),
            Value::vector_u64([1, 2, 3]),
            Value::vector_u64([101, 102]),
            Value::vector_u64([4, 5]),
            vm_struct(vec![Value::u128(88)]),
            Value::u64(1_700_000_000_000_001),
            vm_metadata(),
        ]);
        let order = ok(bulk_order_from_value(value, serialize_with_layout));
        let u64s = |v: &[u64]| MoveValue::Vector(v.iter().map(|x| MoveValue::U64(*x)).collect());
        let move_value = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::Address(addr(9)),
            move_wrapped_u128(77),
            MoveValue::U64(3),
            u64s(&[100, 99, 98]),
            u64s(&[1, 2, 3]),
            u64s(&[101, 102]),
            u64s(&[4, 5]),
            move_wrapped_u128(88),
            MoveValue::U64(1_700_000_000_000_001),
            move_metadata(),
        ]));
        assert_eq!(
            order.serialize().unwrap(),
            bcs::to_bytes(&move_value).unwrap()
        );
    }

    /// `SafeNativeError` has no `Debug`, so results are unwrapped by hand.
    fn ok<T>(result: SafeNativeResult<T>) -> T {
        match result {
            Ok(value) => value,
            Err(SafeNativeError::Abort { abort_code, .. }) => panic!("aborted with {abort_code}"),
            Err(_) => panic!("failed with a non-abort error"),
        }
    }

    fn assert_abort<T>(result: SafeNativeResult<T>, code: u64) {
        match result {
            Err(SafeNativeError::Abort { abort_code, .. }) => assert_eq!(abort_code, code),
            Err(_) => panic!("expected abort {code}, got a non-abort error"),
            Ok(_) => panic!("expected abort {code}, got Ok"),
        }
    }

    #[test]
    fn unknown_variants_abort() {
        assert_abort(
            single_order_from_value(vm_single_order(true, 3), true, serialize_with_layout),
            E_UNKNOWN_VARIANT,
        );
        let mut fields = match vm_single_order(true, 1)
            .value_as::<Struct>()
            .unwrap()
            .unpack_with_tag()
        {
            Ok((_, fields)) => fields.collect::<Vec<_>>(),
            Err(e) => panic!("{e:?}"),
        };
        fields[8] = vm_variant(3, vec![]);
        assert_abort(
            single_order_from_value(vm_variant(V1, fields), true, serialize_with_layout),
            E_UNKNOWN_VARIANT,
        );
        assert_abort(
            single_order_from_value(vm_variant(1, vec![]), true, serialize_with_layout),
            E_UNKNOWN_VARIANT,
        );
    }

    /// A value in the other `Option` representation than the flag selects is
    /// rejected, not misread.
    #[test]
    fn option_representation_must_match_flag() {
        for enum_option in [true, false] {
            assert!(single_order_from_value(
                vm_single_order(!enum_option, 1),
                enum_option,
                serialize_with_layout,
            )
            .is_err());
        }
    }
}
