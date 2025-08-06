// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_move_stdlib::natives::all_natives;
use aptos_native_interface::SafeNativeBuilder;
use aptos_table_natives::NativeTableContext;
use aptos_types::on_chain_config::{Features, TimedFeaturesBuilder};
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::Identifier, language_storage::ModuleId,
};
use move_ir_compiler::Compiler;
use move_vm_runtime::{
    data_cache::TransactionDataCache, dispatch_loader, module_traversal::*, move_vm::MoveVM,
    native_extensions::NativeContextExtensions, native_functions::NativeFunction,
    AsUnsyncCodeStorage, InstantiatedFunctionLoader, LegacyLoaderConfig, RuntimeEnvironment,
    ScriptLoader,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{
    gas::UnmeteredGasMeter, loaded_data::runtime_types::Type, natives::function::NativeResult,
    pop_arg, values::Value,
};
use smallvec::smallvec;
use std::{collections::VecDeque, env, fs, sync::Arc};

/// For profiling, we can use scripts or "run" entry functions.
enum Entrypoint {
    Module(ModuleId),
    Script(Vec<u8>),
}

fn make_native_create_signer() -> NativeFunction {
    Arc::new(|_context, ty_args: Vec<Type>, mut args: VecDeque<Value>| {
        assert!(ty_args.is_empty());
        assert_eq!(args.len(), 1);

        let address = pop_arg!(args, AccountAddress);

        Ok(NativeResult::ok(0.into(), smallvec![Value::master_signer(
            address
        )]))
    })
}

fn compile_test_modules() -> Vec<CompiledModule> {
    let module_sources = [
        r#"
            module 0x1.Test {
                native public create_signer(addr: address): signer;
            }
        "#,
        r#"
            module 0x1.bcs {
                native public to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            }
        "#,
        r#"
            module 0x1.hash {
                native public sha2_256(data: vector<u8>): vector<u8>;
                native public sha3_256(data: vector<u8>): vector<u8>;
            }
        "#,
        r#"
            module 0x1.table {
                struct Table<phantom K: copy + drop, phantom V> has store {
                    handle: address,
                    length: u64,
                }

                struct Box<V> has key, drop, store {
                    val: V
                }

                native new_table_handle<K, V>(): address;
                native add_box<K: copy + drop, V, B>(table: &mut Self.Table<K, V>, key: K, val: Self.Box<V>);
                native borrow_box<K: copy + drop, V, B>(table: &Self.Table<K, V>, key: K): &Self.Box<V>;
                native borrow_box_mut<K: copy + drop, V, B>(table: &mut Self.Table<K, V>, key: K): &mut Self.Box<V>;
                native contains_box<K: copy + drop, V, B>(table: &Self.Table<K, V>, key: K): bool;
                native remove_box<K: copy + drop, V, B>(table: &mut Self.Table<K, V>, key: K): Self.Box<V>;
                native destroy_empty_box<K: copy + drop, V, B>(table: &Self.Table<K, V>);
                native drop_unchecked_box<K: copy + drop, V, B>(table: Self.Table<K, V>);

                public new<K: copy + drop, V: store>(): Self.Table<K, V> {
                label b0:
                    return Table<K, V> {
                        handle: Self.new_table_handle<K, V>(),
                        length: 0,
                    };
                }

                public destroy_empty<K: copy + drop, V>(table: Self.Table<K, V>) {
                label b0:
                    Self.destroy_empty_box<K, V, Self.Box<V>>(&table);
                    Self.drop_unchecked_box<K, V, Self.Box<V>>(move(table));
                    return;
                }

                public add<K: copy + drop, V>(table: &mut Self.Table<K, V>, key: K, val: V) {
                    let b: Self.Box<V>;
                label b0:
                    b = Box<V> { val: move(val) };
                    Self.add_box<K, V, Self.Box<V>>(move(table), move(key), move(b));
                    return;
                }

                public borrow<K: copy + drop, V>(table: &Self.Table<K, V>, key: K): &V {
                label b0:
                    return &Self.borrow_box<K, V, Self.Box<V>>(move(table), move(key)).Box<V>::val;
                }

                public contains<K: copy + drop, V>(table: &Self.Table<K, V>, key: K): bool {
                label b0:
                    return Self.contains_box<K, V, Self.Box<V>>(move(table), move(key));
                }

                public remove<K: copy + drop, V>(table: &mut Self.Table<K, V>, key: K): V {
                    let v: V;
                label b0:
                    Box<V> { v } = Self.remove_box<K, V, Self.Box<V>>(move(table), move(key));
                    return move(v);
                }
            }
        "#,
    ];

    module_sources
        .into_iter()
        .map(|src| Compiler::new(vec![]).into_compiled_module(src).unwrap())
        .collect()
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        bail!("Wrong number of arguments.")
    }

    let mut builder = SafeNativeBuilder::new(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
        None,
    );

    let stdlib_addr = AccountAddress::from_hex_literal("0x1").unwrap();
    let mut natives = all_natives(stdlib_addr, &mut builder);
    natives.push((
        stdlib_addr,
        Identifier::new("Test").unwrap(),
        Identifier::new("create_signer").unwrap(),
        make_native_create_signer(),
    ));
    natives.extend(aptos_table_natives::table_natives(
        stdlib_addr,
        &mut builder,
    ));

    let runtime_environment = RuntimeEnvironment::new(natives);
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let test_modules = compile_test_modules();
    for module in &test_modules {
        let mut blob = vec![];
        module.serialize(&mut blob).unwrap();
        storage.add_module_bytes(module.self_addr(), module.self_name(), blob.into())
    }

    let src = fs::read_to_string(&args[1])?;
    let entrypoint = if let Ok(script_blob) =
        Compiler::new(test_modules.iter().collect()).into_script_blob(&src)
    {
        Entrypoint::Script(script_blob)
    } else {
        let module = Compiler::new(test_modules.iter().collect()).into_compiled_module(&src)?;
        let mut module_blob = vec![];
        module.serialize(&mut module_blob)?;
        storage.add_module_bytes(module.self_addr(), module.self_name(), module_blob.into());
        Entrypoint::Module(module.self_id())
    };

    let mut extensions = NativeContextExtensions::default();
    extensions.add(NativeTableContext::new([0; 32], &storage));

    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    let code_storage = storage.as_unsync_code_storage();

    let return_values = dispatch_loader!(&code_storage, loader, {
        // There was no charging for loading scripts or functions here prior to lazy loading.
        let legacy_loader_config = LegacyLoaderConfig::unmetered();

        let func = match &entrypoint {
            Entrypoint::Script(script_blob) => loader.load_script(
                &legacy_loader_config,
                &mut gas_meter,
                &mut traversal_context,
                script_blob,
                &[],
            )?,
            Entrypoint::Module(module_id) => loader.load_instantiated_function(
                &legacy_loader_config,
                &mut gas_meter,
                &mut traversal_context,
                module_id,
                ident_str!("run"),
                &[],
            )?,
        };

        MoveVM::execute_loaded_function(
            func,
            // No arguments.
            Vec::<Vec<u8>>::new(),
            &mut TransactionDataCache::empty(
                Features::default().is_lightweight_resource_existence_enabled(),
            ),
            &mut gas_meter,
            &mut traversal_context,
            &mut extensions,
            &loader,
            &storage,
        )
    })?;
    println!("{:?}", return_values);

    Ok(())
}
