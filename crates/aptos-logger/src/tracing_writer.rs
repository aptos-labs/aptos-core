// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Writer;
use once_cell::sync::OnceCell;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

struct SizeRollingFileAppender {
    max_log_file_size: u64,
    max_log_files: usize,
    log_file_path: PathBuf,
    current_log_file: File,
    current_log_size: u64,
}

impl SizeRollingFileAppender {
    fn new(log_file: PathBuf, max_log_file_size_mbs: u64, max_log_files: usize) -> Self {
        let max_log_file_size = max_log_file_size_mbs * 1024 * 1024;
        let (current_log_file, current_log_size) = Self::open_log_file(&log_file)
            .expect("Unable to open initial log file");

        Self {
            max_log_file_size,
            max_log_files,
            log_file_path: log_file,
            current_log_file,
            current_log_size,
        }
    }

    fn open_log_file(path: &Path) -> io::Result<(File, u64)> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok((file, size))
    }

    fn rotate(&mut self) {
        if let Err(e) = self.current_log_file.sync_all() {
            eprintln!("Failed to sync log file before rotation: {}", e);
        }

        for i in (1..self.max_log_files).rev() {
            let old_path = self.rotated_log_path(i - 1);
            if old_path.exists() {
                let new_path = self.rotated_log_path(i);
                if let Err(e) = fs::rename(&old_path, &new_path) {
                    eprintln!("Failed to rename log file {:?} -> {:?}: {}", old_path, new_path, e);
                }
            }
        }

        let new_path = self.rotated_log_path(0);
        if let Err(e) = fs::rename(&self.log_file_path, &new_path) {
            eprintln!("Failed to rename log file {:?} -> {:?}: {}", self.log_file_path, new_path, e);
        }

        match Self::open_log_file(&self.log_file_path) {
            Ok((file, size)) => {
                self.current_log_file = file;
                self.current_log_size = size;
            }
            Err(e) => {
                eprintln!("Failed to open new log file after rotation: {}", e);
            }
        }
    }

    fn rotated_log_path(&self, index: usize) -> PathBuf {
        let mut path = self.log_file_path.as_os_str().to_owned();
        path.push(format!(".{}", index));
        path.into()
    }
}

impl Write for SizeRollingFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.current_log_size + buf.len() as u64 > self.max_log_file_size {
            self.rotate();
        }
        let bytes_written = self.current_log_file.write(buf)?;
        self.current_log_size += bytes_written as u64;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.current_log_file.flush()
    }
}

static GLOBAL_WRITER: OnceCell<Arc<(Mutex<NonBlocking>, WorkerGuard)>> = OnceCell::new();

#[derive(Clone)]
pub struct TracingWriter {
    writer_guard: Arc<(Mutex<NonBlocking>, WorkerGuard)>,
}

impl TracingWriter {
    pub fn new(log_file: PathBuf, max_log_file_size_mbs: u64, max_log_files: usize) -> Self {
        let writer_guard = GLOBAL_WRITER.get_or_init(|| {
            let file_appender =
                SizeRollingFileAppender::new(log_file, max_log_file_size_mbs, max_log_files);
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
    fn write(&self, mut log: String) {
        log.push('\n');
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
