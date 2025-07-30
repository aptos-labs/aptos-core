// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    file_format::{CodeOffset, FunctionDefinitionIndex, TableIndex},
    IndexKind,
};
use move_core_types::{
    language_storage::ModuleId,
    vm_status::{self, StatusCode, StatusType, VMStatus},
};
use once_cell::sync::{Lazy, OnceCell};
use std::fmt;

pub type VMResult<T> = ::std::result::Result<T, VMError>;
pub type BinaryLoaderResult<T> = ::std::result::Result<T, PartialVMError>;
pub type PartialVMResult<T> = ::std::result::Result<T, PartialVMError>;

static STABLE_TEST_DISPLAY: OnceCell<bool> = OnceCell::new();

/// Call this function if display of errors should be stable for baseline tests.
/// Specifically, no stack traces should be generated, as they contain transitive
/// file locations.
pub fn set_stable_test_display() {
    STABLE_TEST_DISPLAY.set(true).unwrap_or(())
}

/// Check whether stable test display is enabled. This can be used by other components
/// to adjust their output.
pub fn is_stable_test_display() -> bool {
    STABLE_TEST_DISPLAY.get().copied().unwrap_or(false)
}

/// This macro is used to panic while debugging fuzzing crashes obtaining the right stack trace.
/// e.g. DEBUG_VM_STATUS=ABORTED,UNKNOWN_INVARIANT_VIOLATION_ERROR ./fuzz.sh run move_aptosvm_publish_and_run <testcase>
/// third_party/move/move-core/types/src/vm_status.rs:506 for the list of status codes.
#[cfg(feature = "fuzzing")]
macro_rules! fuzzing_maybe_panic {
    ($major_status:expr, $message:expr) => {{
        if let Ok(debug_statuses) = std::env::var("DEBUG_VM_STATUS") {
            if debug_statuses
                .split(',')
                .any(|s| s.trim() == format!("{:?}", $major_status))
            {
                panic!("PartialVMError: {:?} {:?}", $major_status, $message);
            }
        }
    }};
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Location {
    Undefined,
    Script,
    Module(ModuleId),
}

/// A representation of the execution state (e.g., stack trace) at an
/// error point.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExecutionState {
    stack_trace: Vec<(Option<ModuleId>, FunctionDefinitionIndex, CodeOffset)>,
    // we may consider adding more state if necessary
}

impl ExecutionState {
    pub fn new(stack_trace: Vec<(Option<ModuleId>, FunctionDefinitionIndex, CodeOffset)>) -> Self {
        Self { stack_trace }
    }

