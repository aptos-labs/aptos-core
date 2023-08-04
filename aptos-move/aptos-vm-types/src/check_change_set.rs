// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use move_core_types::vm_status::VMStatus;

/// Trait to check the contents of a change set, e.g. the total number of
/// bytes per write op or event.
pub trait CheckChangeSet {
    fn check_change_set(&self, change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus>;
}
