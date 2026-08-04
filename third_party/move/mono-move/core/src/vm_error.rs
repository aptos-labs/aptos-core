// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`VMInternalError`] is a type-erased VM error: a boxed [`IntoExecutionError`]
//! that any subsystem's concrete error (`RuntimeError`, `LoaderError`, ...)
//! converts into. Functions return [`VMResult`], so `?` propagates a callee's
//! error without the caller's signature naming (and thus depending on) the
//! callee's concrete error type.

use crate::{ExecutionError, ExecutionErrorKind, IntoExecutionError};
use std::{any::Any, fmt};

pub type VMResult<T> = Result<T, VMInternalError>;

pub struct VMInternalError(Box<dyn IntoExecutionError>);

impl VMInternalError {
    pub fn new(err: impl IntoExecutionError) -> Self {
        VMInternalError(Box::new(err))
    }

    pub fn kind(&self) -> ExecutionErrorKind {
        self.0.kind()
    }

    pub fn downcast_ref<T: IntoExecutionError>(&self) -> Option<&T> {
        (&*self.0 as &dyn Any).downcast_ref::<T>()
    }
}

impl fmt::Display for VMInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for VMInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VMInternalError({})", self.0)
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
            kind: err.0.kind(),
            message: err.0.to_string(),
        }
    }
}
