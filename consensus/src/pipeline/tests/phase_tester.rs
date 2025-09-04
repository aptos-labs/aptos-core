// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::{
        buffer_manager::{Receiver, Sender},
        pipeline_phase::{CountedRequest, StatelessPipeline},
    },
    test_utils::{consensus_runtime, timed_block_on},
};
use futures::{SinkExt, StreamExt};
use std::sync::{atomic::AtomicU64, Arc};

pub struct PhaseTestCase<T: StatelessPipeline> {
    index: usize,
    input: T::Request,
    judge: Box<dyn Fn(T::Response)>,
    prompt: Option<String>,
}

pub struct PhaseTester<T: StatelessPipeline> {
    pub cases: Vec<PhaseTestCase<T>>,
}

impl<T: StatelessPipeline> PhaseTester<T> {
    pub fn new() -> Self {
        Self { cases: vec![] }
    }

    pub fn add_test_case(&mut self, input: T::Request, judge: Box<dyn Fn(T::Response)>) {
        self.add_test_case_with_prompt(input, judge, None)
    }

    pub fn add_test_case_with_prompt(
        &mut self,
        input: T::Request,
        judge: Box<dyn Fn(T::Response)>,
        prompt: Option<String>,
    ) {
        self.cases.push(PhaseTestCase {
            index: self.cases.len(),
            input,
            judge,
            prompt,
        });
    }

    // unit tests are for phase processors only,
    // this function consumes the tester
    pub fn unit_test(self, processor: &T) {
        let runtime = consensus_runtime();

        timed_block_on(&runtime, async move {
            for PhaseTestCase {
                index,
                input,
                judge,
                prompt,
            } in self.cases
            {
                eprint!(
                    "Unit Test - {}:",
                    prompt.unwrap_or(format!("Test {}", index))
                );
                let resp = processor.process(input).await;
                judge(resp);
                eprintln!(" OK",);
            }
        })
    }

    // e2e tests are for the pipeline phases
    // this function consumes the tester
    pub fn e2e_test(
        self,
        mut tx: Sender<CountedRequest<T::Request>>,
        mut rx: Receiver<T::Response>,
    ) {
        let runtime = consensus_runtime();

        timed_block_on(&runtime, async move {
            for PhaseTestCase {
                index,
                input,
                judge,
                prompt,
            } in self.cases
            {
                eprint!(
                    "E2E Test - {}:",
                    prompt.unwrap_or(format!("Test {}", index))
                );
                tx.send(CountedRequest::new(input, Arc::new(AtomicU64::new(0))))
                    .await
                    .ok();
                let resp = rx.next().await.unwrap();
                judge(resp);
                eprintln!(" OK",);
            }
        })
    }
}
