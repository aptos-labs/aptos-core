// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api_types::MoveModule;
use aptos_move_graphql_scalars::ALL_CUSTOM_SCALARS_TYPE_NAMES;
use async_graphql::dynamic::{Object, Scalar, Schema};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Defines functions common to all `SchemaBuilder`s.
pub trait SchemaBuilderTrait {
    /// Add modules that we have already retrieved.
    fn add_modules(self, modules: Vec<MoveModule>) -> Self;

    /// Add a module that we have already retreived.
    fn add_module(self, module: MoveModule) -> Self;
}

/// These options influence how the schema builder and Move type parsers work.
#[derive(Debug, Clone, Deserialize, Parser, Serialize)]
pub struct BuilderOptions {
    /// If true, Options will be represented as nullable values of the inner type
    /// rather than an Option struct with a vector inside it.
    #[clap(long)]
    pub use_special_handling_for_option: bool,

    /// If true, rather than only use fully qualified object names when necessary,
    /// i.e. because there is a name collision, we will always use them.
    #[clap(long)]
    pub always_use_fully_qualifed_names: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for BuilderOptions {
    fn default() -> Self {
        Self {
            // False for now because our APIs / indexer stack doesn't represent Options
            // in a way that is compatible with this option.
            use_special_handling_for_option: false,
            // False by default because the fully qualified names are needlessly long
            // and gross in the vast majority of cases.
            always_use_fully_qualifed_names: false,
        }
    }
}

pub fn build_schema(objects: Vec<Object>) -> Result<Schema> {
    let mut builder = Schema::build(objects[0].type_name(), None, None);
    for object in objects.into_iter() {
        builder = builder.register(object);
    }

    // Add our custom scalars.
    for scalar in &*ALL_CUSTOM_SCALARS_TYPE_NAMES {
        builder = builder.register(Scalar::new(*scalar));
    }

    let schema = builder.finish()?;

    Ok(schema)
}

pub fn build_sdl(schema: &Schema) -> String {
    let sdl = schema.sdl();

    // Trim leading newlines.
    let sdl = sdl.trim_start_matches('\n');

    // Remove the schema section (everything starting with `schema {` and after}).
    // We only want the types.
    let sdl = sdl.split("schema {").next().unwrap();

    // Remove all trailing newlines.
    let sdl = sdl.trim_end_matches('\n');

    // Return the schema with one trailing newline.
    format!("{}\n", sdl)
}

/// Takes a list of elements that might be repeated and returns only the repeated items.
pub(crate) fn repeated_elements<T: std::hash::Hash + Eq>(vec: Vec<T>) -> HashSet<T> {
    let mut counts = HashMap::new();
    for item in vec {
        *counts.entry(item).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(item, _)| item)
        .collect()
}
