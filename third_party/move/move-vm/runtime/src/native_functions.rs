// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    interpreter::Interpreter,
    loader::{Function, Loader, Resolver},
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
};
use move_binary_format::errors::{
    ExecutionState, Location, PartialVMError, PartialVMResult, VMResult,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, NumBytes},
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Write,
    sync::Arc,
};

pub type UnboxedNativeFunction = dyn Fn(&mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>
    + Send
    + Sync
    + 'static;

pub type NativeFunction = Arc<UnboxedNativeFunction>;

pub type NativeFunctionTable = Vec<(AccountAddress, Identifier, Identifier, NativeFunction)>;

pub fn make_table(
    addr: AccountAddress,
    elems: &[(&str, &str, NativeFunction)],
) -> NativeFunctionTable {
    make_table_from_iter(addr, elems.iter().cloned())
}

pub fn make_table_from_iter<S: Into<Box<str>>>(
    addr: AccountAddress,
    elems: impl IntoIterator<Item = (S, S, NativeFunction)>,
) -> NativeFunctionTable {
    elems
        .into_iter()
        .map(|(module_name, func_name, func)| {
            (
                addr,
                Identifier::new(module_name).unwrap(),
                Identifier::new(func_name).unwrap(),
                func,
            )
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct NativeFunctions(
    HashMap<AccountAddress, HashMap<String, HashMap<String, NativeFunction>>>,
);

impl NativeFunctions {
    pub fn resolve(
        &self,
        addr: &AccountAddress,
        module_name: &str,
        func_name: &str,
    ) -> Option<NativeFunction> {
        self.0.get(addr)?.get(module_name)?.get(func_name).cloned()
    }

    pub fn new<I>(natives: I) -> PartialVMResult<Self>
    where
        I: IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
    {
        let mut map = HashMap::new();
        for (addr, module_name, func_name, func) in natives.into_iter() {
            let modules = map.entry(addr).or_insert_with(HashMap::new);
            let funcs = modules
                .entry(module_name.into_string())
                .or_insert_with(HashMap::new);

            if funcs.insert(func_name.into_string(), func).is_some() {
                return Err(PartialVMError::new(StatusCode::DUPLICATE_NATIVE_FUNCTION));
            }
        }
        Ok(Self(map))
    }
}

pub struct NativeContext<'a, 'b, 'c> {
    interpreter: &'a mut Interpreter,
    data_store: &'a mut TransactionDataCache<'c>,
    resolver: &'a Resolver<'a>,
    extensions: &'a mut NativeContextExtensions<'b>,
    gas_balance: InternalGas,
    traversal_context: &'a TraversalContext<'a>,
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub(crate) fn new(
        interpreter: &'a mut Interpreter,
        data_store: &'a mut TransactionDataCache<'c>,
        resolver: &'a Resolver<'a>,
        extensions: &'a mut NativeContextExtensions<'b>,
        gas_balance: InternalGas,
        traversal_context: &'a TraversalContext<'a>,
    ) -> Self {
        Self {
            interpreter,
            data_store,
            resolver,
            extensions,
            gas_balance,
            traversal_context,
        }
    }
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub fn print_stack_trace<B: Write>(&self, buf: &mut B) -> PartialVMResult<()> {
        self.interpreter.debug_print_stack_trace(buf, self.resolver)
    }

    pub fn exists_at(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> VMResult<(bool, Option<NumBytes>)> {
        // TODO(Rati, George): propagate exists call the way to resolver, because we
        //                     can implement the check more efficiently, without the
        //                     need to actually load bytes.
        let (value, num_bytes) = self
            .data_store
            .load_resource(
                self.resolver.loader(),
                self.resolver.module_storage(),
                address,
                ty,
                self.resolver.module_store(),
            )
            .map_err(|err| err.finish(Location::Undefined))?;
        let exists = value
            .exists()
            .map_err(|err| err.finish(Location::Undefined))?;
        Ok((exists, num_bytes))
    }

    pub fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        self.resolver
            .loader()
            .type_to_type_tag(ty, self.resolver.module_storage())
    }

    pub fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        self.resolver.type_to_type_layout(ty)
    }

    pub fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.resolver
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub fn type_to_fully_annotated_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        self.resolver.type_to_fully_annotated_layout(ty)
    }

    pub fn extensions(&self) -> &NativeContextExtensions<'b> {
        self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut NativeContextExtensions<'b> {
        self.extensions
    }

    /// Get count stack frames, including the one of the called native function. This
    /// allows a native function to reflect about its caller.
    pub fn stack_frames(&self, count: usize) -> ExecutionState {
        self.interpreter.get_stack_frames(count)
    }

    pub fn gas_balance(&self) -> InternalGas {
        self.gas_balance
    }

    pub fn traversal_context(&self) -> &TraversalContext {
        self.traversal_context
    }

    pub fn load_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &Identifier,
    ) -> PartialVMResult<Arc<Function>> {
        let (_, function) = match self.resolver.loader() {
            Loader::V1(loader) => {
                // Load the module that contains this function regardless of the traversal context.
                //
                // This is just a precautionary step to make sure that caching status of the VM will not alter execution
                // result in case framework code forgot to use LoadFunction result to load the modules into cache
                // and charge properly.
                loader
                    .load_module(module_id, self.data_store, self.resolver.module_store())
                    .map_err(|_| {
                        PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                            .with_message(format!("Module {} doesn't exist", module_id))
                    })?;

                self.resolver
                    .module_store()
                    .resolve_module_and_function_by_name(module_id, function_name)?
            },
            Loader::V2(loader) => loader
                .load_function_without_ty_args(
                    self.resolver.module_storage(),
                    module_id.address(),
                    module_id.name(),
                    function_name,
                )
                // TODO(loader_v2):
                //   Keeping this consistent with loader V1 implementation which returned that
                //   error. Check if we can avoid remapping by replaying transactions.
                .map_err(|_| {
                    PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                        .with_message(format!("Module {} doesn't exist", module_id))
                })?,
        };
        Ok(function)
    }
}
