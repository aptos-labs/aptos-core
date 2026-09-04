// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running a script payload.

use super::args::place_user_txn_args;
use crate::errors::{MoveExecutionFailure, ScriptRejection};
use aptos_types::{chain_id::ChainId, vm::module_metadata::get_compilation_metadata};
use mono_move_core::types::InternedTypeList;
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    transaction_argument::{convert_txn_args, TransactionArgument},
};

const EVENT_MODULE: &IdentStr = ident_str!("event");
const EVENT_EMIT: &IdentStr = ident_str!("emit");

/// Runs the transaction's script, metered against the transaction's gas budget.
pub(crate) fn run_script<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    chain_id: ChainId,
    code: &[u8],
    ty_args: InternedTypeList,
    sender: &AccountAddress,
    secondary_signers: &[AccountAddress],
    args: &[TransactionArgument],
) -> Result<RuntimeStatus, MoveExecutionFailure> {
    let func = interp
        .load_script(code, ty_args)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let module = interp
        .read_set()
        .get_loaded(guard.arena_ref_for_module_id(func.module_id))
        .map_err(MoveExecutionFailure::RuntimeError)?;
    check_script_allowed(&module.ir().module, chain_id)
        .map_err(MoveExecutionFailure::RejectedScript)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(
        &mut call,
        sender,
        secondary_signers,
        &convert_txn_args(args),
    )?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}

/// Checks that AptosVM would run `script`, loaded as a module: mainnet refuses
/// scripts their compiler marked unstable, and no script may emit events.
fn check_script_allowed(script: &CompiledModule, chain_id: ChainId) -> Result<(), ScriptRejection> {
    if chain_id.is_mainnet()
        && get_compilation_metadata(script).is_some_and(|metadata| metadata.unstable)
    {
        return Err(ScriptRejection::UnstableOnMainnet);
    }
    let emits_events = script.function_handles().iter().any(|handle| {
        let module = script.module_handle_at(handle.module);
        *script.address_identifier_at(module.address) == AccountAddress::ONE
            && script.identifier_at(module.name) == EVENT_MODULE
            && script.identifier_at(handle.name) == EVENT_EMIT
    });
    if emits_events {
        return Err(ScriptRejection::EmitsEvents);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_language_e2e_tests::compile::compile_script;
    use move_binary_format::{
        file_format::CompiledScript, module_script_conversion::script_into_module,
    };
    use move_core_types::metadata::Metadata;
    use move_model::metadata::{CompilationMetadata, COMPILATION_METADATA_KEY};

    const EMPTY_SCRIPT: &str = r#"
script

public fun main()
    ret
"#;

    /// Assembles `code` and loads it as a module, marked `unstable` if given.
    fn script_module(code: &str, unstable: Option<bool>) -> CompiledModule {
        let bytes = compile_script(code, vec![]).code().to_vec();
        let mut script =
            CompiledScript::deserialize(&bytes).expect("the assembler's output deserializes");
        if let Some(unstable) = unstable {
            let metadata = CompilationMetadata {
                unstable,
                compiler_version: "2.0".to_string(),
                language_version: "2.0".to_string(),
            };
            script.metadata.push(Metadata {
                key: COMPILATION_METADATA_KEY.to_vec(),
                value: bcs::to_bytes(&metadata).expect("metadata serializes"),
            });
        }
        script_into_module(script, "main")
    }

    #[test]
    fn unstable_script_is_refused_on_mainnet_only() {
        let script = script_module(EMPTY_SCRIPT, Some(true));
        assert!(matches!(
            check_script_allowed(&script, ChainId::mainnet()),
            Err(ScriptRejection::UnstableOnMainnet)
        ));
        assert!(check_script_allowed(&script, ChainId::test()).is_ok());
    }

    #[test]
    fn stable_and_unmarked_scripts_run_on_mainnet() {
        for unstable in [Some(false), None] {
            let script = script_module(EMPTY_SCRIPT, unstable);
            assert!(check_script_allowed(&script, ChainId::mainnet()).is_ok());
        }
    }

    #[test]
    fn event_emitting_script_is_refused() {
        let script = script_module(
            r#"
script
use 0x1::event

public fun main()
    ld_u64 1
    call event::emit<u64>
    ret
"#,
            None,
        );
        assert!(matches!(
            check_script_allowed(&script, ChainId::test()),
            Err(ScriptRejection::EmitsEvents)
        ));
    }
}
