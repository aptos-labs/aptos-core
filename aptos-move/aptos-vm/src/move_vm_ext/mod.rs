// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MoveVM and Session wrapped, to make sure Aptos natives and extensions are always installed and
//! taken care of after session finish.
mod resolver;
mod respawned_session;
mod session;
mod vm;
mod warm_vm_cache;
pub(crate) mod write_op_converter;

pub use crate::move_vm_ext::{
    resolver::{AptosMoveResolver, AsExecutorView, AsResourceGroupView, ResourceGroupResolver},
    respawned_session::RespawnedSession,
    session::{SessionExt, SessionId},
    vm::{get_max_binary_format_version, get_max_identifier_size, verifier_config, MoveVmExt},
};
