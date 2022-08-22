// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use std::io::{Error, ErrorKind};

#[derive(Debug)]
pub(crate) struct TelemetryLogWriter {
    tx: mpsc::Sender<String>,
}

impl TelemetryLogWriter {
    pub fn new(tx: mpsc::Sender<String>) -> Self {
        Self { tx }
    }
}

impl TelemetryLogWriter {
    pub fn write(&mut self, log: String) -> std::io::Result<usize> {
        let len = log.len();
        match self.tx.try_send(log) {
            Ok(_) => Ok(len),
            Err(err) => {
                if err.is_full() {
                    Err(Error::new(ErrorKind::WouldBlock, "Channel full"))
                } else {
                    Err(Error::new(ErrorKind::ConnectionRefused, "Disconnected"))
                }
            }
        }
    }

    #[allow(dead_code)]
    // TODO: hook up flush when it is implemented in LoggerService
    pub fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
