// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod stackless_exec_ir;

pub mod destack;
pub mod lower;
pub mod pipeline;

pub use destack::destack;
pub use lower::{LoweredFunction, LoweredModule};
pub use pipeline::destack_and_lower_module;
