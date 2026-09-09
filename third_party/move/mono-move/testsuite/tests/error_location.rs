// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for the [`ErrorLocation`] attached to failures from compiled
//! Move programs.
//!
//! Expected function definition indexes and bytecode offsets are derived from
//! the compiled modules so compiler layout changes do not invalidate them.
//!
//! A failure while `CallIndirect` loads its callee, a lazy closure-resolution
//! failure, and a VM error raised by a native are all attributed to the call
//! instruction.
//!
//! Bytecode execution errors are tested differentially. These tests cover
//! resolution and native failures separately: missing-module cases require
//! withholding compiled modules, and error locations intentionally differ.

use mono_move_core::{
    native::NativeExtensions, BytecodeOffset, ErrorLocation, FunctionDefinitionIndex,
    VMInternalError,
};
use mono_move_global_context::GlobalContext;
use mono_move_testsuite::{
    function_def_index, with_mono_function, InMemoryModuleProvider, RunResult,
};
use move_binary_format::{access::ModuleAccess, file_format::Bytecode, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
};

/// Compiles `source`, runs `0x1::<module_name>::<function_name>` until it
/// fails, and returns the failure together with the compiled modules the
/// expectations are read from.
///
/// `args` are placed as consecutive 8-byte words in the root frame. No native
/// extensions are installed, which is what makes an extension-backed native
/// fail; a native the registry does not know fails to load instead.
fn run_to_failure(
    source: &str,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[u64],
) -> (Vec<CompiledModule>, VMInternalError) {
    run_to_failure_without(source, &[], module_name, function_name, args)
}

