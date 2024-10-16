// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_executor_types::{ExecutorResult, StateComputeResult};
use aptos_types::transaction::SignedTransaction;
use derivative::Derivative;
use futures::future::BoxFuture;
use std::time::Duration;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PipelineExecutionResult {
    pub input_txns: Vec<SignedTransaction>,
    pub result: StateComputeResult,
    pub execution_time: Duration,
    #[derivative(Debug = "ignore")]
    pub pre_commit_fut: BoxFuture<'static, ExecutorResult<()>>,
}

impl PipelineExecutionResult {
    pub fn new(
        input_txns: Vec<SignedTransaction>,
        result: StateComputeResult,
        execution_time: Duration,
        pre_commit_fut: BoxFuture<'static, ExecutorResult<()>>,
    ) -> Self {
        Self {
            input_txns,
            result,
            execution_time,
            pre_commit_fut,
        }
    }
}
