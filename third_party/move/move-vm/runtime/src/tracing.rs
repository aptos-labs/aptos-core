// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    debug::DebugContext, interpreter::InterpreterDebugInterface, loader::LoadedFunction,
    RuntimeEnvironment,
};
use move_vm_types::{instr::Instruction, values::Locals};
use once_cell::sync::Lazy;
use std::{
    env,
    fs::{File, OpenOptions},
    io::Write,
    sync::Mutex,
};

const MOVE_VM_TRACING_ENV_VAR_NAME: &str = "MOVE_VM_TRACE";

const MOVE_VM_STEPPING_ENV_VAR_NAME: &str = "MOVE_VM_STEP";

static FILE_PATH: Lazy<String> = Lazy::new(|| {
    env::var(MOVE_VM_TRACING_ENV_VAR_NAME).unwrap_or_else(|_| "move_vm_trace.trace".to_string())
});

pub static TRACING_ENABLED: Lazy<bool> =
    Lazy::new(|| env::var(MOVE_VM_TRACING_ENV_VAR_NAME).is_ok());

static DEBUGGING_ENABLED: Lazy<bool> =
    Lazy::new(|| env::var(MOVE_VM_STEPPING_ENV_VAR_NAME).is_ok());

pub static LOGGING_FILE_WRITER: Lazy<Mutex<std::io::BufWriter<File>>> = Lazy::new(|| {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&*FILE_PATH)
        .unwrap();
    Mutex::new(std::io::BufWriter::with_capacity(
        4096 * 1024, /* 4096KB */
        file,
    ))
});

static DEBUG_CONTEXT: Lazy<Mutex<DebugContext>> = Lazy::new(|| Mutex::new(DebugContext::new()));

pub(crate) fn debug_trace(
    function: &LoadedFunction,
    locals: &Locals,
    pc: u16,
    instr: &Instruction,
    runtime_environment: &RuntimeEnvironment,
    interpreter: &dyn InterpreterDebugInterface,
) {
    if *TRACING_ENABLED {
        let buf_writer = &mut *LOGGING_FILE_WRITER.lock().unwrap();
        buf_writer
            .write_fmt(format_args!(
                "{},{}\n",
                function.name_as_pretty_string(),
                pc,
            ))
            .unwrap();
        buf_writer.flush().unwrap();
    }
    if *DEBUGGING_ENABLED {
        DEBUG_CONTEXT.lock().unwrap().debug_loop(
            function,
            locals,
            pc,
            instr,
            runtime_environment,
            interpreter,
        );
    }
}
