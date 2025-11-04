// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Writer;
use once_cell::sync::OnceCell;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};

static GLOBAL_WRITER: OnceCell<Arc<(Mutex<NonBlocking>, WorkerGuard)>> = OnceCell::new();

#[derive(Clone)]
pub struct TracingWriter {
    writer_guard: Arc<(Mutex<NonBlocking>, WorkerGuard)>,
}

impl TracingWriter {
    pub fn new(log_file: PathBuf, rotation: Rotation) -> Self {
        let writer_guard = GLOBAL_WRITER.get_or_init(|| {
            // TODO: Use max_log_size and max_rotated_logs to construct a size based rotation.
            let file_appender = RollingFileAppender::new(rotation, "", log_file);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            Arc::new((Mutex::new(non_blocking), guard))
        });

        Self {
            writer_guard: writer_guard.clone(),
        }
    }
}

impl Writer for TracingWriter {
    /// Write to file
    fn write(&self, log: String) {
        let (writer_mutex, _guard) = &*self.writer_guard;
        if let Ok(mut writer) = writer_mutex.lock() {
            if let Err(err) = writer.write_all(log.as_bytes()) {
                eprintln!("Unable to write to log file: {}", err);
            }
        }
    }

    fn write_buferred(&mut self, log: String) {
        self.write(log);
    }
}
