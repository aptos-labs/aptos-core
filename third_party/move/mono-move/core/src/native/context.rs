// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface — the API surface natives are written against.

use super::{
    result::VMInternalError,
    value::{Opaque, Ref, VMValue, Vector},
};
use crate::types::InternedType;

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
///
/// ### Interior mutability
///
/// All methods take `&self` and the context implementation is expected to manage mutations
/// to its components through interior mutability. This is to allow handle-based value
/// representations that are tied to the lifetime of the native call and can safely survive GC.
//
// TODO: add a gas-charging API (e.g. `charge_gas`). Natives currently meter no
// gas; the gas meter is already plumbed into `ProductionNativeContext`.
pub trait NativeContext {
    /// Number of positional arguments declared by the native's ABI.
    fn num_args(&self) -> usize;

    /// Reads the `i`-th argument from the calling frame. A referenced or
    /// allocated value is rooted for the rest of the call, so it survives GC.
    ///
    /// It is guaranteed this would use the correct memory offset and size for the access,
    /// but the caller is still responsible for ensuring the correct type is being read.
    ///
    /// # Safety
    ///
    /// `T` must match the slot's Move-level type.
    unsafe fn arg<'a, T: VMValue<'a>>(&'a self, i: usize) -> Result<T, VMInternalError>;

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
    unsafe fn set_return<'a, T>(&'a self, i: usize, value: T) -> Result<(), VMInternalError>
    where
        T: VMValue<'a>;

    /// Number of type arguments.
    fn num_ty_args(&self) -> usize;

    /// The `i`-th type argument.
    fn ty_arg(&self, i: usize) -> Result<InternedType, VMInternalError>;

    /// Allocates a `vector<u8>` on the VM heap initialized with `bytes` and
    /// returns a handle to it. The vector stays live for the rest of the
    /// native call.
    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> Result<Vector<'a, u8>, VMInternalError>;

    /// Grows the vector behind `target` so it can hold at least `required_cap`
    /// elements of `elem_size` bytes, allocating it if it is empty. When
    /// `target` is empty, the fresh allocation copies its GC descriptor from
    /// `donor`, a non-empty `vector<T>` of the same element type. May trigger
    /// GC; the new heap pointer is written back through `target`'s reference.
    ///
    /// # Safety
    ///
    /// `target` and `donor` must reference `vector<T>` values of the same `T`,
    /// `elem_size` must equal the byte size of `T`, and if `target` is empty
    /// then `donor` must be non-empty.
    unsafe fn grow_vector<'a>(
        &self,
        target: &Ref<'a, Vector<'a, Opaque>>,
        donor: &Ref<'a, Vector<'a, Opaque>>,
        elem_size: usize,
        required_cap: usize,
    ) -> Result<(), VMInternalError>;
}
