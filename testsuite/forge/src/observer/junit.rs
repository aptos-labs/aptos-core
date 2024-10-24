// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::result::{TestObserver, TestResult};
use anyhow::Result;
use quick_junit::{NonSuccessKind, Report, TestCase, TestSuite};
use std::sync::Mutex;
use uuid::Uuid;

pub struct JunitTestObserver {
    name: String,
    path: String,
    results: Mutex<Vec<(String, TestResult)>>,
}

impl JunitTestObserver {
    pub fn new(name: String, path: String) -> Self {
        Self {
            name,
            path,
            results: Mutex::new(vec![]),
        }
    }
}

impl TestObserver for JunitTestObserver {
    fn name(&self) -> String {
        "junit".to_string()
    }

    fn handle_result(&self, test_name: String, result: &TestResult) -> Result<()> {
        println!("JUNIT: {}: {:?}", test_name, result);
        self.results
            .lock()
            .unwrap()
            .push((test_name, result.clone()));
        Ok(())
    }

    fn finish(&self) -> Result<()> {
        let mut report = Report::new("forge");
        let uuid = Uuid::new_v4();
        report.set_uuid(uuid);

        let mut suite = TestSuite::new(self.name.clone());
        for (test_name, result) in self.results.lock().unwrap().iter() {
            let status = match result {
                TestResult::Ok => quick_junit::TestCaseStatus::success(),
                TestResult::FailedWithMsg(msg) => {
                    // Not 100% sure what the difference between failure and error is.
                    let mut status =
                        quick_junit::TestCaseStatus::non_success(NonSuccessKind::Failure);
                    status.set_message(msg.clone());
                    status
                },
            };

            let test_case = TestCase::new(test_name.clone(), status);
            suite.add_test_case(test_case);
        }

        report.add_test_suite(suite);

        println!("Writing JUnit report to {}", self.path);

        let writer = std::fs::File::create(self.path.clone())?;
        report.serialize(writer)?;
        Ok(())
    }
}
