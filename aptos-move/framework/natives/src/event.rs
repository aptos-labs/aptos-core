// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
#[cfg(feature = "testing")]
use aptos_types::account_address::AccountAddress;
use aptos_types::contract_event::ContractEvent;
#[cfg(feature = "testing")]
use aptos_types::event::EventKey;
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_core_types::{language_storage::TypeTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
};
#[cfg(feature = "testing")]
use move_vm_types::values::{Reference, Struct, StructRef};
use move_vm_types::{
    loaded_data::runtime_types::Type, value_serde::ValueSerDeContext, values::Value,
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Error code from `0x1::events.move`, returned when event creation fails.
pub const ECANNOT_CREATE_EVENT: u64 = 1;

/// Cached emitted module events.
#[derive(Default, Tid)]
pub struct NativeEventContext {
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
}

impl SessionListener for NativeEventContext {
    fn start(&mut self, _session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        // State is handled by finish-abort, session start does not impact anything.
    }

    fn finish(&mut self) {
        // TODO(sessions): implement
    }

    fn abort(&mut self) {
        // TODO(sessions): implement
    }
}

impl NativeRuntimeRefCheckModelsCompleted for NativeEventContext {
    // No native functions in this context return references, so no models to add.
}

impl NativeEventContext {
    /// Returns events from the current context. Only used for non-continuous sessions which are
    /// now legacy.
    pub fn legacy_into_events(self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        self.events
    }

    /// Returns iterator over all events seen so far.
    pub fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.events.iter().map(|(event, _)| event)
    }

    #[cfg(feature = "testing")]
    fn emitted_v1_events(&self, event_key: &EventKey, ty_tag: &TypeTag) -> Vec<&[u8]> {
        let mut events = vec![];
        for event in self.events.iter() {
            if let (ContractEvent::V1(e), _) = event {
                if e.key() == event_key && e.type_tag() == ty_tag {
                    events.push(e.event_data());
                }
            }
        }
        events
    }

    #[cfg(feature = "testing")]
    fn emitted_v2_events(&self, ty_tag: &TypeTag) -> Vec<&[u8]> {
        let mut events = vec![];
        for event in self.events.iter() {
            if let (ContractEvent::V2(e), _) = event {
                if e.type_tag() == ty_tag {
                    events.push(e.event_data());
                }
            }
        }
        events
    }
}

