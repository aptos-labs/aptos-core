// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    debug::DebugContext, interpreter::InterpreterDebugInterface, loader::LoadedFunction,
    RuntimeEnvironment,
};
use arc_swap::ArcSwap;
use move_vm_types::{instr::Instruction, values::Locals};
use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};

const MOVE_VM_TRACING_ENV_VAR_NAME: &str = "MOVE_VM_TRACE";
const MOVE_VM_STEPPING_ENV_VAR_NAME: &str = "MOVE_VM_STEP";

static FILE_PATH: OnceLock<ArcSwap<String>> = OnceLock::new();
static TRACING_ENABLED: OnceLock<AtomicBool> = OnceLock::new();
static DEBUGGING_ENABLED: OnceLock<AtomicBool> = OnceLock::new();
static LOGGING_FILE_WRITER: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();
static DEBUG_CONTEXT: OnceLock<Mutex<DebugContext>> = OnceLock::new();

/// Turn tracing on or off, saving info to the path.
pub fn enable_tracing(path_opt: Option<&str>) {
    if let Some(path) = path_opt {
        is_tracing_enabled().store(true, Ordering::Release);
        get_file_path().store(Arc::new(path.to_owned()));
        *get_logging_file_writer().lock().unwrap() = create_buffered_output(&PathBuf::from(path));
    } else {
        is_tracing_enabled().store(false, Ordering::Release);
        // Ignore current value of file writer, if we turn tracing on again,
        // it will be overridden.
        get_file_path().store(Arc::new(String::new()))
    }
}

pub fn get_file_path() -> &'static ArcSwap<String> {
    FILE_PATH.get_or_init(|| {
        let initial_val = env::var(MOVE_VM_TRACING_ENV_VAR_NAME).unwrap_or_default();
        ArcSwap::new(Arc::new(initial_val))
    })
}

#[inline]
pub(crate) fn is_tracing_enabled() -> &'static AtomicBool {
    TRACING_ENABLED.get_or_init(|| AtomicBool::new(!get_file_path().load().is_empty()))
}

#[inline]
pub(crate) fn is_debugging_enabled() -> &'static AtomicBool {
    DEBUGGING_ENABLED
        .get_or_init(|| AtomicBool::new(env::var(MOVE_VM_STEPPING_ENV_VAR_NAME).is_ok()))
}

pub fn flush_tracing_buffer() {
    if is_tracing_enabled().load(Ordering::Relaxed) {
        let buf_writer = &mut *get_logging_file_writer().lock().unwrap();
        buf_writer.flush().unwrap();
    }
}

pub fn clear_tracing_buffer() {
    if is_tracing_enabled().load(Ordering::Relaxed) {
        let path = PathBuf::from(get_file_path().load().as_str());
        *get_logging_file_writer().lock().unwrap() = create_buffered_output(&path);
    }
}

fn get_logging_file_writer() -> &'static Mutex<BufWriter<File>> {
    LOGGING_FILE_WRITER.get_or_init(|| {
        Mutex::new(create_buffered_output(&PathBuf::from(
            get_file_path().load().as_str(),
        )))
    })
}

fn create_buffered_output(path: &Path) -> BufWriter<File> {
    let file = OpenOptions::new()
        .write(true) // we want to write
        .create(true) // if file does not exist, create
        .truncate(true) // if file exists, truncate
        .open(path)
        .unwrap();
    BufWriter::with_capacity(4096 * 1024 /* 4096KB */, file)
}

pub(crate) fn get_debug_context() -> &'static Mutex<DebugContext> {
    DEBUG_CONTEXT.get_or_init(|| Mutex::new(DebugContext::new()))
}

pub(crate) fn debug_trace(
    function: &LoadedFunction,
    locals: &Locals,
    pc: u16,
    instr: &Instruction,
    runtime_environment: &RuntimeEnvironment,
    interpreter: &dyn InterpreterDebugInterface,
) {
    if is_tracing_enabled().load(Ordering::Relaxed) {
        let buf_writer = &mut *get_logging_file_writer().lock().unwrap();
        buf_writer
            .write_fmt(format_args!(
                "{},{}\n",
                function.name_as_pretty_string(),
                pc,
            ))
            .unwrap();
    }
    if is_debugging_enabled().load(Ordering::Relaxed) {
        get_debug_context().lock().unwrap().debug_loop(
            function,
            locals,
            pc,
            instr,
            runtime_environment,
            interpreter,
        );
    }
}
