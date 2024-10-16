// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_executor_types::{ExecutorResult, StateComputeResult};
use aptos_types::transaction::SignedTransaction;
use derivative::Derivative;
use futures::future::{BoxFuture, Shared};
use std::{future::Future, pin::Pin, time::Duration};

use crate::pipelined_block::SyncPreCommitResultFut;

pub type SyncBoxFuture<'a, T> = Shared<Pin<Box<dyn Future<Output = T> + Send + 'a>>>;
#[derive(Derivative)]
#[derivative(Clone, Debug)]
pub struct PipelineExecutionResult {
    pub input_txns: Vec<SignedTransaction>,
    pub result: StateComputeResult,
    pub execution_time: Duration,
    #[derivative(Debug = "ignore")]
    pub pre_commit_fut: Option<SyncPreCommitResultFut>,
}

impl PipelineExecutionResult {
    pub fn new(
        input_txns: Vec<SignedTransaction>,
        result: StateComputeResult,
        execution_time: Duration,
        pre_commit_fut: Option<SyncPreCommitResultFut>,
    ) -> Self {
        Self {
            input_txns,
            result,
            execution_time,
            pre_commit_fut,
        }
    }

    pub fn set_pre_commit_fut(&mut self, pre_commit_fut: SyncPreCommitResultFut) {
        self.pre_commit_fut = Some(pre_commit_fut);
    }
}
