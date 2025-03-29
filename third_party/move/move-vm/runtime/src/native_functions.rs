// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache, interpreter::InterpreterDebugInterface, loader::Function,
    module_traversal::TraversalContext, native_extensions::NativeContextExtensions,
    storage::module_storage::FunctionValueExtensionAdapter, RuntimeEnvironment,
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
    loaded_data::runtime_types::Type, natives::function::NativeResult, resolver::ResourceResolver,
    values::Value,
};
use std::{
    collections::{HashMap, VecDeque},
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

pub struct NativeContext<'a, 'b> {
    interpreter: &'a mut dyn InterpreterDebugInterface,
    data_store: &'a mut TransactionDataCache,
    resource_resolver: &'a dyn ResourceResolver,
    runtime_environment: &'a RuntimeEnvironment,
    extensions: &'a mut NativeContextExtensions<'b>,
    gas_balance: InternalGas,

    /// Counter used to record the (conceptual) heap memory usage by a native functions,
    /// measured in abstract memory unit.
    ///
    /// This is a hack to emulate memory usage tracking, before we could refactor native functions
    /// and allow them to access the gas meter directly.
    heap_memory_usage: u64,
}

impl<'a, 'b> NativeContext<'a, 'b> {
    pub(crate) fn new(
        interpreter: &'a mut dyn InterpreterDebugInterface,
        data_store: &'a mut TransactionDataCache,
        resource_resolver: &'a dyn ResourceResolver,
        runtime_environment: &'a RuntimeEnvironment,
        extensions: &'a mut NativeContextExtensions<'b>,
        gas_balance: InternalGas,
    ) -> Self {
        Self {
            interpreter,
            data_store,
            resource_resolver,
            runtime_environment,
            extensions,
            gas_balance,

            heap_memory_usage: 0,
        }
    }
}

impl<'a, 'b> NativeContext<'a, 'b> {
    pub fn print_stack_trace(&self, buf: &mut String) -> PartialVMResult<()> {
        self.interpreter
            .debug_print_stack_trace(buf, self.runtime_environment)
    }

    pub fn exists_at(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> VMResult<(bool, Option<NumBytes>)> {
        // TODO(Rati, George): propagate exists call the way to resolver, because we
        //                     can implement the check more efficiently, without the
        //                     need to actually load bytes.
        Ok(match self.data_store.get_resource_if_loaded(&address, ty) {
            Some(value) => {
                let exists = value
                    .exists()
                    .map_err(|err| err.finish(Location::Undefined))?;
                (exists, None)
            },
            None => {
                let (layout, contains_delayed_fields, value, size) =
                    TransactionDataCache::load_resource_native(
                        self.resource_resolver,
                        &address,
                        ty,
                    )
                    .map_err(|err| err.finish(Location::Undefined))?;
                let exists = value
                    .exists()
                    .map_err(|err| err.finish(Location::Undefined))?;
                self.data_store
                    .insert_resource(address, ty.clone(), value, layout, contains_delayed_fields)
                    .map_err(|err| err.finish(Location::Undefined))?;
                (exists, Some(size))
            },
        })
    }

    pub fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        self.runtime_environment.ty_to_ty_tag(ty)
    }

    pub fn type_to_type_layout(&self, _ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        // TODO(lazy): implement native loader to be able to work with layouts here.
        unimplemented!()
        // StorageLayoutConverter::new(self.module_storage).type_to_type_layout(ty)
    }

    pub fn type_to_type_layout_with_identifier_mappings(
        &self,
        _ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        // TODO(lazy): implement native loader to be able to work with layouts here.
        unimplemented!()
        // StorageLayoutConverter::new(self.module_storage)
        //     .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub fn type_to_fully_annotated_layout(&self, _ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        // TODO(lazy): implement native loader to be able to work with layouts here.
        unimplemented!()
        // StorageLayoutConverter::new(self.module_storage).type_to_fully_annotated_layout(ty)
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

    pub fn use_heap_memory(&mut self, amount: u64) {
        self.heap_memory_usage = self.heap_memory_usage.saturating_add(amount);
    }

    pub fn heap_memory_usage(&self) -> u64 {
        self.heap_memory_usage
    }

    pub fn traversal_context(&self) -> &TraversalContext {
        // TODO(lazy): implement native loader to be able to access traversal_context.
        unimplemented!()
    }

    pub fn function_value_extension(&self) -> FunctionValueExtensionAdapter {
        // TODO(lazy): implement native loader to be able to (de)serialize function values.
        unimplemented!()
        // FunctionValueExtensionAdapter {
        //     module_storage: self.module_storage,
        // }
    }

    pub fn load_function(
        &mut self,
        _module_id: &ModuleId,
        _function_name: &Identifier,
    ) -> VMResult<Arc<Function>> {
        // TODO(lazy): implement native loader to be able to load function directly here.
        unimplemented!()
        // let (_, function) = self.module_storage.fetch_function_definition(
        //     module_id.address(),
        //     module_id.name(),
        //     function_name,
        // )?;
        // Ok(function)
    }
}
