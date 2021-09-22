// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::PreprocessedTransaction, read_write_set_analysis::ReadWriteSetAnalysis,
};
use anyhow::Result;
use diem_parallel_executor::task::{Accesses, ReadWriteSetInferencer};
use diem_types::access_path::AccessPath;
use move_core_types::resolver::MoveResolver;
use read_write_set_dynamic::NormalizedReadWriteSetAnalysis;

pub(crate) struct ReadWriteSetAnalysisWrapper<'a, S: MoveResolver> {
    analyzer: ReadWriteSetAnalysis<'a, S>,
}

impl<'a, S: MoveResolver> ReadWriteSetAnalysisWrapper<'a, S> {
    pub fn new(analysis_result: &'a NormalizedReadWriteSetAnalysis, view: &'a S) -> Self {
        Self {
            analyzer: ReadWriteSetAnalysis::new(analysis_result, view),
        }
    }
}

impl<'a, S: MoveResolver + std::marker::Sync> ReadWriteSetInferencer
    for ReadWriteSetAnalysisWrapper<'a, S>
{
    type T = PreprocessedTransaction;
    fn infer_reads_writes(&self, txn: &Self::T) -> Result<Accesses<AccessPath>> {
        let (keys_read, keys_written) = self.analyzer.get_keys_transaction(txn, false)?;
        Ok(Accesses {
            keys_read: keys_read
                .into_iter()
                .map(AccessPath::resource_access_path)
                .collect(),
            keys_written: keys_written
                .into_iter()
                .map(AccessPath::resource_access_path)
                .collect(),
        })
    }
}
