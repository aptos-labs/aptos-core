// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use velor_types::{
    on_chain_config::{Features, OnChainConfig},
    state_store::{state_key::StateKey, StateView},
};
use velor_vm_environment::prod_configs::velor_prod_deserializer_config;
use move_binary_format::{deserializer::DeserializerConfig, CompiledModule};
use move_bytecode_utils::compiled_module_viewer::CompiledModuleView;
use move_core_types::language_storage::ModuleId;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

pub struct ModuleView<'a, S> {
    module_cache: RefCell<HashMap<ModuleId, Arc<CompiledModule>>>,
    deserializer_config: DeserializerConfig,
    state_view: &'a S,
}

impl<'a, S: StateView> ModuleView<'a, S> {
    pub fn new(state_view: &'a S) -> Self {
        let features = Features::fetch_config(state_view).unwrap_or_default();
        let deserializer_config = velor_prod_deserializer_config(&features);

        Self {
            module_cache: RefCell::new(HashMap::new()),
            deserializer_config,
            state_view,
        }
    }
}

impl<S: StateView> CompiledModuleView for ModuleView<'_, S> {
    type Item = Arc<CompiledModule>;

    fn view_compiled_module(&self, module_id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        let mut module_cache = self.module_cache.borrow_mut();
        if let Some(module) = module_cache.get(module_id) {
            return Ok(Some(module.clone()));
        }

        let state_key = StateKey::module_id(module_id);
        Ok(
            match self
                .state_view
                .get_state_value_bytes(&state_key)
                .map_err(|e| anyhow!("Error retrieving module {:?}: {:?}", module_id, e))?
            {
                Some(bytes) => {
                    let compiled_module =
                        CompiledModule::deserialize_with_config(&bytes, &self.deserializer_config)
                            .map_err(|status| {
                                anyhow!(
                                    "Module {:?} deserialize with error code {:?}",
                                    module_id,
                                    status
                                )
                            })?;

                    let compiled_module = Arc::new(compiled_module);
                    module_cache.insert(module_id.clone(), compiled_module.clone());
                    Some(compiled_module)
                },
                None => None,
            },
        )
    }
}
