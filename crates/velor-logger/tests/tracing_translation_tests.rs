// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_infallible::RwLock;
use velor_logger::{velor_logger::VelorData, Writer};
use std::sync::Arc;
use tracing::Level;

#[derive(Default)]
struct VecWriter {
    logs: Arc<RwLock<Vec<String>>>,
    write_to_stderr: bool,
}

impl VecWriter {
    fn write_to_stderr(self, write_to_stderr: bool) -> Self {
        Self {
            write_to_stderr,
            ..self
        }
    }
}

impl Writer for VecWriter {
    fn write(&self, log: String) {
        if self.write_to_stderr {
            eprintln!("{}", log);
        }
        self.logs.write().push(log);
    }

    fn write_buferred(&mut self, log: String) {
        self.write(log);
    }
}

#[test]
fn verify_tracing_kvs() {
    // set up the logger
    let writer = VecWriter::default();
    let logs = writer.logs.clone();
    VelorData::builder()
        .is_async(false)
        .tokio_console_port(None)
        .printer(Box::new(writer.write_to_stderr(false)))
        .build();

    assert_eq!(logs.read().len(), 0);

    // log some messages
    let span = tracing::span!(Level::ERROR, "outer", one = "hello", two = "two");
    let _entered_one = span.enter();
    let span2 = tracing::span!(Level::ERROR, "inner", one = "hello", two = "two");
    let _entered_two = span2.enter();

    tracing::error!(another_value = "hello", "hello world");
    let s = logs.write().pop().unwrap();
    assert!(s.contains("ERROR"));
    assert!(s.contains("hello world"));
    // have the top-level span...
    assert!(s.contains("outer"));
    // ...and the nested spans
    assert!(s.contains("outer.inner"));

    tracing::info!("foo {} bar", 42);
    let s = logs.write().pop().unwrap();
    assert!(s.contains("INFO"));
    assert!(s.contains("foo 42 bar"));

    tracing::warn!(a = true, b = false);
    let s = logs.write().pop().unwrap();
    assert!(s.contains("WARN"));
    assert!(s.contains("true"));
    assert!(s.contains("false"));
}
