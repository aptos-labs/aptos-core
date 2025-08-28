// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, module_traversal::TraversalContext,
    storage::loader::traits::StructDefinitionLoader, RuntimeEnvironment, WithRuntimeEnvironment,
};
use claims::assert_none;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    ability::AbilitySet, identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode,
};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{AbilityInfo, StructIdentifier, StructLayout, StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
};
use smallbitvec::SmallBitVec;
use std::{cell::RefCell, collections::HashMap, str::FromStr, sync::Arc};

/// Creates a dummy struct definition.
pub(crate) fn struct_definition(
    idx: StructNameIndex,
    fields: Vec<(Identifier, Type)>,
) -> StructType {
    StructType {
        idx,
        layout: StructLayout::Single(fields),
        // Below is irrelevant for tests.
        ty_params: vec![],
        phantom_ty_params_mask: SmallBitVec::default(),
        abilities: AbilitySet::EMPTY,
    }
}

/// Creates a dummy enum definition.
pub(crate) fn enum_definition(
    idx: StructNameIndex,
    variants: Vec<(Identifier, Vec<(Identifier, Type)>)>,
) -> StructType {
    StructType {
        idx,
        layout: StructLayout::Variants(variants),
        // Below is irrelevant for tests.
        ty_params: vec![],
        phantom_ty_params_mask: SmallBitVec::default(),
        abilities: AbilitySet::EMPTY,
    }
}

/// Creates a dummy struct (or enum) type.
pub(crate) fn struct_ty(idx: StructNameIndex) -> Type {
    Type::Struct {
        idx,
        // Below is irrelevant for tests.
        ability: AbilityInfo::struct_(AbilitySet::EMPTY),
    }
}

/// Creates a dummy generic struct (or enum) type.
pub(crate) fn generic_struct_ty(idx: StructNameIndex, ty_args: Vec<Type>) -> Type {
    Type::StructInstantiation {
        idx,
        ty_args: triomphe::Arc::new(ty_args),
        // Below is irrelevant for tests.
        ability: AbilityInfo::struct_(AbilitySet::EMPTY),
    }
}

/// Mocks struct definition loading, by holding a cache of [StructType]s and runtime environment.
pub(crate) struct MockStructDefinitionLoader {
    runtime_environment: RuntimeEnvironment,
    struct_definitions: RefCell<HashMap<StructNameIndex, Arc<StructType>>>,
}

impl MockStructDefinitionLoader {
    pub(crate) fn new_with_config(config: VMConfig) -> Self {
        Self {
            runtime_environment: RuntimeEnvironment::new_with_config(vec![], config),
            struct_definitions: RefCell::new(HashMap::new()),
        }
    }

    /// Returns an index for a struct name. The struct name is added to the environment.
    pub(crate) fn get_struct_identifier(&self, struct_name: &str) -> StructNameIndex {
        let struct_identifier = StructIdentifier {
            module: ModuleId::from_str("0x1::foo").unwrap(),
            name: Identifier::from_str(struct_name).unwrap(),
        };
        self.runtime_environment
            .struct_name_to_idx_for_test(struct_identifier)
            .unwrap()
    }

    /// Adds a dummy struct to the mock cache.
    pub(crate) fn add_struct<'a>(
        &self,
        struct_name: &str,
        field_struct_names: impl IntoIterator<Item = (&'a str, Type)>,
    ) {
        let fields = field_struct_names
            .into_iter()
            .map(|(name, field_ty)| {
                let field_name = Identifier::from_str(name).unwrap();
                (field_name, field_ty)
            })
            .collect();

        let idx = self.get_struct_identifier(struct_name);
        let struct_definition = struct_definition(idx, fields);

        assert_none!(self
            .struct_definitions
            .borrow_mut()
            .insert(struct_definition.idx, Arc::new(struct_definition)));
    }

    /// Adds a dummy enum to the mock cache.
    pub(crate) fn add_enum<'a>(
        &self,
        struct_name: &str,
        variant_field_struct_names: impl IntoIterator<Item = (&'a str, Vec<(&'a str, Type)>)>,
    ) {
        let variants = variant_field_struct_names
            .into_iter()
            .map(|(variant, fields)| {
                let variant_name = Identifier::from_str(variant).unwrap();
                let fields = fields
                    .into_iter()
                    .map(|(name, field_ty)| {
                        let field_name = Identifier::from_str(name).unwrap();
                        (field_name, field_ty)
                    })
                    .collect::<Vec<_>>();
                (variant_name, fields)
            })
            .collect();

        let idx = self.get_struct_identifier(struct_name);
        let struct_definition = enum_definition(idx, variants);

        assert_none!(self
            .struct_definitions
            .borrow_mut()
            .insert(struct_definition.idx, Arc::new(struct_definition)));
    }
}

impl Default for MockStructDefinitionLoader {
    fn default() -> Self {
        Self {
            runtime_environment: RuntimeEnvironment::new(vec![]),
            struct_definitions: RefCell::new(HashMap::new()),
        }
    }
}

impl WithRuntimeEnvironment for MockStructDefinitionLoader {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        &self.runtime_environment
    }
}

impl StructDefinitionLoader for MockStructDefinitionLoader {
    fn is_lazy_loading_enabled(&self) -> bool {
        self.runtime_environment().vm_config().enable_lazy_loading
    }

    fn load_struct_definition(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        self.struct_definitions
            .borrow()
            .get(idx)
            .cloned()
            .ok_or_else(|| PartialVMError::new(StatusCode::LINKER_ERROR))
    }
}
