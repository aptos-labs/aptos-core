// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{build_schema, repeated_elements, BuilderOptions, SchemaBuilderTrait},
    discover::{discover_structs_for_module, MoveStructWithModuleId},
    parse::parse_structs,
};
use anyhow::{bail, Context as AnyhowContext, Result};
use aptos_api_types::MoveModule;
use async_graphql::dynamic::Schema;
use move_core_types::language_storage::ModuleId;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// This struct provides a schema builder that works only with the packages that you
/// give it. The obvious downside of this approach is if the schema builder finds a
/// reference to a module that it doesn't have, it can't go fetch it. This is good
/// for use in things like the CLI, where there is already a framework for fetching
/// dependencies and compiling packages.
#[derive(Clone, Debug)]
pub struct SchemaBuilderLocal {
    modules: BTreeMap<ModuleId, MoveModule>,
    options: BuilderOptions,
}

impl SchemaBuilderLocal {
    pub fn new(options: BuilderOptions) -> Self {
        Self {
            modules: BTreeMap::new(),
            options,
        }
    }

    /// Find all the structs we need to include in the schema.
    fn discover_structs(&self) -> Result<Vec<MoveStructWithModuleId>> {
        if self.modules.is_empty() {
            anyhow::bail!("Cannot build Schema without any modules to lookup or add");
        }

        let mut structs: Vec<MoveStructWithModuleId> = Vec::new();
        let mut modules_processed = BTreeSet::new();

        let mut modules = self.modules.clone();

        while let Some((module_id, module)) = modules.pop_first() {
            // Because a module can be depended on multiple times, we keep record of
            // which modules we have already processed so we don't process them again.
            if modules_processed.contains(&module_id) {
                continue;
            }
            modules_processed.insert(module_id.clone());

            let (new_structs, modules_to_retrieve) = discover_structs_for_module(module)
                .with_context(|| format!("Failed to parse module {}", module_id))?;

            structs.extend(new_structs);

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
                    build a complete Schema.",
                    module_id,
                    modules_to_retrieve
                );
            }
        }

        Ok(structs)
    }

    pub fn build(&self) -> Result<Schema> {
        let structs = self
            .discover_structs()
            .context("Discovery of module structs failed")?;

        // Build a list of structs from different modules that have the same name. We
        // will need to resolve the collision for these by using the fully qualified
        // name (address, module, struct name) rather than just the struct name.
        let repeated_struct_names =
            repeated_elements(structs.iter().map(|s| s.struc.name.to_string()).collect());

        // Build GraphQL objects from the structs.
        let objects = parse_structs(structs, &repeated_struct_names, &self.options)
            .context("Failed to build GraphQL objects from structs")?;

        let schema = build_schema(objects).context("Failed to build Schema from objects")?;

        Ok(schema)
    }
}

impl SchemaBuilderTrait for SchemaBuilderLocal {
    fn add_modules(mut self, modules: Vec<MoveModule>) -> Self {
        for module in modules {
            self = self.add_module(module)
        }
        self
    }

    fn add_module(mut self, module: MoveModule) -> Self {
        self.modules.insert(
            ModuleId::new(module.address.into(), module.name.clone().into()),
            module,
        );
        self
    }
}

impl Default for SchemaBuilderLocal {
    fn default() -> Self {
        Self::new(BuilderOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_move_graphql_test_helpers::compile_package;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_build_schema() -> Result<()> {
        // Compile the hero package and all the packages we know it recursively depends on.
        let mut modules = Vec::new();
        for name in &[
            "aptos-stdlib",
            "move-stdlib",
            "aptos-framework",
            "aptos-token-objects",
        ] {
            let path =
                PathBuf::try_from(format!("../../../aptos-move/framework/{}", name)).unwrap();
            modules.extend(compile_package(path)?);
        }
        modules.extend(compile_package(
            PathBuf::try_from("../../../aptos-move/move-examples/token_objects/hero").unwrap(),
        )?);

        let options = BuilderOptions {
            use_special_handling_for_option: true,
            always_use_fully_qualifed_names: false,
        };
        let schema = SchemaBuilderLocal::new(options)
            .add_modules(modules)
            .build()
            .context("Failed to build Schema with all the modules")?;

        let schema_str = schema.sdl();

        // Assert that the structs with names that are used in other Move modules are
        // given fully qualified names in the schema.
        assert!(schema_str.contains("type _0x0000000000000000000000000000000000000000000000000000000000000001__pool_u64__Pool"));
        assert!(schema_str.contains("type _0x0000000000000000000000000000000000000000000000000000000000000001__pool_u64_unbound__Pool"));

        // Assert that the structs with names that are not used in other Move modules
        // are not given fully qualified names in the schema.
        assert!(schema_str.contains("type Hero"));

        Ok(())
    }
}
