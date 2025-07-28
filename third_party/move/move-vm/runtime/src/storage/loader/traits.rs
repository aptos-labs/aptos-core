// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{module_traversal::TraversalContext, WithRuntimeEnvironment};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
};
use std::sync::Arc;

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

/// Encapsulates all possible module accesses in a safe, gas-metered way. This trait (and more
/// fine-grained) traits should be used when working with modules, functions, structs, and other
/// module information.
pub trait Loader: StructDefinitionLoader + NativeModuleLoader + ModuleMetadataLoader {}