/// As [`run_to_failure`], but the modules named in `withheld` are compiled and
/// not published. Calling into one then fails to resolve at run time, which no
/// compilable source can express on its own.
fn run_to_failure_without(
    source: &str,
    withheld: &[&str],
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[u64],
) -> (Vec<CompiledModule>, VMInternalError) {
    let modules = mono_move_testsuite::compile_move_source(source).expect("source compiles");
    let mut module_provider = InMemoryModuleProvider::new();
    for module in modules
        .iter()
        .filter(|module| !withheld.contains(&module.self_name().as_str()))
    {
        module_provider.add_module(module);
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 execution context must be available");

    let outcome = with_mono_function(
        &guard,
        &module_provider,
        AccountAddress::ONE,
        module_name,
        function_name,
        NativeExtensions::new(),
        None,
        |runner| runner.run(&[], |call, index| call.arg(&args[index]), |_| ()),
    )
    .expect("the entry function loads");

    match outcome {
        RunResult::Error(err) => (modules, err),
        RunResult::Success(()) => panic!("the program under test must fail, but it succeeded"),
        RunResult::Aborted { code, .. } => {
            panic!("expected a VM failure, but the program aborted with code {code}")
        },
    }
}

/// Returns the definition index of `name` in the compiled module named
/// `module_name`.
fn def_idx(modules: &[CompiledModule], module_name: &str, name: &str) -> FunctionDefinitionIndex {
    let module = modules
        .iter()
        .find(|module| module.self_name().as_str() == module_name)
        .expect("module is in the compiled output");
    function_def_index(module, name).expect("function is defined in the module")
}

/// The bytecode offset of the only instruction in the function at `def_idx`
/// satisfying `matches`. Panics unless exactly one does, since otherwise the
/// expected offset would be ambiguous.
fn sole_offset_of(
    modules: &[CompiledModule],
    module_name: &str,
    def_idx: FunctionDefinitionIndex,
    matches: impl Fn(&Bytecode) -> bool,
) -> BytecodeOffset {
    let module = modules
        .iter()
        .find(|module| module.self_name().as_str() == module_name)
        .expect("module is in the compiled output");
    let code = &module.function_defs()[def_idx.0 as usize]
        .code
        .as_ref()
        .expect("function has a body")
        .code;
    let mut found = code
        .iter()
        .enumerate()
        .filter(|(_, instruction)| matches(instruction));
    let (offset, _) = found.next().expect("no instruction matched");
    assert!(found.next().is_none(), "more than one instruction matched");
    offset as BytecodeOffset
}

fn module_id(name: &str) -> ModuleId {
    ModuleId::new(
        AccountAddress::ONE,
        IdentStr::new(name).expect("valid identifier").to_owned(),
    )
}

/// The instruction the failure was attributed to.
fn instruction_of(err: &VMInternalError) -> (&ModuleId, FunctionDefinitionIndex, BytecodeOffset) {
    match err.location() {
        Some(ErrorLocation::Instruction {
            module,
            function,
            offset,
        }) => (module, *function, *offset),
        other => panic!("expected the failure to name an instruction, got {other:?}"),
    }
}

/// A callee that cannot be resolved is attributed to the call instruction, so
/// this names the call in `inner` rather than `outer`, the caller's module V1
/// would name. Withholding `missing` is what makes the call fail: no compilable
/// source can name a module that does not exist.
#[test]
fn load_failure_names_the_calling_instruction() {
    const SOURCE: &str = r#"
        module 0x1::missing {
            public fun gone(): u64 { 1 }
        }
        module 0x1::inner {
            public fun middle(): u64 { 0x1::missing::gone() }
        }
        module 0x1::outer {
            public fun entry(): u64 { 0x1::inner::middle() }
        }
    "#;

    let (modules, err) = run_to_failure_without(
        SOURCE,
        &["missing"],
        ident_str!("outer"),
        ident_str!("entry"),
        &[],
    );
    let middle = def_idx(&modules, "inner", "middle");
    let (module, function, offset) = instruction_of(&err);

    assert_eq!(*module, module_id("inner"));
    assert_eq!(function, middle);
    assert_eq!(
        offset,
        sole_offset_of(&modules, "inner", middle, |i| matches!(
            i,
            Bytecode::Call(_)
        ))
    );
}

/// A registered transaction-context native, reached from another module. It
/// fails because the tests install no native extensions.
const NATIVE_SOURCE: &str = r#"
    module 0x1::transaction_context {
        native public fun generate_unique_address(): address;
    }
    module 0x1::caller {
        public fun entry(): address {
            0x1::transaction_context::generate_unique_address()
        }
    }
"#;

/// A native failure is attributed to the instruction that called it, not to the
/// native's own definition: a native has no bytecode of its own to point at, and
/// the call site identifies it. This is deliberately unlike a native's *abort*,
/// whose location names the native's module because abort codes are scoped to
/// the module defining them.
#[test]
fn native_failure_names_the_calling_instruction() {
    let (modules, err) =
        run_to_failure(NATIVE_SOURCE, ident_str!("caller"), ident_str!("entry"), &[
        ]);
    let entry = def_idx(&modules, "caller", "entry");
    let (module, function, offset) = instruction_of(&err);

    assert_eq!(*module, module_id("caller"));
    assert_eq!(function, entry);
    assert_eq!(
        offset,
        sole_offset_of(&modules, "caller", entry, |i| matches!(
            i,
            Bytecode::Call(_)
        )),
        "the offset must be the call to the native, not some other instruction"
    );
}

/// A native the registry does not have fails to load and is blamed on the call
/// instruction like any other load failure. Nothing registers
/// `0x1::absent::nowhere`.
#[test]
fn missing_native_names_the_calling_instruction() {
    const SOURCE: &str = r#"
        module 0x1::absent {
            native public fun nowhere(): u64;
        }
        module 0x1::inner {
            public fun middle(): u64 { 0x1::absent::nowhere() }
        }
        module 0x1::outer {
            public fun entry(): u64 { 0x1::inner::middle() }
        }
    "#;

    let (modules, err) = run_to_failure(SOURCE, ident_str!("outer"), ident_str!("entry"), &[]);
    let middle = def_idx(&modules, "inner", "middle");
    let (module, function, offset) = instruction_of(&err);

    assert_eq!(*module, module_id("inner"));
    assert_eq!(function, middle);
    assert_eq!(
        offset,
        sole_offset_of(&modules, "inner", middle, |i| matches!(
            i,
            Bytecode::Call(_)
        ))
    );
}
