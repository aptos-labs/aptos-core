// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{bail, Result};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_move_stdlib::natives::all_natives;
use aptos_native_interface::SafeNativeBuilder;
use aptos_table_natives::NativeTableContext;
use aptos_types::on_chain_config::{Features, TimedFeaturesBuilder};
use move_asm::assembler;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::Identifier, language_storage::ModuleId,
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    module_traversal::*,
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    native_functions::NativeFunction,
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
    Arc::new(|_context, ty_args: &[Type], mut args: VecDeque<Value>| {
        assert!(ty_args.is_empty());
        assert_eq!(args.len(), 1);

        let address = pop_arg!(args, AccountAddress);

        Ok(NativeResult::ok(0.into(), smallvec![Value::master_signer(
            address
        )]))
    })
}

fn dedent(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l.trim()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assemble_module(src: &str) -> CompiledModule {
    let options = assembler::Options::default();
    let src = dedent(src);
    let src = src.trim();
    assembler::assemble(&options, src, std::iter::empty())
        .unwrap_or_else(|diags| {
            panic!(
                "failed to assemble module: {}",
                assembler::diag_to_string("dep", src, diags)
            )
        })
        .left()
        .expect("expected module, got script")
}

fn compile_test_modules() -> Vec<CompiledModule> {
    let module_sources = [
        r#"
            module 0x1::Test
            public native fun create_signer(l0: address): signer
        "#,
        r#"
            module 0x1::bcs
            public native fun to_bytes<T0>(l0: &T0): vector<u8>
        "#,
        r#"
            module 0x1::hash
            public native fun sha2_256(l0: vector<u8>): vector<u8>
            public native fun sha3_256(l0: vector<u8>): vector<u8>
        "#,
        r#"
            module 0x1::table
            struct Table<phantom T0: copy + drop, phantom T1> has store
              handle: address
              length: u64

            struct Box<T0> has drop + store + key
              val: T0

            native fun new_table_handle<T0, T1>(): address

            native fun add_box<T0: copy + drop, T1, T2>(l0: &mut Table<T0, T1>, l1: T0, l2: Box<T1>)

            native fun borrow_box<T0: copy + drop, T1, T2>(l0: &Table<T0, T1>, l1: T0): &Box<T1>

            native fun borrow_box_mut<T0: copy + drop, T1, T2>(l0: &mut Table<T0, T1>, l1: T0): &mut Box<T1>

            native fun contains_box<T0: copy + drop, T1, T2>(l0: &Table<T0, T1>, l1: T0): bool

            native fun remove_box<T0: copy + drop, T1, T2>(l0: &mut Table<T0, T1>, l1: T0): Box<T1>

            native fun destroy_empty_box<T0: copy + drop, T1, T2>(l0: &Table<T0, T1>)

            native fun drop_unchecked_box<T0: copy + drop, T1, T2>(l0: Table<T0, T1>)

            public fun new<T0: copy + drop, T1: store>(): Table<T0, T1>
                call new_table_handle<T0, T1>
                ld_u64 0
                pack Table<T0, T1>
                ret

            public fun destroy_empty<T0: copy + drop, T1>(l0: Table<T0, T1>)
                borrow_loc l0
                call destroy_empty_box<T0, T1, Box<T1>>
                move_loc l0
                call drop_unchecked_box<T0, T1, Box<T1>>
                ret

            public fun add<T0: copy + drop, T1>(l0: &mut Table<T0, T1>, l1: T0, l2: T1)
                local l3: Box<T1>
                move_loc l2
                pack Box<T1>
                st_loc l3
                move_loc l0
                move_loc l1
                move_loc l3
                call add_box<T0, T1, Box<T1>>
                ret

            public fun borrow<T0: copy + drop, T1>(l0: &Table<T0, T1>, l1: T0): &T1
                move_loc l0
                move_loc l1
                call borrow_box<T0, T1, Box<T1>>
                borrow_field Box<T1>, val
                ret

            public fun contains<T0: copy + drop, T1>(l0: &Table<T0, T1>, l1: T0): bool
                move_loc l0
                move_loc l1
                call contains_box<T0, T1, Box<T1>>
                ret

            public fun remove<T0: copy + drop, T1>(l0: &mut Table<T0, T1>, l1: T0): T1
                local l2: T1
                move_loc l0
                move_loc l1
                call remove_box<T0, T1, Box<T1>>
                unpack Box<T1>
                st_loc l2
                move_loc l2
                ret
        "#,
    ];

    module_sources.into_iter().map(assemble_module).collect()
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
    let options = assembler::Options::default();
    let assembled =
        assembler::assemble(&options, &src, test_modules.iter()).unwrap_or_else(|diags| {
            panic!(
                "failed to assemble: {}",
                assembler::diag_to_string(&args[1], &src, diags)
            )
        });
    let entrypoint = match assembled {
        either::Either::Right(script) => {
            let mut script_blob = vec![];
            script.serialize(&mut script_blob)?;
            Entrypoint::Script(script_blob)
        },
        either::Either::Left(module) => {
            let mut module_blob = vec![];
            module.serialize(&mut module_blob)?;
            storage.add_module_bytes(module.self_addr(), module.self_name(), module_blob.into());
            Entrypoint::Module(module.self_id())
        },
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

        let mut data_cache = TransactionDataCache::empty();
        MoveVM::execute_loaded_function(
            func,
            // No arguments.
            Vec::<Vec<u8>>::new(),
            &mut MoveVmDataCacheAdapter::new(&mut data_cache, &storage, &loader),
            &mut gas_meter,
            &mut traversal_context,
            &mut extensions,
            &loader,
        )
    })?;
    println!("{:?}", return_values);

    Ok(())
}