    pub fn stack_trace(&self) -> &Vec<(Option<ModuleId>, FunctionDefinitionIndex, CodeOffset)> {
        &self.stack_trace
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct VMError(Box<VMError_>);

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct VMError_ {
    major_status: StatusCode,
    sub_status: Option<u64>,
    message: Option<String>,
    exec_state: Option<ExecutionState>,
    location: Location,
    indices: Vec<(IndexKind, TableIndex)>,
    offsets: Vec<(FunctionDefinitionIndex, CodeOffset)>,
}

impl VMError {
    pub fn into_vm_status(self) -> VMStatus {
        let VMError_ {
            major_status,
            sub_status,
            location,
            message,
            mut offsets,
            ..
        } = *self.0;
        match (major_status, sub_status, location) {
            (StatusCode::EXECUTED, sub_status, _) => {
                debug_assert!(sub_status.is_none());
                VMStatus::Executed
            },
            (StatusCode::ABORTED, Some(code), Location::Script) => {
                VMStatus::MoveAbort(vm_status::AbortLocation::Script, code)
            },
            (StatusCode::ABORTED, Some(code), Location::Module(id)) => {
                VMStatus::MoveAbort(vm_status::AbortLocation::Module(id), code)
            },

            (StatusCode::ABORTED, sub_status, location) => {
                debug_assert!(
                    false,
                    "Expected a code and module/script location with ABORTED, but got {:?} and {}",
                    sub_status, location
                );
                VMStatus::Error {
                    status_code: StatusCode::ABORTED,
                    sub_status,
                    message,
                }
            },

            (major_status, sub_status, location)
                if major_status.status_type() == StatusType::Execution =>
            {
                let abort_location = match &location {
                    Location::Script => vm_status::AbortLocation::Script,
                    Location::Module(id) => vm_status::AbortLocation::Module(id.clone()),
                    Location::Undefined => {
                        return VMStatus::Error {
                            status_code: major_status,
                            sub_status,
                            message,
                        };
                    },
                };
                // offset can be None if it comes from `check_dependencies_and_charge_gas` for example
                let (function, code_offset) = match offsets.pop() {
                    None => {
                        return VMStatus::Error {
                            status_code: major_status,
                            sub_status,
                            message,
                        };
                    },
                    Some((fdef_idx, code_offset)) => (fdef_idx.0, code_offset),
                };
                VMStatus::ExecutionFailure {
                    status_code: major_status,
                    location: abort_location,
                    function,
                    code_offset,
                    sub_status,
                    message,
                }
            },

            (major_status, sub_status, _) => VMStatus::Error {
                status_code: major_status,
                sub_status,
                message,
            },
        }
    }

    pub fn major_status(&self) -> StatusCode {
        self.0.major_status
    }

    pub fn set_major_status(&mut self, major_status: StatusCode) {
        self.0.major_status = major_status;
    }

    pub fn sub_status(&self) -> Option<u64> {
        self.0.sub_status
    }

    pub fn set_sub_status(&mut self, status: u64) {
        self.0.sub_status = Some(status);
    }

    pub fn message(&self) -> Option<&String> {
        self.0.message.as_ref()
    }

    pub fn exec_state(&self) -> Option<&ExecutionState> {
        self.0.exec_state.as_ref()
    }

    pub fn remove_exec_state(&mut self) {
        self.0.exec_state = None;
    }

    pub fn location(&self) -> &Location {
        &self.0.location
    }

    pub fn indices(&self) -> &Vec<(IndexKind, TableIndex)> {
        &self.0.indices
    }

    pub fn offsets(&self) -> &Vec<(FunctionDefinitionIndex, CodeOffset)> {
        &self.0.offsets
    }

    pub fn status_type(&self) -> StatusType {
        self.0.major_status.status_type()
    }

    pub fn all_data(
        self,
    ) -> (
        StatusCode,
        Option<u64>,
        Option<String>,
        Option<ExecutionState>,
        Location,
        Vec<(IndexKind, TableIndex)>,
        Vec<(FunctionDefinitionIndex, CodeOffset)>,
    ) {
        let VMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            location,
            indices,
            offsets,
        } = *self.0;
        (
            major_status,
            sub_status,
            message,
            exec_state,
            location,
            indices,
            offsets,
        )
    }

    pub fn to_partial(self) -> PartialVMError {
        let VMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
            ..
        } = *self.0;
        PartialVMError(Box::new(PartialVMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
        }))
    }

    pub fn format_test_output(&self, verbose: bool) -> String {
        let location_string = match &self.location() {
            Location::Undefined => "undefined".to_owned(),
            Location::Script => "script".to_owned(),
            Location::Module(id) => {
                format!("0x{}::{}", id.address().short_str_lossless(), id.name())
            },
        };
        let indices = format!("{:?}", self.indices());
        let offsets = format!("{:?}", self.offsets());

        if verbose {
            let message_str = match &self.message() {
                Some(message_str) => message_str,
                None => "None",
            };
            format!(
                "{{
    message: {message},
    major_status: {major_status:?},
    sub_status: {sub_status:?},
    location: {location_string},
    indices: {indices},
    offsets: {offsets},
    exec_state: {exec_state:?},
}}",
                message = message_str,
                major_status = self.major_status(),
                sub_status = self.sub_status(),
                location_string = location_string,
                exec_state = self.exec_state(),
                // TODO maybe include source map info?
                indices = indices,
                offsets = offsets,
            )
        } else {
            format!(
                "{{
    major_status: {major_status:?},
    sub_status: {sub_status:?},
    location: {location_string},
    indices: {indices},
    offsets: {offsets},
}}",
                major_status = self.major_status(),
                sub_status = self.sub_status(),
                location_string = location_string,
                // TODO maybe include source map info?
                indices = indices,
                offsets = offsets,
            )
        }
    }
}

impl fmt::Debug for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Debug for VMError_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major_status,
            sub_status,
            message,
            exec_state,
            location,
            indices,
            offsets,
        } = self;
        f.debug_struct("VMError")
            .field("major_status", major_status)
            .field("sub_status", sub_status)
            .field("message", message)
            .field("exec_state", exec_state)
            .field("location", location)
            .field("indices", indices)
            .field("offsets", offsets)
            .finish()
    }
}

impl std::error::Error for VMError {}

#[derive(Clone)]
pub struct PartialVMError(Box<PartialVMError_>);

