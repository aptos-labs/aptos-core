// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod alloc;
pub use alloc::GlobalArenaPool;
mod context;
pub use context::{ExecutableIdRef, ExecutionGuard, GlobalContext, MaintenanceGuard, NameRef};
pub mod maintenance_config;
