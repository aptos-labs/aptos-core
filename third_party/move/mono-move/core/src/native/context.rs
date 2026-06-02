// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface — the API surface natives are written against.

use super::{result::VMInternalError, value::VMValue};

/// Trait that native functions are written generic over.
///
/// Native authors should depend only on this trait, never on a concrete
/// implementation. This is to enforce a clearer mental boundary and ensure
/// that natives can work with different VM configurations while still being performant.
///
/// ### Convention on Error Handling
///
/// Methods that return only errors that should be propagated back to the VM
/// should use the return type `Result<_, VMInternalError>`.
///
/// Methods that return errors that are triggerable by user inputs or the native's
/// own mistakes should use the return type `Result<Result<_, CustomError>, VMInternalError>` or
/// simply `Result<NativeStatus, CustomError>` if vm internal errors are not possible.
pub trait NativeContext {
    /// Number of positional arguments declared by the native's ABI.
    fn num_args(&self) -> usize;

    /// Reads the `i`-th argument from the calling frame.
    ///
    /// It is guaranteed this would use the correct memory offset and size for the access,
    /// but the caller is still responsible for ensuring the correct type is being read.
    ///
    /// # Safety
    ///
    /// `T` must match the slot's Move-level type.
    unsafe fn arg<T: VMValue>(&self, i: usize) -> Result<T, VMInternalError>;

    /// Number of return slots declared by the native's ABI.
    fn num_returns(&self) -> usize;

    /// Writes the `i`-th return value into the calling frame.
    ///
    /// It is guaranteed this would use the correct memory offset and size for the access,
    /// but the caller is still responsible for ensuring the correct type is being written.
    ///
    /// # Safety
    ///
    /// `T` must match the slot's Move-level type.
    unsafe fn set_return<T: VMValue>(&mut self, i: usize, value: T) -> Result<(), VMInternalError>;

    // TODO: Escape hatches for raw memory reads and writes for better perf.
}
