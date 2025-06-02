// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::Features,
    vm::module_metadata::{get_metadata_from_compiled_code, RuntimeModuleMetadataV1},
};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, VMError, VMResult},
    file_format::{
        Bytecode, CompiledScript, FunctionHandle,
        SignatureToken::{Struct, StructInstantiation},
    },
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use std::collections::HashSet;

const EVENT_MODULE_NAME: &str = "event";
const EVENT_EMIT_FUNCTION_NAME: &str = "emit";

fn metadata_validation_err(msg: &str) -> Result<(), VMError> {
    Err(metadata_validation_error(msg))
}

fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::EVENT_METADATA_VALIDATION_ERROR)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Validate event metadata on modules one by one:
/// * Extract the event metadata
/// * Verify all changes are compatible upgrades (existing event attributes cannot be removed)
pub(crate) fn validate_module_events(
    _features: &Features,
    _gas_feature_version: u64,
    module_storage: &impl ModuleStorage,
    // TODO(lazy-loading): add a check that the old module has been visited.
    _traversal_context: &TraversalContext,
    new_modules: &[CompiledModule],
) -> VMResult<()> {
    for new_module in new_modules {
        let mut new_event_structs = get_metadata_from_compiled_code(new_module).map_or_else(
            || Ok(HashSet::new()),
            |metadata| extract_event_metadata(&metadata),
        )?;

        // Check all the emit calls have the correct struct with event attribute.
        validate_emit_calls(&new_event_structs, new_module)?;

        let old_module_metadata_if_exists = module_storage
            .fetch_deserialized_module(new_module.address(), new_module.name())?
            .and_then(|module| {
                // TODO(loader_v2): We can optimize this to fetch metadata directly.
                get_metadata_from_compiled_code(module.as_ref())
            });
        if let Some(metadata) = old_module_metadata_if_exists {
            let original_event_structs = extract_event_metadata(&metadata)?;
            for member in original_event_structs {
                // Fail if we see a removal of an event attribute.
                if !new_event_structs.remove(&member) {
                    metadata_validation_err("Invalid change in event attributes")?;
                }
            }
        }
    }
    Ok(())
}

/// Validate all the `0x1::event::emit` calls have the struct defined in the same module with event
/// attribute. Note that this function checks regular calls, e.g.
///
/// ```move
/// // Here, `Event` must be defined in the same module.
/// 0x1::event::emit<Event>();
/// ```
///
/// as well as calls via closures:
///
/// ```move
/// // Both are allowed only if `Event` is defined in the same module.
/// let f = || 0x1::event::emit<Event>();
/// let g = {
///    // ... some Move code ...
///    0x1::event::emit<Event>();
/// }
/// ```
pub(crate) fn validate_emit_calls(
    event_structs: &HashSet<String>,
    module: &CompiledModule,
) -> VMResult<()> {
    for fun in module.function_defs() {
        if let Some(code_unit) = &fun.code {
            for bc in &code_unit.code {
                use Bytecode::*;
                match bc {
                    CallGeneric(index) | PackClosureGeneric(index, ..) => {
                        let func_instantiation = &module.function_instantiation_at(*index);
                        let func_handle = module.function_handle_at(func_instantiation.handle);

                        if !is_event_emit_call(BinaryIndexedView::Module(module), func_handle) {
                            continue;
                        }

                        let param = module
                            .signature_at(func_instantiation.type_parameters)
                            .0
                            .first()
                            .ok_or_else(|| {
                                metadata_validation_error(
                                    "Missing parameter for 0x1::event::emit function",
                                )
                            })?;
                        match param {
                            StructInstantiation(index, _) | Struct(index) => {
                                let struct_handle = &module.struct_handle_at(*index);
                                let struct_name = module.identifier_at(struct_handle.name);
                                if struct_handle.module != module.self_handle_idx() {
                                    metadata_validation_err(format!("{} passed to 0x1::event::emit function is not defined in the same module", struct_name).as_str())
                                } else if !event_structs.contains(struct_name.as_str()) {
                                    metadata_validation_err(format!("Missing #[event] attribute on {}. The #[event] attribute is required for all structs passed into 0x1::event::emit.", struct_name).as_str())
                                } else {
                                    Ok(())
                                }
                            },
                            _ => metadata_validation_err(
                                "Passed in a non-struct parameter into 0x1::event::emit.",
                            ),
                        }?;
                    },
                    // Note: If a closure is packed, it cannot be 0x1::event::emit, but the lifted
                    // lambda body may contain the emit function, and so will match the case above.
                    // For all other instructions, no validation. We specifically do a full match
                    // here to ensure that when a new bytecode gets added, compiler complains and
                    // the validation pass is revisited.
                    PackClosure(_, _)
                    | VecPack(_, _)
                    | VecLen(_)
                    | VecImmBorrow(_)
                    | VecMutBorrow(_)
                    | VecPushBack(_)
                    | VecPopBack(_)
                    | VecUnpack(_, _)
                    | VecSwap(_)
                    | CallClosure(_)
                    | Pop
                    | Ret
                    | BrTrue(_)
                    | BrFalse(_)
                    | Branch(_)
                    | LdU8(_)
                    | LdU16(_)
                    | LdU32(_)
                    | LdU64(_)
                    | LdU128(_)
                    | LdU256(_)
                    | CastU8
                    | CastU16
                    | CastU32
                    | CastU64
                    | CastU128
                    | CastU256
                    | LdConst(_)
                    | LdTrue
                    | LdFalse
                    | CopyLoc(_)
                    | MoveLoc(_)
                    | StLoc(_)
                    | MutBorrowLoc(_)
                    | ImmBorrowLoc(_)
                    | MutBorrowField(_)
                    | ImmBorrowField(_)
                    | MutBorrowFieldGeneric(_)
                    | ImmBorrowFieldGeneric(_)
                    | Call(_)
                    | Pack(_)
                    | PackGeneric(_)
                    | Unpack(_)
                    | UnpackGeneric(_)
                    | Exists(_)
                    | ExistsGeneric(_)
                    | MutBorrowGlobal(_)
                    | ImmBorrowGlobal(_)
                    | MutBorrowGlobalGeneric(_)
                    | ImmBorrowGlobalGeneric(_)
                    | MoveFrom(_)
                    | MoveFromGeneric(_)
                    | MoveTo(_)
                    | MoveToGeneric(_)
                    | FreezeRef
                    | ReadRef
                    | WriteRef
                    | Add
                    | Sub
                    | Mul
                    | Mod
                    | Div
                    | BitOr
                    | BitAnd
                    | Xor
                    | Shl
                    | Shr
                    | Or
                    | And
                    | Not
                    | Eq
                    | Neq
                    | Lt
                    | Gt
                    | Le
                    | Ge
                    | Abort
                    | Nop
                    | ImmBorrowVariantField(_)
                    | ImmBorrowVariantFieldGeneric(_)
                    | MutBorrowVariantField(_)
                    | MutBorrowVariantFieldGeneric(_)
                    | PackVariant(_)
                    | PackVariantGeneric(_)
                    | UnpackVariant(_)
                    | UnpackVariantGeneric(_)
                    | TestVariant(_)
                    | TestVariantGeneric(_) => (),
                }
            }
        }
    }
    Ok(())
}

