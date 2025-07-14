// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ambassador_impl_ModuleStorage, ambassador_impl_WithRuntimeEnvironment,
    check_dependencies_and_charge_gas,
    data_cache::TransactionDataCache,
    interpreter::InterpreterDebugInterface,
    loader::{LazyLoadedFunction, LazyLoadedFunctionState},
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    storage::{
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::{LayoutConverter, StorageLayoutConverter},
    },
    Function, LoadedFunction, Module, ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use ambassador::delegate_to_methods;
use bytes::Bytes;
use move_binary_format::{
    errors::{ExecutionState, PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, NumBytes},
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{ambassador_impl_DependencyGasMeter, DependencyGasMeter, NativeGasMeter},
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
    natives::function::NativeResult,
    resolver::ResourceResolver,
    values::{AbstractFunction, Value},
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
    gas_meter: &'a mut dyn NativeGasMeter,
    traversal_context: &'a mut TraversalContext<'c>,
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub(crate) fn new(
        interpreter: &'a dyn InterpreterDebugInterface,
        data_store: &'a mut TransactionDataCache,
        resource_resolver: &'a dyn ResourceResolver,
        module_storage: &'a dyn ModuleStorage,
        extensions: &'a mut NativeContextExtensions<'b>,
        gas_meter: &'a mut dyn NativeGasMeter,
        traversal_context: &'a mut TraversalContext<'c>,
    ) -> Self {
        Self {
            interpreter,
            data_store,
            resource_resolver,
            module_storage,
            extensions,
            gas_meter,
            traversal_context,
        }
    }
}

impl<'b, 'c> NativeContext<'_, 'b, 'c> {
    pub fn print_stack_trace(&self, buf: &mut String) -> PartialVMResult<()> {
        self.interpreter
            .debug_print_stack_trace(buf, self.module_storage.runtime_environment())
    }

    pub fn exists_at(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        // TODO(#16516):
        //   Propagate exists call all the way to resolver, because we can implement the check more
        //   efficiently, without the need to actually load bytes, deserialize the value and cache
        //   it in the data cache.
        Ok(if !self.data_store.contains_resource(&address, ty) {
            let (mut entry, bytes_loaded) = TransactionDataCache::create_data_cache_entry(
                self.module_storage,
                self.resource_resolver,
                &address,
                ty,
            )?;
            let exists = entry.value(self.module_storage, &address)?.exists()?;
            self.data_store
                .insert_resource(address, ty.clone(), entry)?;
            (exists, Some(bytes_loaded))
        } else {
            let exists = self.data_store.get_resource_mut(&address, ty, self.module_storage)?.exists()?;
            (exists, None)
        })
    }

    pub fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        self.module_storage.runtime_environment().ty_to_ty_tag(ty)
    }

    pub fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        StorageLayoutConverter::new(self.module_storage).type_to_type_layout(ty)
    }

    pub fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        StorageLayoutConverter::new(self.module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub fn type_to_fully_annotated_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        StorageLayoutConverter::new(self.module_storage).type_to_fully_annotated_layout(ty)
    }

    pub fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
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

    pub fn legacy_gas_budget(&self) -> InternalGas {
        self.gas_meter.legacy_gas_budget_in_native_context()
    }

    /// Returns the gas meter used for execution. Even if native functions cannot use it to
    /// charge gas (feature-gating), gas meter can be used to query gas meter's balance.
    pub fn gas_meter(&mut self) -> &mut dyn NativeGasMeter {
        self.gas_meter
    }

    pub fn charge_gas_for_dependencies(&mut self, module_id: ModuleId) -> VMResult<()> {
        let arena_id = self
            .traversal_context
            .referenced_module_ids
            .alloc(module_id);
        check_dependencies_and_charge_gas(
            self.module_storage,
            self.gas_meter,
            self.traversal_context,
            [(arena_id.address(), arena_id.name())],
        )
    }

    pub fn traversal_context(&self) -> &TraversalContext<'c> {
        self.traversal_context
    }

    pub fn function_value_extension(&self) -> FunctionValueExtensionAdapter {
        FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        }
    }

    /// Returns a vector of layouts for captured arguments. Used to format captured arguments as
    /// strings. Returns [Ok(None)] in case layouts contain delayed fields (i.e., the values cannot
    /// be formatted as strings).
    pub fn get_captured_layouts_for_string_utils(
        &mut self,
        fun: &dyn AbstractFunction,
    ) -> PartialVMResult<Option<Vec<MoveTypeLayout>>> {
        Ok(
            match &*LazyLoadedFunction::expect_this_impl(fun)?.state.borrow() {
                LazyLoadedFunctionState::Unresolved { data, .. } => {
                    Some(data.captured_layouts.clone())
                },
                LazyLoadedFunctionState::Resolved {
                    fun,
                    mask,
                    captured_layouts,
                    ..
                } => match captured_layouts.as_ref() {
                    Some(captured_layouts) => Some(captured_layouts.clone()),
                    None => LazyLoadedFunction::construct_captured_layouts(
                        &ModuleStorageWrapper {
                            module_storage: self.module_storage,
                        },
                        &mut DependencyGasMeterWrapper {
                            gas_meter: self.gas_meter,
                        },
                        self.traversal_context,
                        fun,
                        *mask,
                    )?,
                },
            },
        )
    }
}

// Wrappers to use trait objects where static dispatch is expected.
struct ModuleStorageWrapper<'a> {
    module_storage: &'a dyn ModuleStorage,
}

#[delegate_to_methods]
#[delegate(WithRuntimeEnvironment, target_ref = "inner")]
#[delegate(ModuleStorage, target_ref = "inner")]
impl<'a> ModuleStorageWrapper<'a> {
    fn inner(&self) -> &dyn ModuleStorage {
        self.module_storage
    }
}

struct DependencyGasMeterWrapper<'a> {
    gas_meter: &'a mut dyn DependencyGasMeter,
}

#[delegate_to_methods]
#[delegate(DependencyGasMeter, target_mut = "inner_mut")]
impl<'a> DependencyGasMeterWrapper<'a> {
    fn inner_mut(&mut self) -> &mut dyn DependencyGasMeter {
        self.gas_meter
    }
}
