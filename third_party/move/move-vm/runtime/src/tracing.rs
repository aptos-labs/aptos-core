// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(debug_assertions, feature = "debugging"))]
use crate::debug::DebugContext;
#[cfg(any(debug_assertions, feature = "debugging"))]
use crate::{interpreter::InterpreterDebugInterface, loader::LoadedFunction, RuntimeEnvironment};
#[cfg(any(debug_assertions, feature = "debugging"))]
use ::{
    move_binary_format::file_format::Bytecode,
    move_vm_types::values::Locals,
    once_cell::sync::Lazy,
    std::{
        env,
        fs::{File, OpenOptions},
        io::Write,
        sync::Mutex,
    },
};

#[cfg(any(debug_assertions, feature = "debugging"))]
const MOVE_VM_TRACING_ENV_VAR_NAME: &str = "MOVE_VM_TRACE";

#[cfg(any(debug_assertions, feature = "debugging"))]
const MOVE_VM_STEPPING_ENV_VAR_NAME: &str = "MOVE_VM_STEP";

#[cfg(any(debug_assertions, feature = "debugging"))]
static FILE_PATH: Lazy<String> = Lazy::new(|| {
    env::var(MOVE_VM_TRACING_ENV_VAR_NAME).unwrap_or_else(|_| "move_vm_trace.trace".to_string())
});

#[cfg(any(debug_assertions, feature = "debugging"))]
pub static TRACING_ENABLED: Lazy<bool> =
    Lazy::new(|| env::var(MOVE_VM_TRACING_ENV_VAR_NAME).is_ok());

#[cfg(any(debug_assertions, feature = "debugging"))]
static DEBUGGING_ENABLED: Lazy<bool> =
    Lazy::new(|| env::var(MOVE_VM_STEPPING_ENV_VAR_NAME).is_ok());

#[cfg(any(debug_assertions, feature = "debugging"))]
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

#[cfg(any(debug_assertions, feature = "debugging"))]
static DEBUG_CONTEXT: Lazy<Mutex<DebugContext>> = Lazy::new(|| Mutex::new(DebugContext::new()));

// Only include in debug builds
#[cfg(any(debug_assertions, feature = "debugging"))]
pub(crate) fn trace(
    function: &LoadedFunction,
    locals: &Locals,
    pc: u16,
    instr: &Bytecode,
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

#[macro_export]
macro_rules! trace {
    ($function_desc:expr, $locals:expr, $pc:expr, $instr:tt, $resolver:expr, $interp:expr) => {
        // Only include this code in debug releases
        #[cfg(feature = "debugging")]
        $crate::tracing::trace(&$function_desc, $locals, $pc, &$instr, $resolver, $interp)
    };
}
