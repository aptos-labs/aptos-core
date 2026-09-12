// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static preparation of fuzz targets.
//!
//! Analyzes compiled packages into datatype and function registries, builds
//! flow graphs showing how each parameter can be constructed, and emits the
//! Move driver scripts that the fuzzer executes.

pub mod ident;

pub mod datatype;
pub mod function;

pub mod typing;

pub mod canvas;
pub mod graph;
pub mod model;
