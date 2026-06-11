// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface — the API surface natives are written against.

use super::{
    extension::NativeExtension,
    result::VMInternalError,
    value::{Opaque, Ref, VMValue, Vector},
};
use crate::{interner::InternedModuleId, types::InternedType};
use core::cell::RefMut;

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

    /// Returns a copy of the `i`-th argument's raw in-frame bytes -- a low-level
    /// API for natives that need to operate on generic opaque values.
    fn arg_raw(&self, i: usize) -> Result<Vec<u8>, VMInternalError>;

    /// Returns the heap-pointer offsets within the `i`-th argument,
    /// relative to the value's start.
    fn arg_ptr_offsets(&self, i: usize) -> Result<Vec<u32>, VMInternalError>;

    /// Returns the module of the caller of the function that invoked this native,
    /// or `None` if the frame is the entry point (which has no caller).
    fn caller_module(&self) -> Option<InternedModuleId>;

    /// Allocates a `vector<u8>` on the VM heap initialized with `bytes` and
    /// returns a handle to it. The vector stays live for the rest of the
    /// native call.
    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> Result<Vector<'a, u8>, VMInternalError>;

    /// Obtains a mutable reference to the extension of type `T`.
    ///
    /// Errors if `T` is not installed, or if it is already borrowed.
    fn get_extension<T: NativeExtension>(&self) -> Result<RefMut<'_, T>, VMInternalError>;

    /// BCS-serializes the value behind `value` (a `&T` reference) whose type is
    /// `ty`, returning the encoded bytes.
    ///
    /// `Ok(None)` means the value is well-typed but not BCS-serializable (e.g. a
    /// sequence longer than BCS allows); the caller maps this to a
    /// serialization abort. VM-internal failures use the error channel.
    fn serialize(
        &self,
        ty: InternedType,
        value: &Ref<'_, Opaque>,
    ) -> Result<Option<Vec<u8>>, VMInternalError>;

    /// Returns the BCS serialized size of the value behind `value` of type `ty`,
    /// without retaining the encoded bytes. Same failure semantics as
    /// [`Self::serialize`].
    fn serialized_size(
        &self,
        ty: InternedType,
        value: &Ref<'_, Opaque>,
    ) -> Result<Option<usize>, VMInternalError>;
}
