// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Generate the updates to be applied to state tree in batches
// the updates should be sorted lexicographically
// read should be done after writing the key

use crate::pipeline::{Action, ActionConfig};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use bytes::Bytes;
use rand::Rng;
use std::{
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

pub struct ActionGenerator {
    receiver: Receiver<ActionConfig>,
    execution_sender: SyncSender<Vec<Action>>,
    executor_handle: Option<thread::JoinHandle<()>>,
}

impl ActionGenerator {
    pub fn new(
        receiver: Receiver<ActionConfig>,
        execution_sender: SyncSender<Vec<Action>>,
    ) -> Self {
        Self {
            receiver,
            execution_sender,
            executor_handle: None,
        }
    }

    pub fn set_executor_handle(&mut self, handle: thread::JoinHandle<()>) {
        self.executor_handle = Some(handle);
    }

    pub fn run(&mut self) {
        loop {
            let config = self
                .receiver
                .recv()
                .expect("Failure in receiving action config");

            let mut actions = Vec::new();
            if config.count == 0 {
                // notify to stop
                self.execution_sender.send(actions).unwrap();
                break;
            }
            let mut rng = rand::thread_rng();
            for state_key in
                (config.last_state_key_ind + 1)..=(config.last_state_key_ind + config.count)
            {
                let number = rng.gen_range(0, 1000000u32);
                if number < config.delete_ratio {
                    // we want to generate a delete here, we can only generate a delete for existing key
                    let state_key_ind = rng.gen_range(0, state_key as u32);
                    actions.push(Action::Write(
                        self.generate_state_key(state_key_ind as usize),
                        None,
                    ));
                } else {
                    actions.push(Action::Write(
                        self.generate_state_key(state_key),
                        Some(self.generate_state_value(state_key)),
                    ));
                }
            }
            self.execution_sender.send(actions).unwrap();
        }

        if let Some(handle) = self.executor_handle.take() {
            handle.join().unwrap();
        }
    }

    fn generate_state_key(&self, state_key_ind: usize) -> StateKey {
        StateKey::raw(&state_key_ind.to_be_bytes())
    }

    fn generate_state_value(&self, state_key_ind: usize) -> StateValue {
        StateValue::new_legacy(Bytes::copy_from_slice(&state_key_ind.to_be_bytes()))
    }
}
