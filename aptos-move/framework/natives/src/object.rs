// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use aptos_types::transaction::authenticator::AuthenticationKey;
use better_any::{Tid, TidAble};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, InternalGasPerByte},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::PartialVMError, values::Value,
};
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

/// Cached emitted module events.
#[derive(Default, Tid)]
pub struct NativeObjectContext {
    // TODO - if further optimizations is important, we can consider if:
    //   - caching all (or just some derive_from) locations is useful
    //   - if it is faster to use BTreeMap or HashMap, given the lenghts of the addresses
    //   - if it is worth moving to native/caching other address deriving as well
    derived_from_object_addresses:
        RefCell<HashMap<(AccountAddress, AccountAddress), AccountAddress>>,
}

impl SessionListener for NativeObjectContext {
    fn start(&mut self, _session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        // It is safe to persist derived addresses caches because they are only saving compute,
        // there is no behavior change even if they are cached between prologue, user session or
        // epilogue. Hence, on new session start we do not need to reset anything.
    }

    fn finish(&mut self) {
        // No state changes to save.
    }

    fn abort(&mut self) {
        // No state changes to abort.
    }
}

impl NativeRuntimeRefCheckModelsCompleted for NativeObjectContext {
    // No native functions in this context return references, so no models to add.
}

/***************************************************************************************************
 * native exists_at<T>
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct ExistsAtGasParameters {
    pub base: InternalGas,
    pub per_byte_loaded: InternalGasPerByte,
    pub per_item_loaded: InternalGas,
}

fn native_exists_at(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 1);
    safely_assert_eq!(args.len(), 1);

    let type_ = &ty_args[0];
    let address = safely_pop_arg!(args, AccountAddress);

    context.charge(OBJECT_EXISTS_AT_BASE)?;

    let (exists, num_bytes) = context.exists_at(address, type_).map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
            "Failed to read resource: {:?} at {}. With error: {}",
            type_, address, err
        ))
    })?;

    if let Some(num_bytes) = num_bytes {
        context.charge(
            OBJECT_EXISTS_AT_PER_ITEM_LOADED + OBJECT_EXISTS_AT_PER_BYTE_LOADED * num_bytes,
        )?;
    }

    Ok(smallvec![Value::bool(exists)])
}

/***************************************************************************************************
 * native fun create_user_derived_object_address_impl
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_create_user_derived_object_address_impl(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.len() == 2);

    context.charge(OBJECT_USER_DERIVED_ADDRESS_BASE)?;

    let object_context = context.extensions().get::<NativeObjectContext>();
    let derive_from = safely_pop_arg!(args, AccountAddress);
    let source = safely_pop_arg!(args, AccountAddress);

    let derived_address = *object_context
        .derived_from_object_addresses
        .borrow_mut()
        .entry((derive_from, source))
        .or_insert_with(|| {
            AuthenticationKey::object_address_from_object(&source, &derive_from).account_address()
        });

    Ok(smallvec![Value::address(derived_address)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("exists_at", native_exists_at as RawSafeNative),
        (
            "create_user_derived_object_address_impl",
            native_create_user_derived_object_address_impl,
        ),
    ];

    builder.make_named_natives(natives)
}
