// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    on_chain_config::Features,
    vm::module_metadata::{get_metadata_from_compiled_code, RuntimeModuleMetadataV1},
};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, VMError, VMResult},
    file_format::{
        Bytecode, CompiledScript, FunctionHandle, FunctionHandleIndex, FunctionInstantiationIndex,
        SignatureToken::{Struct, StructInstantiation},
    },
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use std::collections::HashSet;

const EVENT_MODULE_NAME: &str = "event";
const EVENT_EMIT_FUNCTION_NAME: &str = "emit";
const INIT_MODULE_NAME: &str = "init";
const INIT_MAYBE_INITIALIZE_FUNCTION_NAME: &str = "internal_maybe_initialize";

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
    features: &Features,
    module_storage: &impl ModuleStorage,
    traversal_context: &TraversalContext,
    new_modules: &[CompiledModule],
) -> VMResult<()> {
    for new_module in new_modules {
        let mut new_event_structs = get_metadata_from_compiled_code(new_module).map_or_else(
            || Ok(HashSet::new()),
            |metadata| extract_event_metadata(&metadata),
        )?;

        // Check all the emit calls have the correct struct with event attribute.
        validate_framework_calls(&new_event_structs, new_module)?;

        // INVARIANT:
        //   No need to charge gas for module access: this function fetches the old version of the
        //   module which has been already charged when publish request was processed first (if
        //   such old version exists).
        if features.is_lazy_loading_enabled() {
            traversal_context
                .check_is_special_or_visited(new_module.address(), new_module.name())
                .map_err(|err| err.finish(Location::Undefined))?;
        }

        let old_module_metadata_if_exists = module_storage
            .unmetered_get_deserialized_module(new_module.address(), new_module.name())?
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
///
/// Also rejects closures over `0x1::init::internal_maybe_initialize`: it identifies the module to
/// initialize by its direct caller, so packed into a function value and called by another module
/// it would initialize (and mint the signer of) that module.
pub(crate) fn validate_framework_calls(
    event_structs: &HashSet<String>,
    module: &CompiledModule,
) -> VMResult<()> {
    for fun in module.function_defs() {
        if let Some(code_unit) = &fun.code {
            for bc in &code_unit.code {
                use Bytecode::*;
                match bc {
                    CallGeneric(index) => validate_emit_call(event_structs, module, *index)?,
                    PackClosureGeneric(index, ..) => {
                        validate_closure(module, module.function_instantiation_at(*index).handle)?;
                        validate_emit_call(event_structs, module, *index)?;
                    },
                    // Note: `0x1::event::emit` is generic, so it cannot be packed here, but the
                    // lifted lambda body may contain the emit function, and so will match the case
                    // above.
                    PackClosure(index, _) => validate_closure(module, *index)?,
                    // For all other instructions, no validation. We specifically do a full match
                    // here to ensure that when a new bytecode gets added, compiler complains and
                    // the validation pass is revisited.
                    VecPack(_, _)
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
                    | LdI8(_)
                    | LdI16(_)
                    | LdI32(_)
                    | LdI64(_)
                    | LdI128(_)
                    | LdI256(_)
                    | CastU8
                    | CastU16
                    | CastU32
                    | CastU64
                    | CastU128
                    | CastU256
                    | CastI8
                    | CastI16
                    | CastI32
                    | CastI64
                    | CastI128
                    | CastI256
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
                    | Negate
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
                    | AbortMsg
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

/// Validates a call of, or closure over, the generic function instantiation at `index` if it is
/// `0x1::event::emit`: the event struct must be defined in `module` with the `#[event]` attribute.
fn validate_emit_call(
    event_structs: &HashSet<String>,
    module: &CompiledModule,
    index: FunctionInstantiationIndex,
) -> VMResult<()> {
    let func_instantiation = &module.function_instantiation_at(index);
    let func_handle = module.function_handle_at(func_instantiation.handle);

    if !is_event_emit_call(BinaryIndexedView::Module(module), func_handle) {
        return Ok(());
    }

    let param = module
        .signature_at(func_instantiation.type_parameters)
        .0
        .first()
        .ok_or_else(|| {
            metadata_validation_error("Missing parameter for 0x1::event::emit function")
        })?;
    match param {
        StructInstantiation(index, _) | Struct(index) => {
            let struct_handle = &module.struct_handle_at(*index);
            let struct_name = module.identifier_at(struct_handle.name);
            if struct_handle.module != module.self_handle_idx() {
                metadata_validation_err(
                    format!(
                        "{} passed to 0x1::event::emit function is not defined in the same module",
                        struct_name
                    )
                    .as_str(),
                )
            } else if !event_structs.contains(struct_name.as_str()) {
                metadata_validation_err(format!("Missing #[event] attribute on {}. The #[event] attribute is required for all structs passed into 0x1::event::emit.", struct_name).as_str())
            } else {
                Ok(())
            }
        },
        _ => metadata_validation_err("Passed in a non-struct parameter into 0x1::event::emit."),
    }
}

/// Validates a closure over the function handle at `index`: it must not be
/// `0x1::init::internal_maybe_initialize`.
fn validate_closure(module: &CompiledModule, index: FunctionHandleIndex) -> VMResult<()> {
    let func_handle = module.function_handle_at(index);
    if is_maybe_initialize_call(BinaryIndexedView::Module(module), func_handle) {
        return Err(
            PartialVMError::new(StatusCode::CLOSURE_OVER_RESTRICTED_FUNCTION)
                .with_message(
                    "0x1::init::internal_maybe_initialize cannot be used as a function value"
                        .to_string(),
                )
                .finish(Location::Module(module.self_id())),
        );
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
///
/// The same applies to `0x1::init::internal_maybe_initialize`: a script has no module to
/// initialize, and a closure over it passed to a module would initialize that module instead.
pub(crate) fn verify_no_restricted_functions_in_compiled_script(
    script: &CompiledScript,
) -> VMResult<()> {
    let view = BinaryIndexedView::Script(script);
    for func_handle in &script.function_handles {
        if is_event_emit_call(view, func_handle) || is_maybe_initialize_call(view, func_handle) {
            return Err(PartialVMError::new(StatusCode::INVALID_OPERATION_IN_SCRIPT)
                .finish(Location::Script));
        }
    }
    Ok(())
}

/// Returns true if the handle corresponds to `0x1::init::internal_maybe_initialize`.
fn is_maybe_initialize_call(view: BinaryIndexedView, func_handle: &FunctionHandle) -> bool {
    let module_handle = view.module_handle_at(func_handle.module);
    let module_addr = view.address_identifier_at(module_handle.address);
    let module_name = view.identifier_at(module_handle.name);
    let func_name = view.identifier_at(func_handle.name);

    module_addr == &AccountAddress::ONE
        && module_name.as_str() == INIT_MODULE_NAME
        && func_name.as_str() == INIT_MAYBE_INITIALIZE_FUNCTION_NAME
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
