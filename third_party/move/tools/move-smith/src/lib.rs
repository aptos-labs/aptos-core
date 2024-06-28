// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MoveSmith is a fuzzer at the Move language level.

pub mod ast;
pub mod codegen;
pub mod config;
pub mod env;
pub mod move_smith;
pub mod names;
pub mod types;
pub mod utils;

pub use codegen::CodeGenerator;
pub use move_smith::MoveSmith;
