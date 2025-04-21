// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use anyhow::anyhow;
use crate::{
    prometheus_metrics::LatencyBreakdown,
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    CoreContext, Result, Swarm, TestReport,
};
use aptos_transaction_emitter_lib::{EmitJobRequest, TxnStats};
use async_trait::async_trait;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::runtime::{Handle, Runtime};

/// The testing interface which defines a test written with full control over an existing network.
/// Tests written against this interface will have access to both the Root account as well as the
/// nodes which comprise the network.
#[async_trait]
pub trait NetworkTest: Test {
    /// Executes the test against the given context.
    async fn run<'t>(&self, ctx: NetworkContextSynchronizer<'t>) -> Result<()>;

    async fn run_with_timeout<'t>(&self, ctx: NetworkContextSynchronizer<'t>, timeout: Duration) -> Result<()> {
        let timeout = tokio::time::timeout(timeout, self.run(ctx));
        timeout.await.map_err(|_| anyhow!("Test timed out"))?
    }
}

#[derive(Clone)]
pub struct NetworkContextSynchronizer<'t> {
    pub ctx: Arc<tokio::sync::Mutex<NetworkContext<'t>>>,
    pub handle: tokio::runtime::Handle,
}

// TODO: some useful things that don't need to hold the lock or make a copy
impl<'t> NetworkContextSynchronizer<'t> {
    pub fn new(ctx: NetworkContext<'t>, handle: tokio::runtime::Handle) -> Self {
        Self {
            ctx: Arc::new(tokio::sync::Mutex::new(ctx)),
            handle,
        }
    }

    pub async fn report_text(&self, text: String) {
        let mut locker = self.ctx.lock().await;
        locker.report.report_text(text);
    }

    pub fn flex_block_on<F: Future>(&self, future: F) -> F::Output {
        match Handle::try_current() {
            Ok(handle) => {
                // we are in an async context, we don't need block_on
                handle.block_on(future)
            },
            Err(_) => self.handle.block_on(future),
        }
    }
}

pub struct NetworkContext<'t> {
    core: CoreContext,
    pub swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    pub report: &'t mut TestReport,
    pub global_duration: Duration,
    pub emit_job: EmitJobRequest,
    pub success_criteria: SuccessCriteria,
    pub runtime: Runtime,
}

impl<'t> NetworkContext<'t> {
    pub fn new(
        core: CoreContext,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &'t mut TestReport,
        global_duration: Duration,
        emit_job: EmitJobRequest,
        success_criteria: SuccessCriteria,
    ) -> Self {
        Self {
            core,
            swarm,
            report,
            global_duration,
            emit_job,
            success_criteria,
            runtime: aptos_runtimes::spawn_named_runtime("emitter".into(), Some(64)),
        }
    }

    pub fn core(&mut self) -> &mut CoreContext {
        &mut self.core
    }

    pub async fn check_for_success(
        &mut self,
        stats: &TxnStats,
        window: Duration,
        latency_breakdown: &LatencyBreakdown,
        start_time: i64,
        end_time: i64,
        start_version: u64,
        end_version: u64,
    ) -> Result<()> {
        SuccessCriteriaChecker::check_for_success(
            &self.success_criteria,
            self.swarm.clone(),
            self.report,
            stats,
            window,
            latency_breakdown,
            start_time,
            end_time,
            start_version,
            end_version,
        )
        .await
    }

    pub fn handle(&self) -> Handle {
        match Handle::try_current() {
            Ok(handle) => {
                // we are in an async context, we don't need block_on
                handle
            },
            Err(_) => self.runtime.handle().clone(),
        }
    }

    pub fn flex_block_on<F: Future>(&self, future: F) -> F::Output {
        match Handle::try_current() {
            Ok(handle) => {
                // we are in an async context, we don't need block_on
                handle.block_on(future)
            },
            Err(_) => self.runtime.block_on(future),
        }
    }
}
