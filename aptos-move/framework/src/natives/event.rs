// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
    native_extensions::VersionControlledNativeExtension, native_functions::NativeFunction,
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

#[derive(Default, Tid)]
pub struct NativeEventContext {
    /// Cached emitted module events.
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    /// Checkpoints for events containing saved lengths of event vector. In case some effects needs
    /// to be undone, the latest length can be used to truncate the event vector.
    checkpoints: Vec<usize>,
}

impl VersionControlledNativeExtension for NativeEventContext {
    fn undo(&mut self) {
        if let Some(saved_len) = self.checkpoints.pop() {
            self.events.truncate(saved_len)
        }
    }

    fn save(&mut self) {
        self.checkpoints.push(self.events.len());
    }

    fn update(&mut self, _txn_hash: &[u8; 32], _script_hash: &[u8]) {
        // No-op: nothing needs to be updated.
    }
}

impl NativeEventContext {
    pub fn into_events(self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        assert!(
            self.checkpoints.is_empty(),
            "Events should not be saved when consumed"
        );
        self.events
    }

    /// Returns iterator over all events seen so far.
    pub fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.events.iter().map(|(event, _)| event)
    }

    /// Extracts all events. Leaves events saved from the latest checkpoint if any.
    pub fn take_events(&mut self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        if let Some(saved_len) = self.checkpoints.last() {
            // In case we have some previous events saved, we need to keep them.
            let saved_events = self
                .events
                .iter()
                .take(*saved_len)
                .cloned()
                .collect::<Vec<_>>();
            let all_events = std::mem::take(&mut self.events);
            self.events.extend(saved_events);
            all_events
        } else {
            std::mem::take(&mut self.events)
        }
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
    mut ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 3);

    let ty = ty_args.pop().unwrap();
    let msg = arguments.pop_back().unwrap();
    let seq_num = safely_pop_arg!(arguments, u64);
    let guid = safely_pop_arg!(arguments, Vec<u8>);

    // TODO(Gas): Get rid of abstract memory size
    context.charge(
        EVENT_WRITE_TO_EVENT_STORE_BASE
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg)?,
    )?;
    let ty_tag = context.type_to_type_tag(&ty)?;
    let (layout, has_aggregator_lifting) =
        context.type_to_type_layout_with_identifier_mappings(&ty)?;

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
    let event =
        ContractEvent::new_v1(key, seq_num, ty_tag, blob).map_err(|_| SafeNativeError::Abort {
            abort_code: ECANNOT_CREATE_EVENT,
        })?;
    ctx.events
        .push((event, has_aggregator_lifting.then_some(layout)));
    Ok(smallvec![])
}

#[cfg(feature = "testing")]
fn native_emitted_events_by_handle(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 1);

    let ty = ty_args.pop().unwrap();
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
    let ty_tag = context.type_to_type_tag(&ty)?;
    let ty_layout = context.type_to_type_layout(&ty)?;
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
    Ok(smallvec![Value::vector_for_testing_only(events)])
}

#[cfg(feature = "testing")]
fn native_emitted_events(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    let ty = ty_args.pop().unwrap();

    let ty_tag = context.type_to_type_tag(&ty)?;
    let ty_layout = context.type_to_type_layout(&ty)?;
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
    Ok(smallvec![Value::vector_for_testing_only(events)])
}

#[inline]
fn native_write_module_event_to_store(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 1);

    let ty = ty_args.pop().unwrap();
    let msg = arguments.pop_back().unwrap();

    context.charge(
        EVENT_WRITE_TO_EVENT_STORE_BASE
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg)?,
    )?;

    let type_tag = context.type_to_type_tag(&ty)?;

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

    let (layout, has_identifier_mappings) =
        context.type_to_type_layout_with_identifier_mappings(&ty)?;

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
    let event = ContractEvent::new_v2(type_tag, blob).map_err(|_| SafeNativeError::Abort {
        abort_code: ECANNOT_CREATE_EVENT,
    })?;
    ctx.events
        .push((event, has_identifier_mappings.then_some(layout)));

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
