// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage::loader::traits::{ModuleMetadataLoader, StructDefinitionLoader},
    ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_vm_types::{
    gas::{GasMeter, ModuleTraversalContext},
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
};
use std::sync::Arc;

/// Loader implementation used for contexts where there is no metering gas for modules. With lazy
/// loading additional checks are performed: any accessed module must be visited in the traversal
/// context.
pub struct NativeLoader<'a> {
    module_storage: &'a dyn ModuleStorage,
}

impl<'a> NativeLoader<'a> {
    /// Returns a new native loader.
    pub fn new(module_storage: &'a dyn ModuleStorage) -> Self {
        Self { module_storage }
    }
}

impl<'a> WithRuntimeEnvironment for NativeLoader<'a> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.module_storage.runtime_environment()
    }
}

impl<'a> StructDefinitionLoader for NativeLoader<'a> {
    fn is_lazy_loading_enabled(&self) -> bool {
        self.runtime_environment().vm_config().enable_lazy_loading
    }

    fn load_struct_definition(
        &self,
        _gas_meter: &mut impl GasMeter,
        traversal_context: &mut impl ModuleTraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        let module_result = if self.is_lazy_loading_enabled() {
            traversal_context.check_is_special_or_visited(
                struct_name.module.address(),
                struct_name.module.name(),
            )?;
            self.module_storage
                .unmetered_get_existing_lazily_verified_module(&struct_name.module)
        } else {
            self.module_storage
                .unmetered_get_existing_eagerly_verified_module(
                    struct_name.module.address(),
                    struct_name.module.name(),
                )
        };

        module_result
            .and_then(|module| module.get_struct(&struct_name.name))
            .map_err(|err| err.to_partial())
    }
}

impl<'a> ModuleMetadataLoader for NativeLoader<'a> {
    fn load_module_metadata(
        &self,
        _gas_meter: &mut impl GasMeter,
        traversal_context: &mut impl ModuleTraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        // If used for lazy loading in native context, ensure module has been charged gas.
        if self.is_lazy_loading_enabled() {
            traversal_context.check_is_special_or_visited(module_id.address(), module_id.name())?;
        }

        self.module_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        config::VMConfig,
        loader::StructDef,
        module_traversal::{TraversalContext, TraversalStorage},
        Module,
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok};
    use move_binary_format::{errors::VMResult, file_format::empty_module, CompiledModule};
    use move_core_types::{
        account_address::AccountAddress, ident_str, identifier::IdentStr,
        language_storage::ModuleId, metadata::Metadata,
    };
    use move_vm_types::{gas::UnmeteredGasMeter, loaded_data::runtime_types::StructIdentifier};
    use std::str::FromStr;

    struct MockModuleStorage {
        runtime_environment: RuntimeEnvironment,
        module: Arc<Module>,
    }

    fn dummy_struct_module_id() -> ModuleId {
        ModuleId::new(
            AccountAddress::from_str("0x123").unwrap(),
            ident_str!("foo").to_owned(),
        )
    }

    impl MockModuleStorage {
        fn new(enable_lazy_loading: bool) -> Self {
            let vm_config = VMConfig {
                paranoid_type_checks: true,
                enable_lazy_loading,
                ..VMConfig::default()
            };
            let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);

            let dummy_struct_name = StructIdentifier {
                module: dummy_struct_module_id(),
                name: ident_str!("Bar").to_owned(),
            };
            let idx = assert_ok!(runtime_environment
                .struct_name_index_map()
                .struct_name_to_idx(&dummy_struct_name));
            let mut struct_definition = StructType::for_test();
            struct_definition.idx = idx;

            // Note: need to patch module name because empty module is created with some dummy id.
            let mut compiled_module = empty_module();
            compiled_module.address_identifiers = vec![*dummy_struct_module_id().address()];
            compiled_module.identifiers = vec![dummy_struct_module_id().name().to_owned()];

            let mut module = assert_ok!(Module::new(
                runtime_environment.natives(),
                10,
                Arc::new(compiled_module),
                runtime_environment.struct_name_index_map(),
            ));

            module.struct_map.insert(ident_str!("Bar").to_owned(), 0);
            module.structs.push(StructDef {
                field_count: 0,
                definition_struct_type: Arc::new(struct_definition),
            });

            Self {
                runtime_environment,
                module: Arc::new(module),
            }
        }

        fn dummy_struct_idx(&self) -> StructNameIndex {
            self.module.structs[0].definition_struct_type.idx
        }
    }

    impl WithRuntimeEnvironment for MockModuleStorage {
        fn runtime_environment(&self) -> &RuntimeEnvironment {
            &self.runtime_environment
        }
    }

    impl ModuleStorage for MockModuleStorage {
        fn unmetered_check_module_exists(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<bool> {
            unreachable!("Irrelevant for tests")
        }

        fn unmetered_get_module_bytes(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<Option<Bytes>> {
            unreachable!("Irrelevant for tests")
        }

        fn unmetered_get_module_size(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<Option<usize>> {
            unreachable!("Irrelevant for tests")
        }

        fn unmetered_get_module_metadata(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<Option<Vec<Metadata>>> {
            Ok(Some(vec![]))
        }

        fn unmetered_get_deserialized_module(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<Option<Arc<CompiledModule>>> {
            unreachable!("Irrelevant for tests")
        }

        fn unmetered_get_eagerly_verified_module(
            &self,
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> VMResult<Option<Arc<Module>>> {
            Ok(Some(self.module.clone()))
        }

        fn unmetered_get_lazily_verified_module(
            &self,
            _module_id: &ModuleId,
        ) -> VMResult<Option<Arc<Module>>> {
            Ok(Some(self.module.clone()))
        }
    }

    #[test]
    fn test_eager_loader_no_checks() {
        let storage = MockModuleStorage::new(false);
        let idx = storage.dummy_struct_idx();

        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = NativeLoader::new(&storage);
        assert!(!loader.is_lazy_loading_enabled());

        assert_ok!(loader.load_struct_definition(&mut gas_meter, &mut traversal_context, &idx,));
        assert_ok!(loader.load_module_metadata(
            &mut gas_meter,
            &mut traversal_context,
            &dummy_struct_module_id()
        ));
    }

    #[test]
    fn test_lazy_loader_checks() {
        let storage = MockModuleStorage::new(true);
        let idx = storage.dummy_struct_idx();
        let module_id = dummy_struct_module_id();

        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = NativeLoader::new(&storage);
        assert!(loader.is_lazy_loading_enabled());

        // Not visited, loader returns an error.
        assert_err!(loader.load_struct_definition(&mut gas_meter, &mut traversal_context, &idx,));
        assert_err!(loader.load_module_metadata(
            &mut gas_meter,
            &mut traversal_context,
            &module_id
        ));

        let not_visited = assert_ok!(traversal_context.visit_if_not_special_module_id(&module_id));
        assert!(not_visited);

        assert_ok!(loader.load_struct_definition(&mut gas_meter, &mut traversal_context, &idx,));
        assert_ok!(loader.load_module_metadata(&mut gas_meter, &mut traversal_context, &module_id));
    }
}