/***************************************************************************************************
 * native fun write_to_event_store
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[inline]
fn native_write_to_event_store(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 3);

    let ty = &ty_args[0];
    let msg = arguments.pop_back().unwrap();
    let seq_num = safely_pop_arg!(arguments, u64);
    let guid = safely_pop_arg!(arguments, Vec<u8>);

    // TODO(Gas): Get rid of abstract memory size
    context.charge(
        EVENT_WRITE_TO_EVENT_STORE_BASE
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg)?,
    )?;
    let ty_tag = context.type_to_type_tag(ty)?;
    let (layout, contains_delayed_fields) = context
        .type_to_type_layout_with_delayed_fields(ty)?
        .unpack();

    let function_value_extension = context.function_value_extension();
    let max_value_nest_depth = context.max_value_nest_depth();
    let blob = ValueSerDeContext::new(max_value_nest_depth)
        .with_delayed_fields_serde()
        .with_func_args_deserialization(&function_value_extension)
        .serialize(&msg, &layout)?
        .ok_or_else(|| {
            SafeNativeError::InvariantViolation(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ))
        })?;
    let key = bcs::from_bytes(guid.as_slice()).map_err(|_| {
        SafeNativeError::InvariantViolation(PartialVMError::new(StatusCode::EVENT_KEY_MISMATCH))
    })?;

    let ctx = context.extensions_mut().get_mut::<NativeEventContext>();
    let event = ContractEvent::new_v1(key, seq_num, ty_tag, blob).map_err(|_| {
        SafeNativeError::abort_with_message(
            ECANNOT_CREATE_EVENT,
            "Event v1 size is not computable: type tag may be invalid or too complex",
        )
    })?;
    // TODO(layouts): avoid cloning layouts for events with delayed fields.
    ctx.events.push((
        event,
        contains_delayed_fields.then(|| layout.as_ref().clone()),
    ));
    Ok(smallvec![])
}

#[cfg(feature = "testing")]
fn native_emitted_events_by_handle(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 1);

    let ty = &ty_args[0];
    let mut guid = safely_pop_arg!(arguments, StructRef)
        .borrow_field(1)?
        .value_as::<StructRef>()?
        .borrow_field(0)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<Struct>()?
        .unpack()?;

    let creation_num = guid
        .next()
        .ok_or_else(|| {
            SafeNativeError::InvariantViolation(PartialVMError::new(
                StatusCode::INTERNAL_TYPE_ERROR,
            ))
        })?
        .value_as::<u64>()?;
    let addr = guid
        .next()
        .ok_or_else(|| {
            SafeNativeError::InvariantViolation(PartialVMError::new(
                StatusCode::INTERNAL_TYPE_ERROR,
            ))
        })?
        .value_as::<AccountAddress>()?;
    let key = EventKey::new(creation_num, addr);
    let ty_tag = context.type_to_type_tag(ty)?;
    let ty_layout = context.type_to_type_layout_check_no_delayed_fields(ty)?;
    let ctx = context.extensions().get::<NativeEventContext>();
    let events = ctx
        .emitted_v1_events(&key, &ty_tag)
        .into_iter()
        .map(|blob| {
            let function_value_extension = context.function_value_extension();
            let max_value_nest_depth = context.max_value_nest_depth();
            ValueSerDeContext::new(max_value_nest_depth)
                .with_func_args_deserialization(&function_value_extension)
                .deserialize(blob, &ty_layout)
                .ok_or_else(|| {
                    SafeNativeError::InvariantViolation(PartialVMError::new(
                        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    ))
                })
        })
        .collect::<SafeNativeResult<Vec<Value>>>()?;
    Ok(smallvec![Value::vector_unchecked(events)?])
}

#[cfg(feature = "testing")]
fn native_emitted_events(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    let ty = &ty_args[0];

    let ty_tag = context.type_to_type_tag(ty)?;
    let ty_layout = context.type_to_type_layout_check_no_delayed_fields(ty)?;
    let ctx = context.extensions().get::<NativeEventContext>();

    let events = ctx
        .emitted_v2_events(&ty_tag)
        .into_iter()
        .map(|blob| {
            let function_value_extension = context.function_value_extension();
            let max_value_nest_depth = context.max_value_nest_depth();
            ValueSerDeContext::new(max_value_nest_depth)
                .with_func_args_deserialization(&function_value_extension)
                .with_delayed_fields_serde()
                .deserialize(blob, &ty_layout)
                .ok_or_else(|| {
                    SafeNativeError::InvariantViolation(PartialVMError::new(
                        StatusCode::VALUE_DESERIALIZATION_ERROR,
                    ))
                })
        })
        .collect::<SafeNativeResult<Vec<Value>>>()?;
    Ok(smallvec![Value::vector_unchecked(events)?])
}

#[inline]
fn native_write_module_event_to_store(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 1);

    let ty = &ty_args[0];
    let msg = arguments.pop_back().unwrap();

    context.charge(
        EVENT_WRITE_TO_EVENT_STORE_BASE
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg)?,
    )?;

    let type_tag = context.type_to_type_tag(ty)?;

    // Additional runtime check for module call.
    let stack_frames = context.stack_frames(1);
    let id = stack_frames
        .stack_trace()
        .first()
        .map(|(caller, _, _)| caller)
        .ok_or_else(|| {
            let err = PartialVMError::new_invariant_violation(
                "Caller frame for 0x1::emit::event is not found",
            );
            SafeNativeError::InvariantViolation(err)
        })?
        .as_ref()
        .ok_or_else(|| {
            // If module is not known, this call must come from the script, which is not allowed.
            let err = PartialVMError::new_invariant_violation("Scripts cannot emit events");
            SafeNativeError::InvariantViolation(err)
        })?;

    if let TypeTag::Struct(ref struct_tag) = type_tag {
        if id != &struct_tag.module_id() {
            return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
                StatusCode::INTERNAL_TYPE_ERROR,
            )));
        }
    } else {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::INTERNAL_TYPE_ERROR,
        )));
    }

    let (layout, contains_delayed_fields) = context
        .type_to_type_layout_with_delayed_fields(ty)?
        .unpack();

    let function_value_extension = context.function_value_extension();
    let max_value_nest_depth = context.max_value_nest_depth();
    let blob = ValueSerDeContext::new(max_value_nest_depth)
        .with_delayed_fields_serde()
        .with_func_args_deserialization(&function_value_extension)
        .serialize(&msg, &layout)?
        .ok_or_else(|| {
            SafeNativeError::InvariantViolation(PartialVMError::new_invariant_violation(
                "Event serialization failure",
            ))
        })?;

    let ctx = context.extensions_mut().get_mut::<NativeEventContext>();
    let event = ContractEvent::new_v2(type_tag, blob).map_err(|_| {
        SafeNativeError::abort_with_message(
            ECANNOT_CREATE_EVENT,
            "Event v2 size is not computable: type tag may be invalid or too complex",
        )
    })?;
    // TODO(layouts): avoid cloning layouts for events with delayed fields.
    ctx.events.push((
        event,
        contains_delayed_fields.then(|| layout.as_ref().clone()),
    ));

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    #[cfg(feature = "testing")]
    natives.extend([(
        "emitted_events_by_handle",
        native_emitted_events_by_handle as RawSafeNative,
    )]);

    #[cfg(feature = "testing")]
    natives.extend([("emitted_events", native_emitted_events as RawSafeNative)]);

    natives.extend([(
        "write_to_event_store",
        native_write_to_event_store as RawSafeNative,
    )]);

    natives.extend([(
        "write_module_event_to_store",
        native_write_module_event_to_store as RawSafeNative,
    )]);

    builder.make_named_natives(natives)
}
