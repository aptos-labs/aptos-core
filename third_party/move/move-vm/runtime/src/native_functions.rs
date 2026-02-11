// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ambassador_impl_ModuleStorage, ambassador_impl_WithRuntimeEnvironment,
    data_cache::NativeContextMoveVmDataCache,
    dispatch_loader,
    interpreter::InterpreterDebugInterface,
    loader::{LazyLoadedFunction, LazyLoadedFunctionState},
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    storage::{
        layout_cache::StructKey,
        loader::traits::NativeModuleLoader,
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::{LayoutConverter, LayoutWithDelayedFields},
    },
    Function, FunctionDefinitionLoader, LayoutCache, LayoutCacheEntry, LoadedFunction,
    LoadedFunctionOwner, Module, ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
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
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{ambassador_impl_DependencyGasMeter, DependencyGasMeter, DependencyKind, NativeGasMeter},
    loaded_data::runtime_types::{Type, TypeParamMap},
    natives::function::NativeResult,
    values::{AbstractFunction, Value},
};
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
};
use triomphe::Arc as TriompheArc;

pub type UnboxedNativeFunction = dyn for<'a> Fn(&mut NativeContext, &'a [Type], VecDeque<Value>) -> PartialVMResult<NativeResult>
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
    data_cache: &'a mut dyn NativeContextMoveVmDataCache,
    module_storage: &'a dyn ModuleStorage,
    extensions: &'a mut NativeContextExtensions<'b>,
    gas_meter: &'a mut dyn NativeGasMeter,
    traversal_context: &'a mut TraversalContext<'c>,
}

