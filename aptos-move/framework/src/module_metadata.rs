// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::AbortInfo;
use move_deps::move_core_types::errmap::ErrorDescription;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The key used to identify the metadata in the metadata section of the module bytecode.
/// This is more or less arbitrary, besides we should use some unique key to identify
/// Aptos specific metadata (`aptos::` here).
pub static APTOS_METADATA_KEY: Lazy<Vec<u8>> =
    Lazy::new(|| "aptos::metadata_v0".as_bytes().to_vec());

/// Aptos specific metadata attached to the metadata section of file_format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModuleMetadata {
    /// The error map containing the description of error reasons as grabed from the source.
    /// These are typically only a few entries so no relevant size difference.
    pub error_map: BTreeMap<u64, ErrorDescription>,
}

impl RuntimeModuleMetadata {
    pub fn extract_abort_info(&self, code: u64) -> Option<AbortInfo> {
        self.error_map
            .get(&(code & 0xfff))
            .or_else(|| self.error_map.get(&code))
            .map(|descr| AbortInfo {
                reason_name: descr.code_name.clone(),
                description: descr.code_description.clone(),
            })
    }
}
