// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{common::parse_module, TypeAccessorBuilderTrait};
use crate::{
    accessor::TypeAccessor,
    module_retriever::{ModuleRetriever, ModuleRetrieverTrait},
};
use anyhow::bail;
use aptos_api_types::{MoveModule, MoveType};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::collections::{BTreeMap, BTreeSet};

/// This builder is able to look up modules as it encounters them. This way we can
/// ensure that the resulting TypeAccessor will be able to resolve information about
/// every type that recursively appears in the types of the modules that were initially
/// registered with the builder.
#[derive(Clone, Debug)]
pub struct RemoteTypeAccessorBuilder {
    modules_to_retrieve: BTreeSet<ModuleId>,
    modules: BTreeMap<ModuleId, MoveModule>,
    module_retriever: ModuleRetriever,
}

impl RemoteTypeAccessorBuilder {
    pub fn new(module_retriever: ModuleRetriever) -> Self {
        Self {
            modules_to_retrieve: BTreeSet::new(),
            modules: BTreeMap::new(),
            module_retriever,
        }
    }

    /// Build a TypeAccessor. All modules in `modules_to_retrieve` will be retrieved
    /// using the ModuleRetriever and added to `modules`. By the end of the build,
    /// `modules_to_retrieve` will be empty.
    ///
    /// This choice is intentional as it allows the TypeAccessorBuilder to serve as
    /// a local cache for modules that have already been retrieved. Read more about
    /// the applcations of this in TODO.
    pub async fn build(&mut self) -> anyhow::Result<TypeAccessor> {
        if self.modules_to_retrieve.is_empty() && self.modules.is_empty() {
            bail!("Cannot build TypeAccessor without any modules to lookup or add");
        }

        let mut field_info: BTreeMap<
            ModuleId,
            BTreeMap<Identifier, BTreeMap<Identifier, MoveType>>,
        > = BTreeMap::new();

        let mut modules_processed = BTreeSet::new();

        loop {
            // First retrieve any modules we need to retrieve.
            if !self.modules_to_retrieve.is_empty() {
                while let Some(module_id) = self.modules_to_retrieve.pop_first() {
                    if self.modules.contains_key(&module_id) {
                        continue;
                    }
                    self.modules.insert(
                        module_id.clone(),
                        self.module_retriever.retrieve_module(module_id).await?,
                    );
                }
                continue;
            }

            let modules_to_process = self
                .modules
                .iter()
                .filter(|(module_id, _)| !modules_processed.contains(*module_id))
                .collect::<BTreeMap<_, _>>();

            // If there are no modules to process, break.
            if modules_to_process.is_empty() {
                break;
            }

            // There are modules to process, let's do that.
            for (module_id, module) in modules_to_process {
                let (structs_info, modules_to_retrieve) = parse_module(module);
                field_info.insert(module_id.clone(), structs_info);
                self.modules_to_retrieve.extend(modules_to_retrieve);
                modules_processed.insert(module_id.clone());
            }
        }

        Ok(TypeAccessor::new(field_info))
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

impl TypeAccessorBuilderTrait for RemoteTypeAccessorBuilder {
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