impl<'a, 'b, 'c> NativeContext<'a, 'b, 'c> {
    pub(crate) fn new(
        interpreter: &'a dyn InterpreterDebugInterface,
        data_cache: &'a mut dyn NativeContextMoveVmDataCache,
        module_storage: &'a dyn ModuleStorage,
        extensions: &'a mut NativeContextExtensions<'b>,
        gas_meter: &'a mut dyn NativeGasMeter,
        traversal_context: &'a mut TraversalContext<'c>,
    ) -> Self {
        Self {
            interpreter,
            data_cache,
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
        self.data_cache.native_check_resource_exists(
            self.gas_meter,
            self.traversal_context,
            &address,
            ty,
        )
    }

    /// Borrows an immutable reference to a resource in global storage.
    /// Returns the reference value and the number of bytes loaded.
    pub fn borrow_resource(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(Value, Option<NumBytes>)> {
        self.data_cache
            .native_borrow_resource(self.gas_meter, self.traversal_context, &address, ty)
    }

    /// Borrows a mutable reference to a resource in global storage.
    /// Returns the reference value and the number of bytes loaded.
    pub fn borrow_resource_mut(
        &mut self,
        address: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(Value, Option<NumBytes>)> {
        self.data_cache.native_borrow_resource_mut(
            self.gas_meter,
            self.traversal_context,
            &address,
            ty,
        )
    }

    pub fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        self.module_storage.runtime_environment().ty_to_ty_tag(ty)
    }

    /// Returns the runtime layout of a type that can be used to (de)serialize the value.
    ///
    /// NOTE: use with caution as this ignores the flag if layout contains delayed fields or not.
    pub fn type_to_type_layout(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<TriompheArc<MoveTypeLayout>> {
        let layout = self
            .loader_context()
            .type_to_type_layout_with_delayed_fields(ty)?
            .unpack()
            .0;
        Ok(layout)
    }

    /// Returns the runtime layout of a type that can be used to (de)serialize the value. Also,
    /// information whether there are any delayed fields in layouts is returned.
    pub fn type_to_type_layout_with_delayed_fields(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        self.loader_context()
            .type_to_type_layout_with_delayed_fields(ty)
    }

    /// Returns the runtime layout of a type that can be used to (de)serialize the value. The
    /// layout does not contain delayed fields (otherwise, invariant violation is returned).
    pub fn type_to_type_layout_check_no_delayed_fields(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<TriompheArc<MoveTypeLayout>> {
        let layout = self
            .loader_context()
            .type_to_type_layout_with_delayed_fields(ty)?;
        layout
            .into_layout_when_has_no_delayed_fields()
            .ok_or_else(|| {
                PartialVMError::new_invariant_violation("Layout should not contain delayed fields")
            })
    }

    /// Returns the annotated layout of a type. If type contains delayed fields, returns [None].
    /// An error is returned when a layout cannot be constructed (e.g., some limits on number of
    /// nodes are reached, or an internal invariant violation is raised).
    pub fn type_to_fully_annotated_layout(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<Option<TriompheArc<MoveTypeLayout>>> {
        self.loader_context().type_to_fully_annotated_layout(ty)
    }

    pub fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }

    pub fn extensions(&self) -> &NativeContextExtensions<'b> {
        self.extensions
    }

    /// Returns native extensions and loader context (storing mutable gas meter and traversal
    /// context). Native functions can use this method to query extensions while holding mutable
    /// reference to loader's context.
    pub fn extensions_with_loader_context(
        &mut self,
    ) -> (&NativeContextExtensions<'b>, LoaderContext<'_, 'c>) {
        (
            self.extensions,
            LoaderContext::new(self.module_storage, self.gas_meter, self.traversal_context),
        )
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

    /// Returns the loader context used by the natives.
    pub fn loader_context(&mut self) -> LoaderContext<'_, 'c> {
        LoaderContext::new(self.module_storage, self.gas_meter, self.traversal_context)
    }

    pub fn traversal_context(&self) -> &TraversalContext<'c> {
        self.traversal_context
    }

    pub fn function_value_extension(&self) -> FunctionValueExtensionAdapter<'_> {
        FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        }
    }
}

/// Helper struct that can be returned together with extensions so that layouts can be constructed
/// while there is a live mutable reference to context extensions.
pub struct LoaderContext<'a, 'b> {
    module_storage: ModuleStorageWrapper<'a>,
    gas_meter: DependencyGasMeterWrapper<'a>,
    traversal_context: &'a mut TraversalContext<'b>,
}

/// Error returned by `LoaderContext::resolve_function`
pub enum FunctionResolutionError {
    Reserved = 0x0,
    FunctionNotFound = 0x1,
    FunctionNotAccessible = 0x2,
    FunctionIncompatibleType = 0x3,
    FunctionNotInstantiated = 0x4,
}

impl<'a, 'b> LoaderContext<'a, 'b> {
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
                    None => dispatch_loader!(&self.module_storage, loader, {
                        LazyLoadedFunction::construct_captured_layouts(
                            &LayoutConverter::new(&loader),
                            &mut self.gas_meter,
                            self.traversal_context,
                            fun,
                            *mask,
                        )
                    })?,
                },
            },
        )
    }

    /// Charges gas for module dependencies for native dynamic dispatch.
    pub fn charge_gas_for_dependencies(&mut self, module_id: ModuleId) -> PartialVMResult<()> {
        dispatch_loader!(&self.module_storage, loader, {
            loader.charge_native_result_load_module(
                &mut self.gas_meter,
                self.traversal_context,
                &module_id,
            )
        })
    }

    /// Returns function value extension that can be used for (de)serializing function values.
    pub fn function_value_extension(&self) -> FunctionValueExtensionAdapter<'_> {
        FunctionValueExtensionAdapter {
            module_storage: self.module_storage.module_storage,
        }
    }

    /// Converts a runtime type into layout for (de)serialization.
    pub fn type_to_type_layout_with_delayed_fields(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        dispatch_loader!(&self.module_storage, loader, {
            LayoutConverter::new(&loader).type_to_type_layout_with_delayed_fields(
                &mut self.gas_meter,
                self.traversal_context,
                ty,
                false,
            )
        })
    }

    /// Resolves a function by module id and function id, with expected function type,
    /// and return an abstract function which can be used to construct a closure value. This
    /// invokes the configured loader and handles gas metering.
    ///
    /// If the function exists and is public, its type will be matched against the expected
    /// type. Any type arguments will be instantiated in course of matching. Eventually,
    /// this function guarantees that the return value has indeed the expected type, including
    /// constraints on type parameters.
    pub fn resolve_function(
        &mut self,
        module_id: &ModuleId,
        fun_id: &IdentStr,
        expected_ty: &Type,
    ) -> PartialVMResult<Result<Box<dyn AbstractFunction>, FunctionResolutionError>> {
        use FunctionResolutionError::*;
        dispatch_loader!(&self.module_storage, loader, {
            match loader.load_function_definition(
                &mut self.gas_meter,
                self.traversal_context,
                module_id,
                fun_id,
            ) {
                Ok((module, function)) => self.verify_function(module, function, expected_ty),
                Err(e)
                    if e.major_status() == StatusCode::FUNCTION_RESOLUTION_FAILURE
                        || e.major_status() == StatusCode::LINKER_ERROR =>
                {
                    Ok(Err(FunctionNotFound))
                },
                Err(e) => Err(e.to_partial()),
            }
        })
    }

    fn verify_function(
        &mut self,
        module: Arc<Module>,
        func: Arc<Function>,
        expected_ty: &Type,
    ) -> PartialVMResult<Result<Box<dyn AbstractFunction>, FunctionResolutionError>> {
        use FunctionResolutionError::*;
        if !func.is_public() {
            return Ok(Err(FunctionNotAccessible));
        }
        let Type::Function {
            args,
            results,
            // Since resolved functions must be public, they always have all possible
            // abilities (store, copy, and drop), and we don't need to check with
            // expected abilities.
            abilities: _,
        } = expected_ty
        else {
            return Ok(Err(FunctionIncompatibleType));
        };
        let func_ref = func.as_ref();

        // Match types, inferring instantiation of function in `subst`.
        let mut subst = TypeParamMap::default();
        if !subst.match_tys(func_ref.param_tys.iter(), args.iter())
            || !subst.match_tys(func_ref.return_tys.iter(), results.iter())
        {
            return Ok(Err(FunctionIncompatibleType));
        }

        // Construct the type arguments from the match.
        let ty_args = match subst.verify_and_extract_type_args(func_ref.ty_param_abilities()) {
            Ok(ty_args) => ty_args,
            Err(err) => match err.major_status() {
                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH => {
                    return Ok(Err(FunctionNotInstantiated));
                },
                StatusCode::CONSTRAINT_NOT_SATISFIED => {
                    return Ok(Err(FunctionIncompatibleType));
                },
                _ => return Err(err),
            },
        };

        // Construct result.
        let env = self.module_storage.runtime_environment();
        let ty_args_id = env.ty_pool().intern_ty_args(&ty_args);
        let loaded_fun = Rc::new(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            ty_args_id,
            function: func,
        });
        Ok(Ok(Box::new(
            LazyLoadedFunction::new_resolved_not_capturing(env, loaded_fun)?,
        )))
    }
}

