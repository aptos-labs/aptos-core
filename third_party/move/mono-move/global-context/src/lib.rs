// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use mono_move_alloc::GlobalArenaPool;
mod context;
pub use context::{
    ArenaRef, Executable, ExecutableBuilder, ExecutableId, ExecutionGuard, FieldLayout, Function,
    GlobalContext, MaintenanceGuard, Type,
};
pub use move_core_types::identifier::Identifier;
pub mod maintenance_config;
