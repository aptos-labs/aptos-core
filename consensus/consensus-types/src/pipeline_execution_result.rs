// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_executor_types::{ExecutorResult, StateComputeResult};
use aptos_types::transaction::SignedTransaction;
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct PipelineExecutionResult {
    pub input_txns: Vec<SignedTransaction>,
    pub result: StateComputeResult,
    pub execution_time: Duration,
    pub pre_commit_result_rx: oneshot::Receiver<ExecutorResult<()>>,
}

impl PipelineExecutionResult {
    pub fn new(
        input_txns: Vec<SignedTransaction>,
        result: StateComputeResult,
        execution_time: Duration,
        pre_commit_result_rx: oneshot::Receiver<ExecutorResult<()>>,
    ) -> Self {
        Self {
            input_txns,
            result,
            execution_time,
            pre_commit_result_rx,
        }
    }
}
