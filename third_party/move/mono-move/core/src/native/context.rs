// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface — the API surface natives are written against.

use super::{
    extension::NativeExtension,
    value::{Boxed, Opaque, Ref, TableHandle, VMValue, Vector},
};
use crate::{interner::InternedModuleId, types::InternedType, DescriptorId, VMResult};
use core::{cell::RefMut, cmp::Ordering};
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
/// should use the return type `VMResult<_>`.
///
/// Methods that return errors that are triggerable by user inputs or the native's
/// own mistakes should use the return type `VMResult<Result<_, CustomError>>` or
/// simply `Result<NativeStatus, CustomError>` if vm internal errors are not possible.
///
/// ### Interior mutability
///
/// All methods take `&self` and the context implementation is expected to manage mutations
/// to its components through interior mutability. This is to allow handle-based value
/// representations that are tied to the lifetime of the native call and can safely survive GC.
//
// TODO(metering): add a gas-charging API (e.g. `charge_gas`). Natives currently meter no
// gas; the gas meter is already plumbed into `ProductionNativeContext`.
pub trait NativeContext {
    /// Number of positional arguments declared by the native's ABI.
    fn num_args(&self) -> usize;

    /// Reads the `i`-th argument from the calling frame. A referenced or
    /// allocated value is rooted for the rest of the call, so it survives GC.
    ///
    /// All arguments must be read before any return value is written; an `arg`
    /// call after a `set_return*` fails.
    ///
    /// It is guaranteed this would use the correct memory offset and size for the access,
    /// but the caller is still responsible for ensuring the correct type is being read.
    ///
    /// # Safety
    ///
    /// `T` must match the slot's Move-level type.
    unsafe fn arg<'a, T: VMValue<'a>>(&'a self, i: usize) -> VMResult<T>;

    /// Number of return slots declared by the native's ABI.
    fn num_returns(&self) -> usize;

    /// In-frame byte size of the `i`-th return slot.
    fn return_size(&self, i: usize) -> VMResult<usize>;

    /// Writes the `i`-th return value into the calling frame. Writing any return
    /// value poisons the context against further argument reads.
    ///
    /// It is guaranteed this would use the correct memory offset and size for the access,
    /// but the caller is still responsible for ensuring the correct type is being written.
    ///
    /// # Safety
    ///
    /// `T` must match the slot's Move-level type.
    unsafe fn set_return<'a, T>(&'a self, i: usize, value: T) -> VMResult<()>
    where
        T: VMValue<'a>;

    /// Writes `bytes` as the in-frame representation of the `i`-th return value.
    /// `bytes.len()` must equal the slot's size. Writing any return value poisons
    /// the context against further argument reads.
    ///
    /// # Safety
    ///
    /// `bytes` must be a valid in-frame representation of the slot's Move-level
    /// type. Any heap pointers it embeds must reference live objects.
    unsafe fn set_return_raw(&self, i: usize, bytes: &[u8]) -> VMResult<()>;

    /// Number of type arguments.
    fn num_ty_args(&self) -> usize;

    /// The `i`-th type argument.
    fn ty_arg(&self, i: usize) -> VMResult<InternedType>;

    /// In-memory byte size of a value of type `ty`. Errors if the type has no
    /// published layout (e.g. an unresolved generic or an unloaded module).
    fn value_size(&self, ty: InternedType) -> VMResult<u32>;

    /// Returns a copy of the `i`-th argument's raw in-frame bytes -- a low-level
    /// API for natives that need to operate on generic opaque values.
    fn arg_raw(&self, i: usize) -> VMResult<Vec<u8>>;

    /// Returns the heap-pointer offsets within the `i`-th argument,
    /// relative to the value's start.
    fn arg_ptr_offsets(&self, i: usize) -> VMResult<Vec<u32>>;

    /// Returns the module of the caller of the function that invoked this native,
    /// or `None` if the frame is the entry point (which has no caller).
    fn caller_module(&self) -> Option<InternedModuleId>;

    /// Allocates a `vector<u8>` on the VM heap initialized with `bytes` and
    /// returns a handle to it. The vector stays live for the rest of the
    /// native call.
    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> VMResult<Vector<'a, u8>>;

    /// BCS-serializes the value of type `ty` stored at `base`.
    ///
    /// # Safety
    ///
    /// `base` must point to a fully initialized value of type `ty` that stays
    /// live for the duration of the call.
    unsafe fn bcs_serialize_value(&self, base: *const u8, ty: InternedType) -> VMResult<Vec<u8>>;

