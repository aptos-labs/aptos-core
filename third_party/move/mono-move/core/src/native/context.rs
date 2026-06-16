// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface — the API surface natives are written against.

use super::{
    extension::NativeExtension,
    result::VMInternalError,
    value::{Boxed, Opaque, Ref, TableHandle, VMValue, Vector},
};
use crate::{interner::InternedModuleId, types::InternedType, DescriptorId};
use core::cell::RefMut;
use move_core_types::account_address::AccountAddress;

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

    /// Writes `bytes` as the in-frame representation of the `i`-th return value.
    /// `bytes.len()` must equal the slot's size.
    ///
    /// # Safety
    ///
    /// `bytes` must be a valid in-frame representation of the slot's Move-level
    /// type. Any heap pointers it embeds must reference live objects.
    unsafe fn set_return_raw(&self, i: usize, bytes: &[u8]) -> Result<(), VMInternalError>;

    /// Unboxes `value` into the `i`-th return slot: copies the slot's worth of
    /// inline bytes out of the heap object. Used to return a value moved out of
    /// global storage (e.g. `table::remove_box`'s `Box<V>`), whose stored form
    /// is a heap object but whose return slot is the inline value.
    ///
    /// # Safety
    ///
    /// `value` must be a live heap object whose data region holds at least the
    /// return slot's size of bytes, laid out as the slot's Move-level type.
    unsafe fn set_return_unboxed(
        &self,
        i: usize,
        value: &Boxed<'_, Opaque>,
    ) -> Result<(), VMInternalError>;

    /// Structurally compares the values behind reference arguments `i` and `j`,
    /// both of type `ty`, with MonoMove's natural ordering. Used by
    /// `cmp::compare`.
    ///
    /// # Safety
    ///
    /// Arguments `i` and `j` must be references (`&T`) to live values of type
    /// `ty`.
    unsafe fn compare_args(
        &self,
        i: usize,
        j: usize,
        ty: InternedType,
    ) -> Result<core::cmp::Ordering, VMInternalError>;

    /// Number of type arguments.
    fn num_ty_args(&self) -> usize;

    /// The `i`-th type argument.
    fn ty_arg(&self, i: usize) -> Result<InternedType, VMInternalError>;

    /// The type of the `i`-th return value. Used by natives whose return type is
    /// not one of their type arguments (e.g. `cmp::compare` returning
    /// `Ordering`) and so cannot be named via [`Self::ty_arg`].
    fn return_ty(&self, i: usize) -> Result<InternedType, VMInternalError>;

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

    /// BCS-serializes the value of type `ty` stored at `base`.
    ///
    /// # Safety
    ///
    /// `base` must point to a fully initialized value of type `ty` that stays
    /// live for the duration of the call.
    unsafe fn bcs_serialize_value(
        &self,
        base: *const u8,
        ty: InternedType,
    ) -> Result<Vec<u8>, VMInternalError>;

    /// BCS serialized size of the value of type `ty` stored at `base`.
    ///
    /// # Safety
    ///
    /// `base` must point to a fully initialized value of type `ty` that stays
    /// live for the duration of the call.
    unsafe fn bcs_serialized_size(
        &self,
        base: *const u8,
        ty: InternedType,
    ) -> Result<usize, VMInternalError>;

    /// Deserializes `bytes` as a value of type `ty`, returning its in-frame
    /// representation.
    ///
    /// The returned bytes may embed pointers to freshly allocated, unrooted heap
    /// objects, so they must be written into a frame slot before any further
    /// heap allocation.
    fn bcs_deserialize_value(
        &self,
        ty: InternedType,
        bytes: &[u8],
    ) -> Result<Vec<u8>, VMInternalError>;

    /// Whether a resource of type `ty` exists at `address` in global storage.
    //
    // TODO: see if the specializer can lower the caller (object::exists_at) to
    // the `Exists` micro-op directly, dropping this native path.
    fn resource_exists(
        &self,
        address: AccountAddress,
        ty: InternedType,
    ) -> Result<bool, VMInternalError>;

    /// BCS-serializes the by-value argument `i` of type `ty` (e.g. a table key).
    fn bcs_serialize_arg(&self, i: usize, ty: InternedType) -> Result<Vec<u8>, VMInternalError>;

    /// The `i`-th GC descriptor the native requires.
    fn required_descriptor(&self, i: usize) -> Option<DescriptorId>;

    /// Boxes the by-value argument `value_arg` into a fresh heap object built
    /// from `descriptor`, returning an owned handle that stays live for the
    /// rest of the call.
    fn box_arg<'a>(
        &'a self,
        value_arg: usize,
        descriptor: DescriptorId,
    ) -> Result<Boxed<'a, Opaque>, VMInternalError>;

    /// Whether a table entry exists at `(handle, key)`. `value_ty` is the
    /// stored value's type (`Box<V>`), needed to materialize the entry on a
    /// provider read.
    fn table_contains(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> Result<bool, VMInternalError>;

    /// Borrows the table entry at `(handle, key)`, returning a reference to it.
    /// Returns `None` if the entry does not exist. `value_ty` is the stored
    /// value's type (`Box<V>`), needed to materialize the entry.
    fn table_borrow(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
        mutable: bool,
    ) -> Result<Option<Ref<'_, Opaque>>, VMInternalError>;

    /// Adds an already-boxed `value` to the table referenced by `handle` under
    /// `key`. Returns false if an entry already exists at `key`. `value_ty` is
    /// the stored value's type (`Box<V>`).
    fn table_add(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
        value: Boxed<'_, Opaque>,
    ) -> Result<bool, VMInternalError>;

    /// Removes the table entry at `(handle, key)`, returning the moved-out
    /// value, or `None` if no entry exists. `value_ty` is the stored value's
    /// type (`Box<V>`).
    fn table_remove(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> Result<Option<Boxed<'_, Opaque>>, VMInternalError>;

    /// Obtains a mutable reference to the extension of type `T`.
    ///
    /// Errors if `T` is not installed, or if it is already borrowed.
    fn get_extension<T: NativeExtension>(&self) -> Result<RefMut<'_, T>, VMInternalError>;

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
