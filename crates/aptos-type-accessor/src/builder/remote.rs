// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::{parse_module, TypeAccessorBuilderTrait};
use crate::accessor::TypeAccessor;
use anyhow::{bail, Context, Result};
use aptos_api_types::{MoveModule, MoveType};
use aptos_rest_client::Client as RestClient;
use move_binary_format::file_format::CompiledModule;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// This builder is able to look up modules as it encounters them. This way we can
/// ensure that the resulting TypeAccessor will be able to resolve information about
/// every type that recursively appears in the types of the modules that were initially
/// registered with the builder.
#[derive(Clone, Debug)]
pub struct TypeAccessorBuilderRemote {
    modules_to_retrieve: BTreeSet<ModuleId>,
    modules: BTreeMap<ModuleId, MoveModule>,
    rest_client: Arc<RestClient>,
}

impl TypeAccessorBuilderRemote {
    pub fn new(rest_client: Arc<RestClient>) -> Self {
        Self {
            modules_to_retrieve: BTreeSet::new(),
            modules: BTreeMap::new(),
            rest_client,
        }
    }

    pub async fn build(mut self) -> anyhow::Result<TypeAccessor> {
        if self.modules_to_retrieve.is_empty() && self.modules.is_empty() {
            bail!("Cannot build TypeAccessor without any modules to lookup or add");
        }

        let mut field_info: BTreeMap<
            ModuleId,
            BTreeMap<Identifier, BTreeMap<Identifier, MoveType>>,
        > = BTreeMap::new();

        let mut modules_processed = BTreeSet::new();

        loop {
            if !self.modules_to_retrieve.is_empty() {
                while let Some(module_id) = self.modules_to_retrieve.pop_first() {
                    if self.modules.contains_key(&module_id) {
                        continue;
                    }
                    self.modules
                        .insert(module_id.clone(), self.retrieve_module(module_id).await?);
                }
            } else if !self.modules.is_empty() {
                while let Some((module_id, module)) = self.modules.pop_first() {
                    if modules_processed.contains(&module_id) {
                        continue;
                    }
                    modules_processed.insert(module_id.clone());
                    let (structs_info, modules_to_retrieve) = parse_module(module);

                    self.modules_to_retrieve.extend(modules_to_retrieve);

                    field_info.insert(module_id, structs_info);
                }
            } else {
                break;
            }
        }

        Ok(TypeAccessor::new(field_info))
    }

    async fn retrieve_module(&self, module_id: ModuleId) -> Result<MoveModule> {
        let module_bytecode = self
            .rest_client
            .get_account_module_bcs(*module_id.address(), module_id.name().as_str())
            .await
            .context(format!(
                "Failed to get module {}::{}",
                module_id.address(),
                module_id.name()
            ))?
            .into_inner();

        let module: MoveModule = CompiledModule::deserialize(&module_bytecode)
            .context(format!(
                "Failed to deserialize module {}::{}",
                module_id.address(),
                module_id.name()
            ))?
            .into();

        Ok(module)
    }

    /// Add modules to look up.
    pub fn lookup_modules(mut self, module_ids: Vec<ModuleId>) -> Self {
        self.modules_to_retrieve.extend(module_ids);
        self
    }

    /// Add a module to look up.
    pub fn lookup_module(self, module_id: ModuleId) -> Self {
        self.lookup_modules(vec![module_id])
    }
}

impl TypeAccessorBuilderTrait for TypeAccessorBuilderRemote {
    fn add_modules(mut self, modules: Vec<MoveModule>) -> Self {
        for module in modules {
            self.modules.insert(
                ModuleId::new(module.address.into(), module.name.clone().into()),
                module,
            );
        }
        self
    }

    fn add_module(self, module: MoveModule) -> Self {
        self.add_modules(vec![module])
    }
}
