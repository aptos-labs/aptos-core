// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    loader::LoadedFunction,
    module_traversal::TraversalContext,
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    storage::{module_storage::ModuleStorage, ty_tag_converter::TypeTagConverter},
    CodeStorage, LayoutConverter, StorageLayoutConverter,
};
use move_binary_format::{errors::*, file_format::LocalIndex};
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Changes},
    gas_algebra::NumBytes,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    values::{GlobalValue, Value},
};
use std::borrow::Borrow;

pub struct Session<'r, 'l> {
    pub(crate) move_vm: &'l MoveVM,
    pub(crate) data_cache: TransactionDataCache<'r>,
    pub(crate) native_extensions: NativeContextExtensions<'r>,
}

/// Serialized return values from function/script execution
/// Simple struct is designed just to convey meaning behind serialized values
#[derive(Debug)]
pub struct SerializedReturnValues {
    /// The value of any arguments that were mutably borrowed.
    /// Non-mut borrowed values are not included
    pub mutable_reference_outputs: Vec<(LocalIndex, Vec<u8>, MoveTypeLayout)>,
    /// The return values from the function
    pub return_values: Vec<(Vec<u8>, MoveTypeLayout)>,
}

impl<'r, 'l> Session<'r, 'l> {
    /// Execute a Move entry function.
    ///
    /// NOTE: There are NO checks on the `args` except that they can deserialize
    /// into the provided types. The ability to deserialize `args` into arbitrary
    /// types is *very* powerful, e.g., it can be used to manufacture `signer`s
    /// or `Coin`s from raw bytes. It is the responsibility of the caller to ensure
    /// that this power is used responsibly/securely for its use-case.
    pub fn execute_entry_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<()> {
        if !func.is_entry() {
            let module_id = func
                .module_id()
                .cloned()
                .expect("Entry function always has module id");
            return Err(PartialVMError::new(
                StatusCode::EXECUTE_ENTRY_FUNCTION_CALLED_ON_NON_ENTRY_FUNCTION,
            )
            .finish(Location::Module(module_id)));
        }

        self.move_vm.runtime.execute_function_instantiation(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.native_extensions,
            module_storage,
        )?;
        Ok(())
    }

    /// Execute a Move function ignoring its visibility and whether it is entry or not.
    pub fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        let func = module_storage.load_function(module_id, function_name, &ty_args)?;

        self.move_vm.runtime.execute_function_instantiation(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.native_extensions,
            module_storage,
        )
    }

    pub fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        self.move_vm.runtime.execute_function_instantiation(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.native_extensions,
            module_storage,
        )
    }

    /// Execute a transaction script.
    ///
    /// The Move VM MUST return a user error (in other words, an error that's not an invariant
    /// violation) if
    ///   - The script fails to deserialize or verify. Not all expressible signatures are valid.
    ///     See `move_bytecode_verifier::script_signature` for the rules.
    ///   - Type arguments refer to a non-existent type.
    ///   - Arguments (senders included) fail to deserialize or fail to match the signature of the
    ///     script function.
    ///
    /// If any other error occurs during execution, the Move VM MUST propagate that error back to
    /// the caller.
    /// Besides, no user input should cause the Move VM to return an invariant violation.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn execute_script(
        &mut self,
        script: impl Borrow<[u8]>,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        code_storage: &impl CodeStorage,
    ) -> VMResult<()> {
        self.move_vm.runtime.execute_script(
            script,
            ty_args,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.native_extensions,
            code_storage,
        )
    }

    pub fn num_mutated_resources(&self, sender: &AccountAddress) -> u64 {
        self.data_cache.num_mutated_resources(sender)
    }

    /// Finish up the session and produce the side effects.
    ///
    /// This function should always succeed with no user errors returned, barring invariant violations.
    ///
    /// This MUST NOT be called if there is a previous invocation that failed with an invariant violation.
    pub fn finish(self, module_storage: &impl ModuleStorage) -> VMResult<ChangeSet> {
        self.data_cache
            .into_effects(module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub fn finish_with_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(Value, MoveTypeLayout, bool) -> PartialVMResult<Resource>,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<Changes<Resource>> {
        self.data_cache
            .into_custom_effects(resource_converter, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }

    /// Same like `finish`, but also extracts the native context extensions from the session.
    pub fn finish_with_extensions(
        self,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<(ChangeSet, NativeContextExtensions<'r>)> {
        let Session {
            data_cache,
            native_extensions,
            ..
        } = self;
        let change_set = data_cache
            .into_effects(module_storage)
            .map_err(|e| e.finish(Location::Undefined))?;
        Ok((change_set, native_extensions))
    }

    pub fn finish_with_extensions_with_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(Value, MoveTypeLayout, bool) -> PartialVMResult<Resource>,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<(Changes<Resource>, NativeContextExtensions<'r>)> {
        let Session {
            data_cache,
            native_extensions,
            ..
        } = self;
        let change_set = data_cache
            .into_custom_effects(resource_converter, module_storage)
            .map_err(|e| e.finish(Location::Undefined))?;
        Ok((change_set, native_extensions))
    }

    /// Try to load a resource from remote storage and create a corresponding GlobalValue
    /// that is owned by the data store.
    pub fn load_resource(
        &mut self,
        module_storage: &impl ModuleStorage,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        self.data_cache.load_resource(module_storage, addr, ty)
    }

    pub fn get_type_tag(
        &self,
        ty: &Type,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<TypeTag> {
        let ty_tag_builder = TypeTagConverter::new(module_storage.runtime_environment());
        ty_tag_builder
            .ty_to_ty_tag(ty)
            .map_err(|e| e.finish(Location::Undefined))
    }

    /// Returns [MoveTypeLayout] of a given runtime type. For [TypeTag] to layout conversions, one
    /// needs to first convert the tag to runtime type.
    pub fn get_type_layout_from_ty(
        &self,
        ty: &Type,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        StorageLayoutConverter::new(module_storage)
            .type_to_type_layout(ty)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub fn get_fully_annotated_type_layout_from_ty(
        &self,
        ty: &Type,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        StorageLayoutConverter::new(module_storage)
            .type_to_fully_annotated_layout(ty)
            .map_err(|e| e.finish(Location::Undefined))
    }

    /// Gets the underlying native extensions.
    pub fn get_native_extensions(&mut self) -> &mut NativeContextExtensions<'r> {
        &mut self.native_extensions
    }

    pub fn get_move_vm(&self) -> &'l MoveVM {
        self.move_vm
    }

    /// If type is a (generic or non-generic) struct or enum, returns its name. Otherwise, returns
    /// [None].
    pub fn get_struct_name(
        &self,
        ty: &Type,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<Option<(ModuleId, Identifier)>> {
        use Type::*;

        Ok(match ty {
            Struct { idx, .. } | StructInstantiation { idx, .. } => {
                let struct_identifier = module_storage
                    .runtime_environment()
                    .struct_name_index_map()
                    .idx_to_struct_name(*idx)?;
                Some((struct_identifier.module, struct_identifier.name))
            },
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | Address
            | Signer
            | TyParam(_)
            | Vector(_)
            | Reference(_)
            | MutableReference(_)
            | Function { .. } => None,
        })
    }
}
