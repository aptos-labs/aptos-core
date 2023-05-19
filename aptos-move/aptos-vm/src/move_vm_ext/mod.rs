// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

///! MoveVM and Session wrapped, to make sure Aptos natives and extensions are always installed and
///! taken care of after session finish.
mod resolver;
mod session;
mod vm;

pub use crate::move_vm_ext::{
    resolver::MoveResolverExt,
    session::{SessionExt, SessionId},
    vm::{get_max_binary_format_version, verifier_config, MoveVmExt},
};