    /// BCS serialized size of the value of type `ty` stored at `base`.
    ///
    /// # Safety
    ///
    /// `base` must point to a fully initialized value of type `ty` that stays
    /// live for the duration of the call.
    unsafe fn bcs_serialized_size(&self, base: *const u8, ty: InternedType) -> VMResult<usize>;

    /// Deserializes `bytes` as a value of type `ty`, returning its in-frame
    /// representation.
    ///
    /// The returned bytes may embed pointers to freshly allocated, unrooted heap
    /// objects, so they must be written into a frame slot before any further
    /// heap allocation.
    fn bcs_deserialize_value(&self, ty: InternedType, bytes: &[u8]) -> VMResult<Vec<u8>>;

    /// The constant BCS-serialized size of any value of type `ty`.
    /// Returns `None` if the size is data-dependent (e.g. vectors).
    fn constant_serialized_size(&self, ty: InternedType) -> VMResult<Option<u64>>;

    /// Compares the values of type `ty` at `a` and `b` (the natural ordering).
    ///
    /// # Safety
    ///
    /// `a` and `b` must point to valid values of type `ty` that stay
    /// live for the duration of the call.
    unsafe fn compare(&self, a: *const u8, b: *const u8, ty: InternedType) -> VMResult<Ordering>;

    /// Builds an enum value with variant `tag` and payload `value`, tagged with
    /// `descriptor` for GC tracing, and returns an owned handle. Pass `()` for a
    /// fieldless variant.
    ///
    /// # Safety
    ///
    /// `descriptor` must correctly trace the payload's heap pointers (use the
    /// trivial descriptor for a pointer-free payload), and `tag`/`value` must
    /// form a valid representation of the enum variant.
    unsafe fn new_enum<'a, V: VMValue<'a>>(
        &'a self,
        descriptor: DescriptorId,
        tag: u64,
        value: V,
    ) -> VMResult<Boxed<'a, Opaque>>;

    /// Whether a resource of type `ty` exists at `address` in global storage.
    //
    // TODO(cleanup): see if the specializer can lower the caller (object::exists_at) to
    // the `Exists` micro-op directly, dropping this native path.
    fn resource_exists(&self, address: AccountAddress, ty: InternedType) -> VMResult<bool>;

    /// BCS-serializes the by-value argument `i` of type `ty` (e.g. a table key).
    fn bcs_serialize_arg(&self, i: usize, ty: InternedType) -> VMResult<Vec<u8>>;

    /// The `i`-th GC descriptor the native requires.
    fn required_descriptor(&self, i: usize) -> Option<DescriptorId>;

    /// Boxes the by-value argument `value_arg` into a fresh heap object built
    /// from `descriptor`, returning an owned handle that stays live for the
    /// rest of the call.
    fn box_arg<'a>(
        &'a self,
        value_arg: usize,
        descriptor: DescriptorId,
    ) -> VMResult<Boxed<'a, Opaque>>;

    // The table ops below take `value_ty`, the entry's stored type, so a
    // working-map miss can resolve its layout and deserialize from storage.

    /// Whether a table entry exists at `(handle, key)`.
    fn table_contains(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> VMResult<bool>;

    /// Borrows the table entry at `(handle, key)`, returning a reference to it.
    /// Returns `None` if the entry does not exist.
    fn table_borrow(
        &self,
        handle: &TableHandle,
        key: &[u8],
        mutable: bool,
        value_ty: InternedType,
    ) -> VMResult<Option<Ref<'_, Opaque>>>;

    /// Adds an already-boxed `value` to the table referenced by `handle` under
    /// `key`. Returns false if an entry already exists at `key`.
    fn table_add(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value: Boxed<'_, Opaque>,
        value_ty: InternedType,
    ) -> VMResult<bool>;

    /// Removes the table entry at `(handle, key)`, returning its value as an
    /// boxed object. Returns `None` if the entry does not exist.
    fn table_remove(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> VMResult<Option<Boxed<'_, Opaque>>>;

    /// Obtains a mutable reference to the extension of type `T`.
    ///
    /// Errors if `T` is not installed, or if it is already borrowed.
    fn get_extension<T: NativeExtension>(&self) -> VMResult<RefMut<'_, T>>;
}
