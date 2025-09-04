// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use std::{
    fs::{create_dir_all, File},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing_subscriber::fmt::MakeWriter;

/// This struct impls MakeWriter, a trait that returns writers for using by the
/// tracing_subscriber library. It returns a custom logger that logs to different
/// files based on the name of the worker (thread).
///
/// To be as efficient as possible and only create + open each file once, we keep
/// track of file handles in a DashMap with this FileLock struct as the values.
/// Learn more about FileLock in the doc comment there.
pub struct ThreadNameMakeWriter {
    /// The base directory to output logs to.
    base_dir: PathBuf,
    /// We keep open file handles here to avoid making them every time. They key is the
    /// name of the file (thread name without the number).
    file_handles: DashMap<String, FileLock>,
}

impl ThreadNameMakeWriter {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            file_handles: DashMap::new(),
        }
    }
}

impl<'a> MakeWriter<'a> for ThreadNameMakeWriter {
    type Writer = Box<FileLock>;

    fn make_writer(&'a self) -> Self::Writer {
        let base_dir = self.base_dir.clone();
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("no-thread-name")
            .to_string();
        let thread_name_no_number = truncate_last_segment(&thread_name, '-');
        let log_file = self
            .file_handles
            .entry(thread_name_no_number.clone())
            .or_insert_with(|| FileLock::new(create_file(base_dir, thread_name_no_number)))
            .value()
            .clone();
        Box::new(log_file)
    }
}

fn create_file(base_dir: PathBuf, thread_name_no_number: String) -> File {
    let dir_path = base_dir.join(thread_name_no_number);
    create_dir_all(&dir_path).expect("Failed to create log directory");
    let log_path = dir_path.join("tracing.log");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap()
}

fn truncate_last_segment(s: &str, delimiter: char) -> String {
    s.rsplit_once(delimiter)
        .map(|x| x.0)
        .unwrap_or(s)
        .to_string()
}

/// This struct protects access to an open file handle. Using a Mutex is necessary
/// because we cannot allow concurrent access to the file. The Arc ensures that
/// every time we hand out a reference to the handle, it is the same handle.
#[derive(Clone)]
pub struct FileLock {
    file: Arc<Mutex<File>>,
}

impl FileLock {
    pub fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl std::io::Write for FileLock {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.lock().unwrap().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.file.lock().unwrap().write_all(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.lock().unwrap().flush()
    }
}
