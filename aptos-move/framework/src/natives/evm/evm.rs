use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;
use aptos_vm::evm::engine::Engine;

/***************************************************************************************************
* public native fun create(caller: H160, value: u256, init_code: Vec<u8>, gas_limit: u64) -> Vec<u8>;
***************************************************************************************************/

fn native_create(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);
    context.charge(EVM_CREATE_BASE)?;
    
    let gas_limit = safely_pop_arg!(args, u64);
    let init_code = safely_pop_arg!(args, Vec<u8>);
    let value = safely_pop_arg!(args, u256);
    let caller = safely_pop_arg!(args, H160);

    let evm_context = context.extensions().get::<NativeEvmContext>();
    let engine = Engine::new(
        evm_context.resolver,
        evm_context.nonce_table_handle,
        evm_context.balance_table_handle,
        evm_context.code_table_handle,
        evm_context.storage_table_handle,
        evm_context.origin
    );
    let (exit_reason, output) = engine.transact_create(caller, value, init_code, gas_limit);
}

/***************************************************************************************************
* public native fun call(caller: H160, address: H160, value: u256, data: Vec<u8>, gas_limit: u64);
***************************************************************************************************/

fn native_call(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 5);
    context.charge(EVM_CALL_BASE)?;

    let gas_limit = safely_pop_arg!(args, u64);
    let data = safely_pop_arg!(args, Vec<u8>);
    let value = safely_pop_arg!(args, u256);
    let address = safely_pop_arg!(args, H160);
    let caller = safely_pop_arg!(args, H160);

    let evm_context = context.extensions().get::<NativeEvmContext>();
    let engine = Engine::new(
        evm_context.resolver,
        evm_context.nonce_table_handle,
        evm_context.balance_table_handle,
        evm_context.code_table_handle,
        evm_context.storage_table_handle,
        evm_context.origin
    );
    let (exit_reason, output) = engine.transact_call(caller, address, value, data, gas_limit);
    
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
 pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("create", native_create as RawSafeNative),
        ("call", native_call),
    ];

    builder.make_named_natives(natives)
}