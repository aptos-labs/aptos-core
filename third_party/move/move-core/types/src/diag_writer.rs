// Copyright (c) Aptos Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared writer infrastructure for threading `WriteColor` through Move tools.
//!
//! [`DiagWriter`] wraps an `Arc<Mutex<dyn WriteColor + Send>>` so it can be
//! cheaply cloned and shared between the tools pipeline and callers (e.g.
//! test harnesses) that want to capture tool output.

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use termcolor::{Buffer, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// A clonable writer that delegates to an `Arc<Mutex<dyn WriteColor + Send>>`.
///
/// Implements both [`Write`] and [`WriteColor`] so it can be used everywhere
/// the build pipeline needs a writer.
#[derive(Clone)]
pub struct DiagWriter(pub Arc<Mutex<dyn WriteColor + Send>>);

impl DiagWriter {
    /// Create a production writer backed by `StandardStream::stderr`.
    pub fn stderr() -> Self {
        Self(Arc::new(Mutex::new(StandardStream::stderr(
            ColorChoice::Auto,
        ))))
    }

    /// Create a writer backed by an in-memory [`Buffer`] with colors stripped.
    ///
    /// Returns both the writer and a handle to the buffer. After execution,
    /// read captured output via `buffer.lock().unwrap().as_slice()`.
    pub fn new_buffer() -> (Self, Arc<Mutex<Buffer>>) {
        let buffer = Arc::new(Mutex::new(Buffer::no_color()));
        let writer = Self(buffer.clone() as Arc<Mutex<dyn WriteColor + Send>>);
        (writer, buffer)
    }
}

impl Write for DiagWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?
            .flush()
    }
}

impl WriteColor for DiagWriter {
    fn supports_color(&self) -> bool {
        self.0.lock().map(|w| w.supports_color()).unwrap_or(false)
    }

    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?
            .set_color(spec)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?
            .reset()
    }
}
