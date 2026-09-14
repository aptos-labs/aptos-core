// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The VM-provided `txn_arg` module, which deserializes transaction
//! arguments in Move.

use crate::calls::resolve_function_by_name;
use mono_move_core::{
    interner::{TXN_ARG_MODULE, VM_MODULE_ADDRESS},
    types::InternedType,
    Function, Interner, VMInternalError,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::BuiltinModule;
use mono_move_runtime::InterpreterContext;
use move_core_types::{ident_str, identifier::IdentStr, language_storage::ModuleId};

/// The compiled `txn_arg/` package. Regenerate with `regenerate_txn_arg_module`
/// after editing the Move source.
static TXN_ARG_MODULE_BYTES: &[u8] = include_bytes!("txn_arg.mv");

/// The modules the VM serves ahead of storage.
pub(crate) static BUILTIN_MODULES: [BuiltinModule; 1] = [BuiltinModule {
    address: VM_MODULE_ADDRESS,
    name: "txn_arg",
    bytes: TXN_ARG_MODULE_BYTES,
}];

const DESERIALIZE_ARG: &IdentStr = ident_str!("deserialize_arg");

/// Whether `module_id` is a module the VM provides or generates: the
/// argument module or a deserializer module for public structs and enums.
pub(crate) fn is_vm_module(module_id: &ModuleId) -> bool {
    module_id.address() == &VM_MODULE_ADDRESS
}

/// Loads `deserialize_arg<ty>`, and with it the deserializers of every type
/// `ty` contains. Fails if a transaction may not supply `ty`.
pub(crate) fn load_deserializer<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
) -> Result<&'a Function, VMInternalError> {
    resolve_function_by_name(
        guard,
        interp,
        &VM_MODULE_ADDRESS,
        TXN_ARG_MODULE,
        DESERIALIZE_ARG,
        guard.type_list_of(&[ty]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::CompiledModule;

    /// Compiles `txn_arg/` and writes the module bytes next to this file.
    #[test]
    #[ignore]
    fn regenerate_txn_arg_module() {
        use aptos_framework::{BuildOptions, BuiltPackage};

        let package = concat!(env!("CARGO_MANIFEST_DIR"), "/txn_arg");
        let built = BuiltPackage::build(package.into(), BuildOptions::move_2())
            .expect("the txn_arg package compiles");
        let code = built.extract_code();
        assert_eq!(code.len(), 1, "the package holds one module");
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/user_txn/txn_arg.mv"),
            &code[0],
        )
        .expect("the module bytes are written");
    }

    #[test]
    fn txn_arg_module_is_the_reserved_module() {
        let module =
            CompiledModule::deserialize(TXN_ARG_MODULE_BYTES).expect("the module deserializes");
        assert!(is_vm_module(&module.self_id()));
        assert_eq!(module.self_id().name(), TXN_ARG_MODULE);
        assert_eq!(BUILTIN_MODULES[0].name, TXN_ARG_MODULE.as_str());
    }
}
