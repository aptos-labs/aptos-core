// Copyright © Aptos Foundation

use aptos_evm::{engine::Engine, eth_address::EthAddress, utils::vec_to_h160};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_table_natives::{NativeTableContext, TableHandle};
use aptos_types::account_address::AccountAddress;
use aptos_types::vm_status::StatusCode;
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

use crate::natives::evm::NativeEvmContext;

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
* public native fun native fun create_impl(nonce: Table, balance: Table, code: Table, storage: Table, pub_keys: Table, caller: vec<u8>, payload: vector<u8>, signature: vector<u8>);
***************************************************************************************************/

fn native_create_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);
    context.charge(EVM_CREATE_BASE)?;

    let signature = safely_pop_arg!(args, Vec<u8>);
    let payload = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
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

fn native_call_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 5);
    context.charge(EVM_CALL_BASE)?;

    let signature = safely_pop_arg!(args, Vec<u8>);
    let payload = safely_pop_arg!(args, Vec<u8>);
    let caller = safely_pop_arg!(args, Vec<u8>);
    let pub_keys_table_handle = get_handle(&safely_pop_arg!(args, StructRef))?;
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
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("create_impl", native_create_impl as RawSafeNative),
        ("call_impl", native_call_impl),
    ];
    builder.make_named_natives(natives)
}
