// Copyright © Aptos Foundation

use aptos_rest_client::error::RestError;

#[derive(Debug)]
pub struct TestLog {
    pub result: TestResult,
    pub time: f64,
}

#[derive(Debug)]
pub enum TestResult {
    Success,
    Fail(TestFailure),
}

#[derive(Debug)]
pub enum TestFailure {
    Fail(&'static str),
    Error(anyhow::Error),
}

impl From<RestError> for TestFailure {
    fn from(e: RestError) -> TestFailure {
        TestFailure::Error(e.into())
    }
}

impl From<anyhow::Error> for TestFailure {
    fn from(e: anyhow::Error) -> TestFailure {
        TestFailure::Error(e)
    }
}