#[derive(Clone)]
struct PartialVMError_ {
    major_status: StatusCode,
    sub_status: Option<u64>,
    message: Option<String>,
    exec_state: Option<ExecutionState>,
    indices: Vec<(IndexKind, TableIndex)>,
    offsets: Vec<(FunctionDefinitionIndex, CodeOffset)>,
}

impl PartialVMError {
    pub fn all_data(
        self,
    ) -> (
        StatusCode,
        Option<u64>,
        Option<String>,
        Option<ExecutionState>,
        Vec<(IndexKind, TableIndex)>,
        Vec<(FunctionDefinitionIndex, CodeOffset)>,
    ) {
        let PartialVMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
        } = *self.0;
        (
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
        )
    }

    pub fn finish(self, location: Location) -> VMError {
        let PartialVMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
        } = *self.0;
        static MOVE_TEST_DEBUG: Lazy<bool> = Lazy::new(|| {
            std::env::var("MOVE_TEST_DEBUG").map_or(false, |v| matches!(v.as_str(), "true" | "1"))
        });
        let message = if *MOVE_TEST_DEBUG {
            // Do this only if env var is set. Otherwise, we cannot use the output in baseline files
            // since it is not deterministic.
            let bt = std::backtrace::Backtrace::capture();
            if std::backtrace::BacktraceStatus::Captured == bt.status() {
                if let Some(message) = message {
                    Some(format!("{}\nBacktrace: {:#?}", message, bt).to_string())
                } else {
                    Some(format!("Backtrace: {:#?}", bt).to_string())
                }
            } else {
                message
            }
        } else {
            message
        };

        VMError(Box::new(VMError_ {
            major_status,
            sub_status,
            message,
            exec_state,
            location,
            indices,
            offsets,
        }))
    }

    pub fn new(major_status: StatusCode) -> Self {
        debug_assert!(major_status != StatusCode::EXECUTED);
        let message = if major_status == StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
            && !is_stable_test_display()
        {
            let mut len = 5;
            let mut trace: String = "Unknown invariant violation generated:\n".to_string();
            backtrace::trace(|frame| {
                backtrace::resolve_frame(frame, |symbol| {
                    let mut function_name = backtrace::SymbolName::new("<unknown>".as_bytes());
                    if let Some(name) = symbol.name() {
                        function_name = name;
                    }
                    let mut file_name = "<unknown>";
                    if let Some(filename) = symbol.filename() {
                        if let Some(filename) = filename.to_str() {
                            file_name = filename;
                        }
                    }
                    let lineno = symbol.lineno().unwrap_or(0);
                    trace.push_str(&format!(
                        "In function {} at {}:{}\n",
                        function_name, file_name, lineno
                    ));
                });
                len -= 1;
                len > 0
            });
            Some(trace)
        } else {
            None
        };

        #[cfg(feature = "fuzzing")]
        fuzzing_maybe_panic!(major_status, message);

        Self(Box::new(PartialVMError_ {
            major_status,
            sub_status: None,
            message,
            exec_state: None,
            indices: vec![],
            offsets: vec![],
        }))
    }

    pub fn new_invariant_violation(msg: impl ToString) -> PartialVMError {
        Self::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg.to_string())
    }

    pub fn major_status(&self) -> StatusCode {
        self.0.major_status
    }

    pub fn sub_status(&self) -> Option<u64> {
        self.0.sub_status
    }

    pub fn with_sub_status(mut self, sub_status: u64) -> Self {
        debug_assert!(self.0.sub_status.is_none());
        self.0.sub_status = Some(sub_status);
        self
    }

    pub fn with_message(mut self, mut message: String) -> Self {
        if self.0.major_status == StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR {
            if let Some(stacktrace) = self.0.message.take() {
                message = format!("{} @{}", message, stacktrace);
            }
        }
        debug_assert!(self.0.message.is_none());
        self.0.message = Some(message);
        self
    }

    pub fn with_exec_state(mut self, exec_state: ExecutionState) -> Self {
        debug_assert!(self.0.exec_state.is_none());
        self.0.exec_state = Some(exec_state);
        self
    }

    pub fn message(&self) -> Option<&str> {
        self.0.message.as_deref()
    }

    pub fn at_index(mut self, kind: IndexKind, index: TableIndex) -> Self {
        self.0.indices.push((kind, index));
        self
    }

    pub fn at_indices(mut self, additional_indices: Vec<(IndexKind, TableIndex)>) -> Self {
        self.0.indices.extend(additional_indices);
        self
    }

    pub fn at_code_offset(mut self, function: FunctionDefinitionIndex, offset: CodeOffset) -> Self {
        self.0.offsets.push((function, offset));
        self
    }

    pub fn at_code_offsets(
        mut self,
        additional_offsets: Vec<(FunctionDefinitionIndex, CodeOffset)>,
    ) -> Self {
        self.0.offsets.extend(additional_offsets);
        self
    }

    /// Append the message `message` to the message field of the VM status, and insert a separator
    /// if the original message is non-empty.
    pub fn append_message_with_separator(
        mut self,
        separator: char,
        additional_message: String,
    ) -> Self {
        match self.0.message.as_mut() {
            Some(msg) => {
                if !msg.is_empty() {
                    msg.push(separator);
                }
                msg.push_str(&additional_message);
            },
            None => self.0.message = Some(additional_message),
        };
        self
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Undefined => write!(f, "UNDEFINED"),
            Location::Script => write!(f, "Script"),
            Location::Module(id) => write!(f, "Module {:?}", id),
        }
    }
}

