// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{access_path::AccessPath, on_chain_config::ConfigID, transaction::Version};
use anyhow::Result;

// TODO combine with ConfigStorage
pub trait MoveStorage {
    /// Returns a Move resources as a serialized byte array.
    fn fetch_resource(&self, access_path: AccessPath) -> Result<Vec<u8>>;

    /// Returns a Move resources as serialized byte array from a
    /// specified version of the database.
    fn fetch_resource_by_version(
        &self,
        access_path: AccessPath,
        version: Version,
    ) -> Result<Vec<u8>>;

    /// Returns an on-chain resource as a serialized byte array from a
    /// specified version of the database.
    fn fetch_config_by_version(&self, config_id: ConfigID, version: Version) -> Result<Vec<u8>>;

    /// Get the version on the latest transaction info.
    fn fetch_synced_version(&self) -> Result<Version>;

    /// Get the version of the latest state checkpoint
    fn fetch_latest_state_checkpoint_version(&self) -> Result<Version>;
}
