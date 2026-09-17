// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`VMInternalError`] is a type-erased VM error: a boxed [`IntoExecutionError`]
//! that any subsystem's concrete error (`RuntimeError`, `LoaderError`, ...)
//! converts into. Functions return [`VMResult`], so `?` propagates a callee's
//! error without the caller's signature naming (and thus depending on) the
//! callee's concrete error type.
//!
//! A [`VMInternalError`] may also carry an [`ErrorLocation`]. Attaching a
//! location does not replace the underlying error, so callers can still recover
//! its concrete type with [`VMInternalError::downcast_ref`].

use crate::{BytecodeOffset, ExecutionError, ExecutionErrorKind, IntoExecutionError};
use move_binary_format::file_format::FunctionDefinitionIndex;
use move_core_types::language_storage::ModuleId;
use std::{any::Any, fmt};

pub type VMResult<T> = Result<T, VMInternalError>;

/// Where the interpreter attributed a VM error.
///
/// Subsystem errors remain location-independent. The interpreter attaches a
/// location when it knows the executing frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorLocation {
    /// A Move instruction: its module, defining function, and offset in that
    /// function's original bytecode.
    Instruction {
        module: ModuleId,
        function: FunctionDefinitionIndex,
        offset: BytecodeOffset,
    },
    /// Code in `module` with no attributable instruction.
    Module(ModuleId),
    /// A script instruction, by its offset in the script's original bytecode.
    ScriptInstruction { offset: BytecodeOffset },
    /// Script code with no attributable instruction.
    Script,
}

pub struct VMInternalError(Box<ErrorData>);

/// Boxed error data that keeps [`VMInternalError`] one word wide. Every VM
/// function returns [`VMResult`], so the result size affects the interpreter's
/// hot path while allocation occurs only when raising an error.
struct ErrorData {
    error: Box<dyn IntoExecutionError>,
    /// [`None`] for an error raised outside Move code.
    location: Option<ErrorLocation>,
}

const _: () = assert!(std::mem::size_of::<VMInternalError>() == 8);

impl VMInternalError {
    pub fn new(err: impl IntoExecutionError) -> Self {
        VMInternalError(Box::new(ErrorData {
            error: Box::new(err),
            location: None,
        }))
    }

    pub fn kind(&self) -> ExecutionErrorKind {
        self.0.error.kind()
    }

    pub fn downcast_ref<T: IntoExecutionError>(&self) -> Option<&T> {
        (&*self.0.error as &dyn Any).downcast_ref::<T>()
    }

    /// Attaches `location` only if no location is already present.
    pub fn at(mut self, location: ErrorLocation) -> Self {
        self.0.location.get_or_insert(location);
        self
    }

    /// Returns the attached location, or [`None`] if none was attached.
    pub fn location(&self) -> Option<&ErrorLocation> {
        self.0.location.as_ref()
    }
}

impl fmt::Display for VMInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.error.fmt(f)
    }
}

impl fmt::Debug for VMInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VMInternalError({})", self.0.error)
    }
}

impl std::error::Error for VMInternalError {}

impl<E: IntoExecutionError> From<E> for VMInternalError {
    fn from(err: E) -> Self {
        VMInternalError::new(err)
    }
}

impl From<VMInternalError> for ExecutionError {
    fn from(err: VMInternalError) -> Self {
        ExecutionError {
            kind: err.0.error.kind(),
            message: err.0.error.to_string(),
        }
    }
}
