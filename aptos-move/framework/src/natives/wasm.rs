// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Reference, StructRef, Value, Vector, VectorRef},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};
use wasmtime::*;

/// Abort code when from_bytes fails (0x01 == INVALID_ARGUMENT)
const EFROM_BYTES: u64 = 0x01_0001;

/***************************************************************************************************
 * native fun validate_and_annotate_wasm_bytecode
 *
 *   gas cost: TBD
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct ValidateWASMGasParameters();

fn native_validate_and_annotate_wasm_bytecode(
    _gas_params: &ValidateWASMGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);

    // TODO(Gas): charge for getting the layout
    let bytes = pop_arg!(args, Vec<u8>);

    Ok(NativeResult::ok(
        InternalGas::zero(),
        smallvec![Value::vector_u8(bytes)],
    ))
}

pub fn make_validate_and_annotate_wasm_bytecode(
    gas_params: ValidateWASMGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_validate_and_annotate_wasm_bytecode(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun execute_bytecode
 *
 *   gas cost: TBD
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct ExecuteWASMGasParameters();

fn native_execute_wasm_bytecode(
    _gas_params: &ExecuteWASMGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);

    // let table_context = context.extensions().get::<NativeTableContext>();

    let r_arg = pop_arg!(args, VectorRef);
    let r_ref = r_arg.as_bytes_ref();

    // Hook the table handle with WASM runtime later.
    let _ = pop_arg!(args, StructRef);

    let func_args = pop_arg!(args, Vector).to_vec_u8()?;
    let is_mutable = pop_arg!(args, bool);

    let result = execute_function(r_ref.as_ref(), func_args, is_mutable).unwrap();

    Ok(NativeResult::ok(
        InternalGas::zero(),
        smallvec![Value::vector_u8(result.into_iter())],
    ))
}

fn execute_function(
    module_bytes: &[u8],
    args: Vec<u8>,
    _is_mutable: bool,
) -> anyhow::Result<Vec<u8>> {
    let engine = Engine::default();
    let module = Module::new(&engine, module_bytes).unwrap();
    let mut store = Store::new(&engine, 4);
    let memory_ty = MemoryType::new(1024, None);
    let memory = Memory::new(&mut store, memory_ty)?;
    memory.write(&mut store, 0, args.as_ref())?;

    let linker = Linker::new(&engine);

    let instance = linker.instantiate(&mut store, &module)?;
    instance
        .get_global(&mut store, "INPUT_PARAMS")
        .unwrap()
        .set(&mut store, 0, args.as_ref())?;
    let hello = instance.get_typed_func::<(), (), _>(&mut store, "entry")?;

    // And finally we can call the wasm!
    hello.call(&mut store, ())?;
    let mut output = vec![];
    instance
        .get_memory(&mut store, "OUTPUT_PARAMS")
        .unwrap()
        .read(&mut store, 0, &mut output)?;

    Ok(output)
}

pub fn make_execute_wasm_bytecode(gas_params: ExecuteWASMGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_execute_wasm_bytecode(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub validate_wasm: ValidateWASMGasParameters,
    pub execute: ExecuteWASMGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "validate_and_annotate_wasm_bytecode",
            make_validate_and_annotate_wasm_bytecode(gas_params.validate_wasm),
        ),
        (
            "execute_bytecode",
            make_execute_wasm_bytecode(gas_params.execute),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}

#[test]
fn test_wasm_execute() {
    let module_wat = r#"
    (module
        (type $t0 (func))
        (func $entry (export "entry") (type $t0)
          (i32.store8
            (i32.const 1048578)
            (i32.add
              (i32.load8_u
                (i32.const 1048577))
              (i32.load8_u
                (i32.const 1048576)))))
        (memory $memory (export "memory") 17)
        (global $INPUT_PARAMS (export "INPUT_PARAMS") i32 (i32.const 1048576))
        (global $OUTPUT_PARAMS (export "OUTPUT_PARAMS") i32 (i32.const 1048578)))
    "#;

    let engine = Engine::default();
    let module = Module::new(&engine, &module_wat).unwrap();
    let bytes = module.serialize().unwrap();
    println!(
        "{:?}",
        execute_function(module_wat.as_bytes(), vec![0u8, 1u8], true)
    );
}
