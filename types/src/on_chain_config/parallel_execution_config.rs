// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use anyhow::{format_err, Result};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use move_read_write_set_types::ReadWriteSet;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Defines the operation status of parallel execution. If this `read_write_analysis_result` is not
/// None VM will execute transactions in parallel.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParallelExecutionConfig {
    pub read_write_analysis_result: Option<ReadWriteSetAnalysis>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct ParallelExecutionConfigInner {
    pub read_write_analysis_result: Option<Vec<u8>>,
}

impl ParallelExecutionConfigInner {
    fn as_analysis_result(&self) -> Result<ParallelExecutionConfig> {
        Ok(ParallelExecutionConfig {
            read_write_analysis_result: match &self.read_write_analysis_result {
                Some(bytes) => Some(bcs::from_bytes(bytes)?),
                None => None,
            },
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ReadWriteSetAnalysis {
    V1(BTreeMap<ModuleId, BTreeMap<Identifier, ReadWriteSet>>),
}

impl ReadWriteSetAnalysis {
    pub fn into_inner(self) -> BTreeMap<ModuleId, BTreeMap<Identifier, ReadWriteSet>> {
        match self {
            Self::V1(inner) => inner,
        }
    }
}

impl OnChainConfig for ParallelExecutionConfig {
    const IDENTIFIER: &'static str = "ParallelExecutionConfig";

    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_config = bcs::from_bytes::<ParallelExecutionConfigInner>(bytes).map_err(|e| {
            format_err!(
                "Failed first round of deserialization for VMConfigInner: {}",
                e
            )
        })?;
        raw_config.as_analysis_result()
    }
}