impl fmt::Display for PartialVMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut status = format!("PartialVMError with status {:#?}", self.0.major_status);

        if let Some(sub_status) = self.0.sub_status {
            status = format!("{} with sub status {}", status, sub_status);
        }

        if let Some(msg) = &self.0.message {
            status = format!("{} and message '{}'", status, msg);
        }

        for (kind, index) in &self.0.indices {
            status = format!("{} at index {} for {}", status, index, kind);
        }
        for (fdef, code_offset) in &self.0.offsets {
            status = format!(
                "{} at code offset {} in function definition {}",
                status, code_offset, fdef
            );
        }

        write!(f, "{}", status)
    }
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut status = format!("VMError with status {:#?}", self.0.major_status);

        if let Some(sub_status) = self.0.sub_status {
            status = format!("{} with sub status {}", status, sub_status);
        }

        status = format!("{} at location {}", status, self.0.location);

        if let Some(msg) = &self.0.message {
            status = format!("{} and message {}", status, msg);
        }

        for (kind, index) in &self.0.indices {
            status = format!("{} at index {} for {}", status, index, kind);
        }
        for (fdef, code_offset) in &self.0.offsets {
            status = format!(
                "{} at code offset {} in function definition {}",
                status, code_offset, fdef
            );
        }

        write!(f, "{}", status)
    }
}

////////////////////////////////////////////////////////////////////////////
/// Conversion functions from internal VM statuses into external VM statuses
////////////////////////////////////////////////////////////////////////////
impl From<VMError> for VMStatus {
    fn from(vm_error: VMError) -> VMStatus {
        vm_error.into_vm_status()
    }
}

pub fn vm_status_of_result<T>(result: VMResult<T>) -> VMStatus {
    match result {
        Ok(_) => VMStatus::Executed,
        Err(err) => err.into_vm_status(),
    }
}

pub fn offset_out_of_bounds(
    status: StatusCode,
    kind: IndexKind,
    target_offset: usize,
    target_pool_len: usize,
    cur_function: FunctionDefinitionIndex,
    cur_bytecode_offset: CodeOffset,
) -> PartialVMError {
    let msg = format!(
        "Index {} out of bounds for {} at bytecode offset {} in function {} while indexing {}",
        target_offset, target_pool_len, cur_bytecode_offset, cur_function, kind
    );
    PartialVMError::new(status)
        .with_message(msg)
        .at_code_offset(cur_function, cur_bytecode_offset)
}

pub fn bounds_error(
    status: StatusCode,
    kind: IndexKind,
    idx: TableIndex,
    len: usize,
) -> PartialVMError {
    let msg = format!(
        "Index {} out of bounds for {} while indexing {}",
        idx, len, kind
    );
    PartialVMError::new(status)
        .at_index(kind, idx)
        .with_message(msg)
}

pub fn verification_error(status: StatusCode, kind: IndexKind, idx: TableIndex) -> PartialVMError {
    PartialVMError::new(status).at_index(kind, idx)
}

impl fmt::Debug for PartialVMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Debug for PartialVMError_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major_status,
            sub_status,
            message,
            exec_state,
            indices,
            offsets,
        } = self;
        f.debug_struct("PartialVMError")
            .field("major_status", major_status)
            .field("sub_status", sub_status)
            .field("message", message)
            .field("exec_state", exec_state)
            .field("indices", indices)
            .field("offsets", offsets)
            .finish()
    }
}

impl std::error::Error for PartialVMError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
