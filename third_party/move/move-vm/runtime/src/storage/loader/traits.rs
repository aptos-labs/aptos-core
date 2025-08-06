// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext, Function, LoadedFunction, LoadedFunctionOwner, Module,
    ModuleStorage, WithRuntimeEnvironment,
};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    vm_status::{sub_status::type_resolution_failure::EUSER_TYPE_LOADING_FAILURE, StatusCode},
};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
};
use std::{rc::Rc, sync::Arc};

/// Provides access to struct definitions.
pub trait StructDefinitionLoader: WithRuntimeEnvironment {
    /// Returns true if the current loader is lazy, and false otherwise.
    fn is_lazy_loading_enabled(&self) -> bool;

    /// Returns the struct definition corresponding to the specified index. The function may also
    /// charge gas for loading the module where the struct is defined. Returns an error if such
    /// metering fails, or if the struct / module where it is defined do not exist.
    fn load_struct_definition(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>>;
}

/// Provides access to function definitions.
pub trait FunctionDefinitionLoader {
    /// Returns the function definition corresponding to the specified name. Also returns the
    /// module where this function is defined (verified). Returns an error if module or function
    /// does not exist. Charges gas for module access.
    fn load_function_definition(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
    ) -> VMResult<(Arc<Module>, Arc<Function>)>;
}

/// Charges gas for native module loading.
pub trait NativeModuleLoader {
    /// Charges gas when [move_vm_types::natives::function::NativeResult::LoadModule]) is returned
    /// from the native context.
    fn charge_native_result_load_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<()>;
}

/// Provides access to module metadata.
pub trait ModuleMetadataLoader {
    /// Loads the module metadata, ensuring the module access gets charged. Returns an error if
    /// out-of-gas, module does not exist, or if there is some miscellaneous storage error.
    fn load_module_metadata(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>>;
}

/// Configuration used by legacy eager loader only. Used to allow single implementation for both
/// metered and not metered entrypoints like entry functions or scripts.
pub struct LegacyLoaderConfig {
    /// If true, charge gas for transitive dependencies of a function or a script.
    pub charge_for_dependencies: bool,
    /// If true, charge gas for all modules used in type arguments (tags) of a function / script.
    pub charge_for_ty_tag_dependencies: bool,
}

impl LegacyLoaderConfig {
    /// Returns config which does not charge for anything.
    pub fn unmetered() -> Self {
        Self {
            charge_for_dependencies: false,
            charge_for_ty_tag_dependencies: false,
        }
    }
}

/// Private helper trait common for eager and lazy loaders when instantiating a function.
pub(crate) trait InstantiatedFunctionLoaderHelper {
    /// Loads a single type argument for the function instantiation, converting the type tag into
    /// a runtime type instance.
    fn load_ty_arg(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty_arg: &TypeTag,
    ) -> VMResult<Type>;

    /// Helper to construct a loaded function instance given the function and its module.
    fn build_instantiated_function(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module: Arc<Module>,
        function: Arc<Function>,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let ty_args = ty_args
            .iter()
            .map(|ty_arg| self.load_ty_arg(gas_meter, traversal_context, ty_arg))
            .collect::<VMResult<Vec<_>>>()
            .map_err(|mut err| {
                // User provided type argument failed to load. Set extra sub status to distinguish
                // from internal type loading error.
                if StatusCode::TYPE_RESOLUTION_FAILURE == err.major_status() {
                    err.set_sub_status(EUSER_TYPE_LOADING_FAILURE);
                }
                err
            })?;

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module.self_id().clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

/// Allows to load function instantiations, resolving function type arguments.
#[allow(private_bounds)]
pub trait InstantiatedFunctionLoader: InstantiatedFunctionLoaderHelper {
    /// Loads function definition, converts type argument tags to runtime types, to obtain a
    /// [LoadedFunction]. All module accesses are metered here with lazy loading. With eager
    /// loading, configuration specifies some of the metering.
    fn load_instantiated_function(
        &self,
        // Only used for eager loader!
        config: &LegacyLoaderConfig,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction>;
}

/// Resolves closures into loaded functions.
pub trait ClosureLoader: InstantiatedFunctionLoader {
    fn load_closure(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> PartialVMResult<Rc<LoadedFunction>> {
        self.load_instantiated_function(
            // Note: lazy loader ignores this config.
            &LegacyLoaderConfig {
                charge_for_dependencies: true,
                charge_for_ty_tag_dependencies: true,
            },
            gas_meter,
            traversal_context,
            module_id,
            function_name,
            ty_args,
        )
        .map_err(|err| err.to_partial())
        .map(Rc::new)
    }
}

impl<T> ClosureLoader for T where T: InstantiatedFunctionLoader {}

/// Encapsulates all possible module accesses in a safe, gas-metered way. This trait (and more
/// fine-grained) traits should be used when working with modules, functions, structs, and other
/// module information.
pub trait Loader:
    ClosureLoader
    + FunctionDefinitionLoader
    + ModuleMetadataLoader
    + NativeModuleLoader
    + StructDefinitionLoader
    + InstantiatedFunctionLoader
{
    /// **USE WITH CAUTION**
    ///
    /// Allows to convert loader to raw module storage which does not enforce gas metering for any
    /// module access! Used to pass to native context. Any other use-cases are discouraged.
    fn unmetered_module_storage(&self) -> &dyn ModuleStorage;
}
