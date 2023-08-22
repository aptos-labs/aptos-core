// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
#[cfg(feature = "testing")]
use move_binary_format::errors::PartialVMError;
use move_core_types::account_address::AccountAddress;
#[cfg(feature = "testing")]
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
#[cfg(feature = "testing")]
use move_vm_types::values::{Reference, Struct, StructRef};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use serde::Serialize;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

#[derive(Serialize)]
pub struct GUID {
    creation_num: u64,
    addr: AccountAddress,
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

    if !context.save_event(guid, seq_num, ty, msg)? {
        return Err(SafeNativeError::Abort { abort_code: 0 });
    }

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
        .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?
        .value_as::<u64>()?;
    let addr = guid
        .next()
        .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?
        .value_as::<AccountAddress>()?;
    let guid = GUID { creation_num, addr };
    let events = context.emitted_events(
        bcs::to_bytes(&guid)
            .map_err(|_| PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR))?,
        ty,
    )?;
    Ok(smallvec![Value::vector_for_testing_only(events)])
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

    builder.make_named_natives(natives)
}
