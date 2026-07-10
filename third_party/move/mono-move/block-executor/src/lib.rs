// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove as a Block-STM execution task.
//!
//! Implements the block executor's VM-facing traits ([`Record`],
//! `TransactionExecutor`) for the mono VM: transactions execute on the mono
//! interpreter against a Block-STM-backed resource provider, and publish
//! their effects as heap pointers into the shared multi-version map.
//!
//! [`Record`]: aptos_block_executor::record::Record

mod entry_call;
mod events;
mod executor_task;
mod module_provider;
mod natives;
mod record;
mod resource_provider;
mod value;

pub use executor_task::MonoExecutorTask;
pub use module_provider::StateViewModuleProvider;
pub use natives::build_production_natives;
pub use record::MonoRecord;
pub use resource_provider::{BlockStmResourceProvider, ReadMode};
pub use value::{to_mono_version, MonoValue};
