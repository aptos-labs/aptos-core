// Copyright © Aptos Foundation

use crate::natives::evm_natives::NativeEvmContext;
use aptos_evm::{engine::Engine, eth_address::EthAddress, utils::vec_to_h160};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_table_natives::{NativeTableContext, TableHandle};
use aptos_types::{account_address::AccountAddress, vm_status::StatusCode};
use evm_core::ExitReason;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, StructRef, Value},
};
use primitive_types::U256;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// The index of the `handle` field in the `Table` Move struct.
const TABLE_HANDLE_FIELD_INDEX: usize = 0;

pub(crate) fn get_handle(table_data: &StructRef) -> PartialVMResult<TableHandle> {
    Ok(TableHandle(
        table_data
            .borrow_field(TABLE_HANDLE_FIELD_INDEX)?
            .value_as::<Reference>()?
            .read_ref()?
            .value_as::<AccountAddress>()?,
    ))
}

/***************************************************************************************************
* public native fun native fun create_impl2(nonce: Table, balance: Table, code: Table, storage: Table, pub_keys: Table, caller: vec<u8>, payload: vector<u8>, signature: vector<u8>);
***************************************************************************************************/

fn native_create_impl2(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 9);
    context.charge(EVM_CREATE_BASE)?;
    let gas_limit = safely_pop_arg!(args, u64);
    let init_code = safely_pop_arg!(args, Vec<u8>);
    let value = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let caller = vec_to_h160(&caller);
    let value = U256::from_big_endian(&value);

    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let table_context = context.extensions().get::<NativeTableContext>();
    let evm_context = context.extensions().get::<NativeEvmContext>();

    let mut engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output, mut change_set) =
        engine.transact_create(caller, value, init_code, gas_limit, [].to_vec());
    evm_context
        .table_change_set
        .borrow_mut()
        .new_tables
        .append(&mut change_set.new_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .removed_tables
        .append(&mut change_set.removed_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .changes
        .append(&mut change_set.changes);

    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}

fn native_call_impl2(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 10);
    context.charge(EVM_CREATE_BASE)?;
    let gas_limit = safely_pop_arg!(args, u64);
    let data = safely_pop_arg!(args, Vec<u8>);
    let value = safely_pop_arg!(args, Vec<u8>);
    let address = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let caller = vec_to_h160(&caller);
    let value = U256::from_big_endian(&value);
    let address = vec_to_h160(&address);

    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let table_context = context.extensions().get::<NativeTableContext>();
    let evm_context = context.extensions().get::<NativeEvmContext>();
    let mut engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output, mut change_set) =
        engine.transact_call(caller, address, value, data, gas_limit, [].to_vec());
    evm_context
        .table_change_set
        .borrow_mut()
        .new_tables
        .append(&mut change_set.new_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .removed_tables
        .append(&mut change_set.removed_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .changes
        .append(&mut change_set.changes);
    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}

/***************************************************************************************************
* public native fun native fun create_impl(nonce: Table, balance: Table, code: Table, storage: Table, pub_keys: Table, caller: vec<u8>, payload: vector<u8>, signature: vector<u8>);
***************************************************************************************************/

fn _native_create_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);
    context.charge(EVM_CREATE_BASE)?;

    let _signature = safely_pop_arg!(args, Vec<u8>);
    let _payload = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    // let deserialized_payload = payload.deserialize();
    let caller = vec_to_h160(&caller);

    let gas_limit = 5;
    let value = U256::zero();
    let init_code = [].to_vec();

    let table_context = context.extensions().get::<NativeTableContext>();
    let evm_context = context.extensions().get::<NativeEvmContext>();
    let mut engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output, mut change_set) =
        engine.transact_create(caller, value, init_code, gas_limit, [].to_vec());
    evm_context
        .table_change_set
        .borrow_mut()
        .new_tables
        .append(&mut change_set.new_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .removed_tables
        .append(&mut change_set.removed_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .changes
        .append(&mut change_set.changes);
    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}

/***************************************************************************************************
* public native fun call_impl(nonce: Table, balance: Table, code: Table, storage: Table, pub_keys: Table, caller: U256, payload: Vec<u8>, signature: Vec<u8>);
***************************************************************************************************/

fn _native_call_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 5);
    context.charge(EVM_CALL_BASE)?;

    let _signature = safely_pop_arg!(args, Vec<u8>);
    let _payload = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    // let deserialized_data = payload.deserialize();
    let caller = vec_to_h160(&caller);

    let gas_limit = 5;
    let value = U256::zero();
    let data = [].to_vec();
    let address = caller;

    let table_context = context.extensions().get::<NativeTableContext>();
    let evm_context = context.extensions().get::<NativeEvmContext>();
    let mut engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output, mut change_set) =
        engine.transact_call(caller, address, value, data, gas_limit, [].to_vec());
    evm_context
        .table_change_set
        .borrow_mut()
        .new_tables
        .append(&mut change_set.new_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .removed_tables
        .append(&mut change_set.removed_tables);
    evm_context
        .table_change_set
        .borrow_mut()
        .changes
        .append(&mut change_set.changes);
    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}

/***************************************************************************************************
* public native fun view_impl(nonce: Table, balance: Table, code: Table, storage: Table, pub_keys: Table, caller: U256, payload: Vec<u8>, signature: Vec<u8>);
***************************************************************************************************/

fn _native_view_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 8);
    context.charge(EVM_VIEW_BASE)?;

    let _signature = safely_pop_arg!(args, Vec<u8>);
    let _payload = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    // let deserialized_data = payload.deserialize();
    let caller = vec_to_h160(&caller);

    let value = U256::zero();
    let data = [].to_vec();
    let address = caller;

    let table_context = context.extensions().get::<NativeTableContext>();
    let engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output) = engine.view(caller, address, value, data);
    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}

fn native_view_impl2(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 9);
    context.charge(EVM_VIEW_BASE)?;

    let data = safely_pop_arg!(args, Vec<u8>);
    let value = safely_pop_arg!(args, Vec<u8>);
    let address = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let _pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let storage_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let code_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let balance_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
    let nonce_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;

    let caller = vec_to_h160(&caller);
    let value = U256::from_big_endian(&value);
    let address = vec_to_h160(&address);

    let table_context = context.extensions().get::<NativeTableContext>();
    let engine = Engine::new(
        table_context.resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(caller),
    );
    let (exit_reason, output) = engine.view(caller, address, value, data);
    match exit_reason {
        ExitReason::Succeed(_) => Ok(smallvec![Value::vector_u8(output)]),
        ExitReason::Error(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned an error".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Revert(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM reverted".to_string())
            .with_sub_status(0x03_0002)
            .into()),
        ExitReason::Fatal(_) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("EVM returned fatal".to_string())
            .with_sub_status(0x03_0003)
            .into()),
    }
}
/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("create_impl2", native_create_impl2 as RawSafeNative),
        ("call_impl2", native_call_impl2),
        ("view_impl2", native_view_impl2),
    ];
    builder.make_named_natives(natives)
}
