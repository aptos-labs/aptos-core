// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::get_metadata;
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
use ark_std::iterable::Iterable;
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
#[cfg(feature = "testing")]
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    resolver::MoveResolver,
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
#[cfg(feature = "testing")]
use move_vm_types::values::{Reference, Struct, StructRef};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Cached emitted module events.
#[derive(Tid)]
pub struct NativeEventContext {
    events: Vec<ContractEvent>,
}

impl NativeEventContext {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn into_events(self) -> Vec<ContractEvent> {
        self.events
    }

    #[cfg(feature = "testing")]
    fn emitted_v0_events(
        &self,
        event_key: &EventKey,
        ty_tag: &TypeTag,
    ) -> PartialVMResult<Vec<&[u8]>> {
        let mut events = vec![];
        for event in self.events.iter() {
            if let ContractEvent::V1(e) = event {
                if e.key() == event_key && e.type_tag() == ty_tag {
                    events.push(e.event_data());
                }
            }
        }
        Ok(events)
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
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg),
    )?;

    let ty_tag = context.type_to_type_tag(&ty)?;
    let ty_layout = context.type_to_type_layout(&ty)?;
    let blob = msg.simple_serialize(&ty_layout).ok_or_else(|| {
        SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::VALUE_DESERIALIZATION_ERROR,
        ))
    })?;
    let key = bcs::from_bytes(guid.as_slice()).map_err(|_| {
        SafeNativeError::InvariantViolation(PartialVMError::new(StatusCode::EVENT_KEY_MISMATCH))
    })?;

    let ctx = context.extensions_mut().get_mut::<NativeEventContext>();
    ctx.events
        .push(ContractEvent::new_v1(key, seq_num, ty_tag, blob));
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
    let ctx = context.extensions_mut().get_mut::<NativeEventContext>();
    let events = ctx
        .emitted_v0_events(&key, &ty_tag)?
        .into_iter()
        .map(|blob| {
            Value::simple_deserialize(blob, &ty_layout).ok_or_else(|| {
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
            + EVENT_WRITE_TO_EVENT_STORE_PER_ABSTRACT_VALUE_UNIT * context.abs_val_size(&msg),
    )?;

    let type_tag = context.type_to_type_tag(&ty)?;

    // Maybe not necessary but just in case
    let struct_tag = match type_tag {
        TypeTag::Struct(ref struct_tag) => Ok(struct_tag),
        _ => Err(SafeNativeError::Abort {
            // not an struct type
            abort_code: 0x10001,
        }),
    }?;
    let layout = context.type_to_type_layout(&ty)?;
    let blob = msg.simple_serialize(&layout).ok_or_else(|| {
        SafeNativeError::InvariantViolation(
            PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                .with_message("Event serialization failure".to_string()),
        )
    })?;
    let ctx = context.extensions_mut().get_mut::<NativeEventContext>();
    ctx.events.push(ContractEvent::new_v2(type_tag, blob));

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

    natives.extend([(
        "write_to_event_store",
        native_write_to_event_store as RawSafeNative,
    )]);

    natives.extend([(
        "write_to_module_event_store",
        native_write_module_event_to_store as RawSafeNative,
    )]);

    builder.make_named_natives(natives)
}
