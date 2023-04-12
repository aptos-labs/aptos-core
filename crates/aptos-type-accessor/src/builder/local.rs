// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::{parse_module, TypeAccessorBuilderTrait};
use crate::accessor::TypeAccessor;
use anyhow::{bail, Result};
use aptos_api_types::{MoveModule, MoveType};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// This builder operates only with modules provided to it directly. It is not able to
/// look up modules, so `build` will fail if it encounters a module that wasn't
/// registered in advance. If you want a TypeAccessorBuilder that can look up modules
/// as it encounters them, use [`crate::TypeAccessorBuilderRemote`].
#[derive(Clone, Debug)]
pub struct TypeAccessorBuilderLocal {
    modules: BTreeMap<ModuleId, MoveModule>,
}

impl TypeAccessorBuilderLocal {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    pub fn build(mut self) -> Result<TypeAccessor> {
        if self.modules.is_empty() {
            bail!("Cannot build TypeAccessor without any modules to lookup or add");
        }

        let mut field_info: BTreeMap<
            ModuleId,
            BTreeMap<Identifier, BTreeMap<Identifier, MoveType>>,
        > = BTreeMap::new();

        let mut modules_processed = BTreeSet::new();

        while let Some((module_id, module)) = self.modules.pop_first() {
            if modules_processed.contains(&module_id) {
                continue;
            }
            modules_processed.insert(module_id.clone());
            let (structs_info, modules_to_retrieve) = parse_module(module);

            // Filter out modules we already have / have processed.
            let modules_to_retrieve: HashSet<_> = modules_to_retrieve
                .into_iter()
                .filter(|module_id| {
                    !self.modules.contains_key(module_id) && !modules_processed.contains(module_id)
                })
                .collect();

            if !modules_to_retrieve.is_empty() {
                bail!(
                    "While processing {} references to modules not available \
                    to the builder were found: {:?}. This makes it impossible \
                    to comprehensively resolve types, and this builder is \
                    unable to look up new modules, so it is impossible to \
                    build a complete TypeAccessor.",
                    module_id,
                    modules_to_retrieve
                );
            }

            field_info.insert(module_id, structs_info);
        }

        Ok(TypeAccessor::new(field_info))
    }
}

impl TypeAccessorBuilderTrait for TypeAccessorBuilderLocal {
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

impl Default for TypeAccessorBuilderLocal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::compile_package;
    use anyhow::Context;
    use std::path::PathBuf;

    /// Test that the TypeAccessor works as expected in DoNotLookup mode.
    #[tokio::test]
    async fn test_food_no_lookup() -> Result<()> {
        // First just build the top level module in move/food.
        let food_modules = compile_package(PathBuf::try_from("move/food").unwrap())?;

        // Build a TypeAccessor with just that module. We expect this to fail, because
        // while the TypeAccessorBuilder is building it will find references to
        // modules that it doesn't know about and because it is in DoNotLookup mode,
        // it will return an error.
        let type_accessor_result = TypeAccessorBuilderLocal::new()
            .add_modules(food_modules.clone())
            .build();
        assert!(
            type_accessor_result.is_err(),
            "Expected TypeAccessorBuilder to fail because we didn't give it all the modules it needs"
        );

        let mut modules = food_modules;

        // Compile the modules that we know food recursively depends on.
        for name in &["aptos-framework", "aptos-stdlib", "move-stdlib"] {
            let path = PathBuf::try_from(format!("../../aptos-move/framework/{}", name)).unwrap();
            modules.extend(compile_package(path)?);
        }
        modules.extend(compile_package(PathBuf::try_from("move/food").unwrap())?);

        // Build a new type accessor with all of those modules, which we expect to succeed.
        TypeAccessorBuilderLocal::new()
            .add_modules(modules)
            .build()
            .context("Failed to build TypeAccessor with all the modules")?;

        Ok(())
    }
}
