// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::{APTOS_LOG_INGEST_WRITER_DISCONNECTED, APTOS_LOG_INGEST_WRITER_FULL};
use futures::channel;
use std::{
    io::{Error, ErrorKind},
    sync,
};

#[derive(Debug)]
pub enum TelemetryLog {
    Log(String),
    Flush(sync::mpsc::SyncSender<()>),
}

#[derive(Debug)]
pub(crate) struct TelemetryLogWriter {
    tx: channel::mpsc::Sender<TelemetryLog>,
}

impl TelemetryLogWriter {
    pub fn new(tx: channel::mpsc::Sender<TelemetryLog>) -> Self {
        Self { tx }
    }
}

impl TelemetryLogWriter {
    pub fn write(&mut self, log: String) -> std::io::Result<usize> {
        let len = log.len();
        match self.tx.try_send(TelemetryLog::Log(log)) {
            Ok(_) => Ok(len),
            Err(err) => {
                if err.is_full() {
                    APTOS_LOG_INGEST_WRITER_FULL.inc_by(len as u64);
                    Err(Error::new(ErrorKind::WouldBlock, "Channel full"))
                } else {
                    APTOS_LOG_INGEST_WRITER_DISCONNECTED.inc_by(len as u64);
                    Err(Error::new(ErrorKind::ConnectionRefused, "Disconnected"))
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn flush(&mut self) -> std::io::Result<sync::mpsc::Receiver<()>> {
        let (tx, rx) = sync::mpsc::sync_channel(1);
        match self.tx.try_send(TelemetryLog::Flush(tx)) {
            Ok(_) => Ok(rx),
            Err(err) => {
                if err.is_full() {
                    Err(Error::new(ErrorKind::WouldBlock, "Channel full"))
                } else {
                    Err(Error::new(ErrorKind::ConnectionRefused, "Disconnected"))
                }
            },
        }
    }
}
