// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{SpeculativeEvent, SpeculativeEvents};
use claims::{assert_err, assert_ok};
use std::{
    collections::HashSet,
    sync::mpsc::{sync_channel, SyncSender},
};

// Test speculative logging use-case and provide an example implementation.

// Fake local logging levels for testing.
enum Level {
    Error = 0,
    Warn,
    Debug,
    Trace,
}

// Struct to capture speculative logging event.
struct SpeculativeLog {
    level: Level,
    message: String,
    sender: SyncSender<String>,
}

impl SpeculativeLog {
    fn new(level: Level, message: String, sender: SyncSender<String>) -> Self {
        Self {
            level,
            message,
            sender,
        }
    }
}

// Implementing the required SpeculativeEvents trait. In the real use-case, match
// would dispatch to real logging macros, but here we send on a channel to test.
impl SpeculativeEvent for SpeculativeLog {
    fn dispatch(self) {
        match self.level {
            Level::Debug => {
                self.sender.send(format!("Debug {}", self.message)).unwrap();
            },
            Level::Warn => {
                self.sender.send(format!("Warn {}", self.message)).unwrap();
            },
            Level::Error => {
                self.sender.send(format!("Error {}", self.message)).unwrap();
            },
            _ => (),
        }
    }
}

#[test]
fn test_speculative_logging() {
    let (sender, receiver) = sync_channel(10);

    let speculative_logs = SpeculativeEvents::<SpeculativeLog>::new(2);

    assert_ok!(speculative_logs.record(
        1,
        SpeculativeLog::new(Level::Warn, "1/warn: A".to_string(), sender.clone())
    )); // Expected
    assert_ok!(speculative_logs.record(
        0,
        SpeculativeLog::new(Level::Trace, "0/trace: B".to_string(), sender.clone()),
    )); // not enabled
    assert_ok!(speculative_logs.record(
        0,
        SpeculativeLog::new(Level::Trace, "1/trace: A".to_string(), sender.clone()),
    )); // not enabled
    assert_ok!(speculative_logs.record(
        1,
        SpeculativeLog::new(Level::Error, "1/error: C".to_string(), sender.clone()),
    )); // Expected
    assert_ok!(speculative_logs.record(
        0,
        SpeculativeLog::new(Level::Debug, "0/debug: B".to_string(), sender),
    )); // Expected

    assert_err!(receiver.try_recv());
    speculative_logs.flush(2);

    // We expect 3 messages.
    let expected = vec![
        "Warn 1/warn: A".to_string(),
        "Error 1/error: C".to_string(),
        "Debug 0/debug: B".to_string(),
    ];
    let mut expected_set: HashSet<String> = expected.into_iter().collect();

    for _ in 0..3 {
        let m = receiver.recv();
        assert!(expected_set.remove(&m.expect("expected a message")));
    }
    assert_err!(receiver.try_recv());
}
