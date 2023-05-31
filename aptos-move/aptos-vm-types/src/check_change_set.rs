// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use move_core_types::vm_status::VMStatus;

/// Useful trait for checking the contents of a change set. For example, the
/// total number of bytes ber write op or event can be checked.
pub trait CheckChangeSet {
    /// Returns an error if the change set does not pass the check.
    fn check_change_set(&self, change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus>;
}