/// Given a module id extract all event metadata
pub(crate) fn extract_event_metadata(
    metadata: &RuntimeModuleMetadataV1,
) -> VMResult<HashSet<String>> {
    let mut event_structs = HashSet::new();
    for (struct_, attrs) in &metadata.struct_attributes {
        for attr in attrs {
            if attr.is_event() && !event_structs.insert(struct_.clone()) {
                metadata_validation_err("Found duplicate event attribute")?;
            }
        }
    }
    Ok(event_structs)
}

/// Returns an error if the script uses 0x1::event::emit function (whether as a direct call, or as
/// a closure). Note that this is not overly restrictive: even a callback to emit an event cannot
/// be passed, i.e. the following script should fail:
///
/// ```move
/// script {
///   fun main() {
///     let f = |e| {
///       // ... do something here ...
///       0x1::event::emit(e);
///     }
///
///     // This call creates an event and calls `f` on it.
///     0x123::some_module::some_function(f);
///   }
/// }
/// ```
///
/// This is ok to fail here, as event emission should be done by the module where event is defined.
pub(crate) fn verify_no_event_emission_in_compiled_script(script: &CompiledScript) -> VMResult<()> {
    for func_handle in &script.function_handles {
        if is_event_emit_call(BinaryIndexedView::Script(script), func_handle) {
            debug_assert!(func_handle.type_parameters.len() == 1);
            return Err(PartialVMError::new(StatusCode::INVALID_OPERATION_IN_SCRIPT)
                .finish(Location::Script));
        }
    }
    Ok(())
}

/// Returns true if the handle corresponds to `0x1::event::emit` function call.
fn is_event_emit_call(view: BinaryIndexedView, func_handle: &FunctionHandle) -> bool {
    let module_handle = view.module_handle_at(func_handle.module);
    let module_addr = view.address_identifier_at(module_handle.address);
    let module_name = view.identifier_at(module_handle.name);
    let func_name = view.identifier_at(func_handle.name);

    module_addr == &AccountAddress::ONE
        && module_name.as_str() == EVENT_MODULE_NAME
        && func_name.as_str() == EVENT_EMIT_FUNCTION_NAME
}
