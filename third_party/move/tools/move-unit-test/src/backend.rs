// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The interface between the test runner and the VMs that execute tests. The
//! runner filters tests, schedules execution, checks expectations, and reports
//! results; backends execute tests and collect their results and effects.

use anyhow::Result;
use bytes::Bytes;
use legacy_move_compiler::unit_test::{TestCase, TestPlan};
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    language_storage::{ModuleId, TypeTag},
};
use std::{collections::BTreeMap, time::Duration};

/// Executes unit tests. Shared across the runner's threads.
pub trait UnitTestBackend: Send + Sync {
    /// Names the backend in reports.
    fn name(&self) -> &str;

    fn execute(&self, module_id: &ModuleId, test: &TestCase) -> TestExecution;
}

pub trait UnitTestBackendFactory {
    fn build<'a>(
        &'a self,
        plan: &TestPlan,
        genesis: &ChangeSet,
        config: &BackendConfig,
    ) -> Result<Box<dyn UnitTestBackend + 'a>>;
}

/// The backends a run executes tests on.
#[derive(Clone, Copy)]
pub enum Backends<'a> {
    Single(&'a dyn UnitTestBackendFactory),
    /// Runs every test on both backends. The runner checks `reference` against
    /// the test's expectation and fails the test if `candidate`'s outcome differs.
    Differential {
        reference: &'a dyn UnitTestBackendFactory,
        candidate: &'a dyn UnitTestBackendFactory,
    },
}

#[derive(Clone, Debug)]
pub struct BackendConfig {
    /// Number of runner threads that may call `execute` concurrently.
    pub num_threads: usize,
    /// Fill `TestRun::effects`.
    pub capture_effects: bool,
    /// Fill `TestRun::storage_dump` for a run that may fail its test.
    pub render_storage_dump: bool,
    /// Attach the caller stack trace to failures.
    pub report_stacktrace_on_abort: bool,
}

#[derive(Debug)]
pub enum TestExecution {
    Ran(TestRun),
    /// A typed backend error identifies an unsupported feature needed by this test.
    Unsupported {
        reason: String,
    },
    /// The backend itself failed. Fails the test regardless of its expectation.
    BackendError {
        message: String,
    },
}

#[derive(Debug)]
pub struct TestRun {
    /// BCS return values, or the failure as a V1-shaped `VMError`.
    pub result: VMResult<Vec<Vec<u8>>>,
    /// Whether a failed `result` can be checked against an expected failure or
    /// another backend's failure: false when the backend could only approximate
    /// V1's status, sub-status or location. Ignored on success.
    pub v1_exact_error: bool,
    /// `None` unless captured, or if the backend could not record them.
    pub effects: Option<TestEffects>,
    pub storage_dump: Option<String>,
    pub gas_used: u64,
    pub elapsed: Duration,
}

/// VM-neutral effects of one run, as the backend reports them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestEffects {
    pub resources: ChangeSet,
    /// Table handle -> serialized key -> operation.
    pub table_items: BTreeMap<AccountAddress, BTreeMap<Vec<u8>, Op<Bytes>>>,
    /// In emission order.
    pub events: Vec<TestEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestEvent {
    /// GUID and sequence number of a handle event; `None` for a module event.
    pub handle: Option<(Vec<u8>, u64)>,
    pub type_tag: TypeTag,
    pub data: Vec<u8>,
}

impl TestEffects {
    pub fn new(resources: ChangeSet) -> Self {
        Self {
            resources,
            table_items: BTreeMap::new(),
            events: vec![],
        }
    }
}
