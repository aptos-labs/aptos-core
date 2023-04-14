// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::ModuleRetrieverTrait;
use anyhow::{Context, Result};
use aptos_api_types::MoveModule;
use aptos_rest_client::Client;
use async_trait::async_trait;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct ApiModuleRetriever {
    api_client: Arc<Client>,
}

impl ApiModuleRetriever {
    pub fn new(api_client: Arc<Client>) -> Self {
        Self { api_client }
    }
}

#[async_trait]
impl ModuleRetrieverTrait for ApiModuleRetriever {
    async fn retrieve_modules(
        &self,
        module_ids: &[ModuleId],
    ) -> Result<BTreeMap<ModuleId, MoveModule>> {
        let mut modules = BTreeMap::new();

        for module_id in module_ids {
            let module = self
                .api_client
                .get_account_module_bcs(*module_id.address(), module_id.name().as_str())
                .await
                .context(format!(
                    "Failed to get module {}::{}",
                    module_id.address(),
                    module_id.name()
                ))?
                .into_inner();

            let module: MoveModule = CompiledModule::deserialize(&module)
                .context(format!(
                    "Failed to deserialize module {}::{}",
                    module_id.address(),
                    module_id.name()
                ))?
                .into();

            modules.insert(module_id.clone(), module);
        }

        Ok(modules)
    }

    async fn retrieve_module(&self, module_id: ModuleId) -> Result<MoveModule> {
        Ok(self
            .retrieve_modules(vec![module_id].as_slice())
            .await?
            .pop_first()
            .unwrap()
            .1)
    }
}
