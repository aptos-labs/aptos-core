// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate exposes SchemaBuilders, tools that can build GraphQL schemas from Move
//! modules. These schemas can then be used to power code generation to make working
//! with Move resources easier.

mod common;
mod discover;
mod local;
mod parse;

pub use common::{build_sdl, BuilderOptions, SchemaBuilderTrait};
pub use local::SchemaBuilderLocal;