// Private interfaces.
impl<'a, 'b> LoaderContext<'a, 'b> {
    /// Creates a new loader context.
    fn new(
        module_storage: &'a dyn ModuleStorage,
        gas_meter: &'a mut dyn DependencyGasMeter,
        traversal_context: &'a mut TraversalContext<'b>,
    ) -> Self {
        Self {
            module_storage: ModuleStorageWrapper { module_storage },
            gas_meter: DependencyGasMeterWrapper { gas_meter },
            traversal_context,
        }
    }

    /// Converts a runtime type into decorated layout for pretty-printing.
    fn type_to_fully_annotated_layout(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<Option<TriompheArc<MoveTypeLayout>>> {
        let layout = dispatch_loader!(&self.module_storage, loader, {
            LayoutConverter::new(&loader).type_to_annotated_type_layout_with_delayed_fields(
                &mut self.gas_meter,
                self.traversal_context,
                ty,
            )
        })?;
        Ok(layout.into_layout_when_has_no_delayed_fields())
    }
}

// Wrappers to use trait objects where static dispatch is expected.
struct ModuleStorageWrapper<'a> {
    module_storage: &'a dyn ModuleStorage,
}

impl<'a> LayoutCache for ModuleStorageWrapper<'a> {
    fn get_struct_layout(&self, key: &StructKey) -> Option<LayoutCacheEntry> {
        self.module_storage.get_struct_layout(key)
    }

    fn store_struct_layout(&self, key: &StructKey, entry: LayoutCacheEntry) -> PartialVMResult<()> {
        self.module_storage.store_struct_layout(key, entry)
    }
}

#[delegate_to_methods]
#[delegate(WithRuntimeEnvironment, target_ref = "inner")]
#[delegate(ModuleStorage, target_ref = "inner")]
impl<'a> ModuleStorageWrapper<'a> {
    fn inner(&self) -> &dyn ModuleStorage {
        self.module_storage
    }
}

pub(crate) struct DependencyGasMeterWrapper<'a> {
    gas_meter: &'a mut dyn DependencyGasMeter,
}

impl<'a> DependencyGasMeterWrapper<'a> {
    pub(crate) fn new(gas_meter: &'a mut dyn DependencyGasMeter) -> Self {
        Self { gas_meter }
    }
}

#[delegate_to_methods]
#[delegate(DependencyGasMeter, target_mut = "inner_mut")]
impl<'a> DependencyGasMeterWrapper<'a> {
    fn inner_mut(&mut self) -> &mut dyn DependencyGasMeter {
        self.gas_meter
    }
}
