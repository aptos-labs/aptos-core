// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    interpreter::InterpreterDebugInterface,
    loader::Function,
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    storage::{
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::{
            LayoutConverter, MetredLazyNativeLayoutConverter, UnmeteredLayoutConverter,
        },
    },
    ModuleStorage,
};
use move_binary_format::errors::{
    ExecutionState, Location, PartialVMError, PartialVMResult, VMResult,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, NumBytes, NumModules},
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter, loaded_data::runtime_types::Type, natives::function::NativeResult,
    resolver::ResourceResolver, values::Value,
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

pub struct NativeContext<'a, 'b, 'c> {
    interpreter: &'a dyn InterpreterDebugInterface,
    data_store: &'a mut TransactionDataCache,
    resource_resolver: &'a dyn ResourceResolver,
    module_storage: &'a dyn ModuleStorage,
    extensions: &'a mut NativeContextExtensions<'b>,
    gas_balance: InternalGas,
    traversal_context: &'a mut TraversalContext<'c>,

    /// Counter used to record the (conceptual) heap memory usage by a native functions,
    /// measured in abstract memory unit.
    ///
    /// This is a hack to emulate memory usage tracking, before we could refactor native functions
    /// and allow them to access the gas meter directly.
    heap_memory_usage: u64,

    total_dependency_size: NumBytes,
    max_total_dependency_size: NumBytes,
    num_dependencies: NumModules,
    max_num_dependencies: NumModules,
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub(crate) fn new(
        interpreter: &'a dyn InterpreterDebugInterface,
        data_store: &'a mut TransactionDataCache,
        resource_resolver: &'a dyn ResourceResolver,
        module_storage: &'a dyn ModuleStorage,
        extensions: &'a mut NativeContextExtensions<'b>,
        gas_meter: &impl GasMeter,
        traversal_context: &'a mut TraversalContext<'c>,
    ) -> Self {
        Self {
            interpreter,
            data_store,
            resource_resolver,
            module_storage,
            extensions,
            gas_balance: gas_meter.balance_internal(),
            traversal_context,

            heap_memory_usage: 0,
            total_dependency_size: gas_meter.total_dependency_size(),
            max_total_dependency_size: gas_meter.max_total_dependency_size(),
            num_dependencies: gas_meter.num_dependencies(),
            max_num_dependencies: gas_meter.max_num_dependencies(),
        }
    }
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub fn print_stack_trace(&self, buf: &mut String) -> PartialVMResult<()> {
        self.interpreter
            .debug_print_stack_trace(buf, self.module_storage.runtime_environment())
    }

    pub fn exists_at(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        let bytes_loaded = if !self.data_store.is_resource_loaded(&address, ty) {
            let (entry, bytes_loaded) = TransactionDataCache::load_resource_for_natives(
                self.module_storage,
                self.resource_resolver,
                &address,
                ty,
            )?;
            self.data_store
                .store_loaded_resource(address, ty.clone(), entry)?;
            Some(bytes_loaded)
        } else {
            None
        };
        let exists = self
            .data_store
            .get_resource_if_loaded(&address, ty)?
            .exists()?;
        Ok((exists, bytes_loaded))
    }

    pub fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        self.module_storage.runtime_environment().ty_to_ty_tag(ty)
    }

    pub fn metered_lazy_type_to_type_layout<F>(
        &mut self,
        ty: &Type,
        gas_used: InternalGas,
        cost_fn: F,
    ) -> PartialVMResult<MoveTypeLayout>
    where
        F: Fn(u64) -> InternalGas,
    {
        MetredLazyNativeLayoutConverter::new(
            self.traversal_context,
            self.module_storage,
            gas_used,
            self.gas_balance,
            self.total_dependency_size,
            self.max_total_dependency_size,
            self.num_dependencies,
            self.max_num_dependencies,
            cost_fn,
        )
        .type_to_type_layout(ty)
    }

    pub fn unmetered_type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        UnmeteredLayoutConverter::new(self.module_storage).type_to_type_layout(ty)
    }

    pub fn metered_lazy_type_to_type_layout_with_identifier_mappings<F>(
        &mut self,
        ty: &Type,
        gas_used: InternalGas,
        cost_fn: F,
    ) -> PartialVMResult<(MoveTypeLayout, bool)>
    where
        F: Fn(u64) -> InternalGas,
    {
        MetredLazyNativeLayoutConverter::new(
            self.traversal_context,
            self.module_storage,
            gas_used,
            self.gas_balance,
            self.total_dependency_size,
            self.max_total_dependency_size,
            self.num_dependencies,
            self.max_num_dependencies,
            cost_fn,
        )
        .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub fn unmetered_type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        UnmeteredLayoutConverter::new(self.module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub fn metered_lazy_type_to_fully_annotated_layout<F>(
        &mut self,
        ty: &Type,
        gas_used: InternalGas,
        cost_fn: F,
    ) -> PartialVMResult<MoveTypeLayout>
    where
        F: Fn(u64) -> InternalGas,
    {
        MetredLazyNativeLayoutConverter::new(
            self.traversal_context,
            self.module_storage,
            gas_used,
            self.gas_balance,
            self.total_dependency_size,
            self.max_total_dependency_size,
            self.num_dependencies,
            self.max_num_dependencies,
            cost_fn,
        )
        .type_to_fully_annotated_layout(ty)
    }

    pub fn unmetered_type_to_fully_annotated_layout(
        &self,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        UnmeteredLayoutConverter::new(self.module_storage).type_to_fully_annotated_layout(ty)
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

    pub fn num_dependencies(&self) -> NumModules {
        self.num_dependencies
    }

    pub fn total_dependency_size(&self) -> NumBytes {
        self.total_dependency_size
    }

    pub fn use_heap_memory(&mut self, amount: u64) {
        self.heap_memory_usage = self.heap_memory_usage.saturating_add(amount);
    }

    pub fn heap_memory_usage(&self) -> u64 {
        self.heap_memory_usage
    }

    pub fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }

    pub fn use_lazy_loading(&self) -> bool {
        self.module_storage
            .runtime_environment()
            .vm_config()
            .use_lazy_loading
    }

    pub fn traversal_context(&self) -> &TraversalContext {
        self.traversal_context
    }

    pub fn traversal_context_mut(&mut self) -> &mut TraversalContext<'c> {
        self.traversal_context
    }

    pub fn function_value_extension(&self) -> FunctionValueExtensionAdapter {
        FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        }
    }

    pub fn load_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &Identifier,
    ) -> VMResult<Arc<Function>> {
        // MODULE LOADING METERING:
        //   Metering is done when native returns LoadModule result, so this access will never load
        //   anything and will access cached and metered modules. Currently, native implementations
        //   check if the loading was metered or not before calling here, but this kept as an extra
        //   redundancy check in case there is a mistake and gas is not charged somehow.
        self.traversal_context
            .check_is_special_or_visited(module_id.address(), module_id.name())
            .map_err(|err| err.finish(Location::Undefined))?;
        let (_, function) = self.module_storage.unmetered_get_function_definition(
            module_id.address(),
            module_id.name(),
            function_name,
        )?;
        Ok(function)
    }
}
