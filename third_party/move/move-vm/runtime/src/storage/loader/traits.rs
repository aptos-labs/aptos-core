// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{module_traversal::TraversalContext, WithRuntimeEnvironment};
use move_binary_format::errors::PartialVMResult;
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
};
use std::sync::Arc;

/// Provides access to struct definitions.
pub trait StructDefinitionLoader: WithRuntimeEnvironment {
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

/// Encapsulates all possible module accesses in a safe, gas-metered way. This trait (and more
/// fine-grained) traits should be used when working with modules, functions, structs, and other
/// module information.
pub trait Loader: StructDefinitionLoader {}
