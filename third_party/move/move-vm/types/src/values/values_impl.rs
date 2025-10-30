// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    delayed_values::delayed_field_id::{DelayedFieldID, TryFromMoveValue, TryIntoMoveValue},
    loaded_data::runtime_types::Type,
    value_serde::ValueSerDeContext,
    values::function_values_impl::{AbstractFunction, Closure, ClosureVisitor},
    views::{ValueView, ValueVisitor},
};
use itertools::Itertools;
use move_binary_format::{
    errors::*,
    file_format::{Constant, SignatureToken, VariantIndex},
};
#[cfg(any(test, feature = "fuzzing", feature = "testing"))]
use move_core_types::value::{MoveStruct, MoveValue};
use move_core_types::{
    account_address::AccountAddress,
    effects::Op,
    int256,
    value::{
        self, MoveStructLayout, MoveTypeLayout, MASTER_ADDRESS_FIELD_OFFSET, MASTER_SIGNER_VARIANT,
        PERMISSIONED_SIGNER_VARIANT, PERMISSION_ADDRESS_FIELD_OFFSET,
    },
    vm_status::{sub_status::NFE_VECTOR_ERROR_BASE, StatusCode},
};
use serde::{
    de::{EnumAccess, Error as DeError, Unexpected, VariantAccess},
    ser::{Error as SerError, SerializeSeq, SerializeTuple, SerializeTupleVariant},
    Deserialize,
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter},
    iter, mem,
    rc::Rc,
};
use triomphe::Arc as TriompheArc;

/// Values can be recursive, and so it is important that we do not use recursive algorithms over
/// deeply nested values as it can cause stack overflow. Since it is not always possible to avoid
/// recursion, we opt for a reasonable limit on VM value depth. It is defined in Move VM config,
/// but since it is difficult to propagate config context everywhere, we use this constant.
///
/// IMPORTANT: When changing this constant, make sure it is in-sync with one in VM config (it is
/// used there now).
pub const DEFAULT_MAX_VM_VALUE_NESTED_DEPTH: u64 = 128;

/***************************************************************************************
 *
 * Types
 *
 *   Representation of the Move value calculus. These types are abstractions
 *   over the concrete Move concepts and may carry additional information that is not
 *   defined by the language, but required by the implementation.
 *
 **************************************************************************************/

/// Runtime representation of a Move value.
#[allow(private_interfaces)] // because of Container, ContainerRef, IndexedRef
pub enum Value {
    Invalid,

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(int256::U256),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    I256(int256::I256),
    Bool(bool),
    Address(AccountAddress),

    Container(Container),

    ContainerRef(ContainerRef),
    IndexedRef(IndexedRef),

    /// Delayed values are values that live outside of MoveVM and are processed in
    /// a delayed (some may it call lazy) fashion, e.g., aggregators or snapshots.
    /// The implementation stores a unique identifier so that the value can be
    /// fetched and processed by native functions.
    ///
    /// Delayed values are sized, and the variant carries the information about
    /// the serialized size of the external Move value. This allows to make sure
    /// size information is known, e.g. for gas metering purposes.
    ///
    /// Delayed values should not be displayed in any way, to ensure we do not
    /// accidentally introduce non-determinism if identifiers are generated at
    /// random. For that reason, `Debug` is not derived for `ValueImpl` enum and
    /// is implemented directly.
    ///
    /// Semantics:
    ///   - Delayed values cannot be compared. An equality check results in a
    ///     runtime error. As a result, equality for any Move value that contains
    ///     a delayed value stops being reflexive, symmetric and transitive, and
    ///     results in a runtime error as well.
    ///   - Delayed values cannot be serialized and stored in the global blockchain
    ///     state because they are used purely at runtime. Any attempt to serialize
    ///     a delayed value, e.g. using `0x1::bcs::to_bytes` results in a runtime
    ///     error.
    DelayedFieldID {
        id: DelayedFieldID,
    },

    /// A closure, consisting of a function reference and captured arguments.
    /// Notice that captured arguments cannot be referenced, hence a closure is
    /// not a container.
    ClosureValue(Closure),
}

/// A container is a collection of values. It is used to represent data structures like a
/// Move vector or struct.
///
/// There is one general container that can be used to store an array of any values, same
/// type or not, and a few specialized flavors to offer compact memory layout for small
/// primitive types.
///
/// Except when not owned by the VM stack, a container always lives inside an Rc<RefCell<>>,
/// making it possible to be shared by references.
#[derive(Debug)]
pub(crate) enum Container {
    Locals(Rc<RefCell<Vec<Value>>>),
    Vec(Rc<RefCell<Vec<Value>>>),
    Struct(Rc<RefCell<Vec<Value>>>),
    VecU8(Rc<RefCell<Vec<u8>>>),
    VecU64(Rc<RefCell<Vec<u64>>>),
    VecU128(Rc<RefCell<Vec<u128>>>),
    VecBool(Rc<RefCell<Vec<bool>>>),
    VecAddress(Rc<RefCell<Vec<AccountAddress>>>),
    VecU16(Rc<RefCell<Vec<u16>>>),
    VecU32(Rc<RefCell<Vec<u32>>>),
    VecU256(Rc<RefCell<Vec<int256::U256>>>),
    VecI8(Rc<RefCell<Vec<i8>>>),
    VecI16(Rc<RefCell<Vec<i16>>>),
    VecI32(Rc<RefCell<Vec<i32>>>),
    VecI64(Rc<RefCell<Vec<i64>>>),
    VecI128(Rc<RefCell<Vec<i128>>>),
    VecI256(Rc<RefCell<Vec<int256::I256>>>),
}

/// A ContainerRef is a direct reference to a container, which could live either in the frame
/// or in global storage. In the latter case, it also keeps a status flag indicating whether
/// the container has been possibly modified.
#[derive(Debug)]
pub(crate) enum ContainerRef {
    Local(Container),
    Global {
        status: Rc<RefCell<GlobalDataStatus>>,
        container: Container,
    },
}

/// Status for global (on-chain) data:
/// Clean - the data was only read.
/// Dirty - the data was possibly modified.
#[derive(Debug, Clone, Copy)]
pub(crate) enum GlobalDataStatus {
    Clean,
    Dirty,
}

/// A Move reference pointing to an element in a container. Used only for primitive types, e.g.,
/// vectors of integers or an integer field in a struct.
#[derive(Debug)]
pub(crate) struct IndexedRef {
    idx: usize,
    container_ref: ContainerRef,
}

/// An umbrella enum for references. It is used to hide the internals of the public type
/// Reference.
#[derive(Debug)]
enum ReferenceImpl {
    IndexedRef(IndexedRef),
    ContainerRef(ContainerRef),
}

/***************************************************************************************
 *
 * Public Types
 *
 *   Types visible from outside the module. They are almost exclusively wrappers around
 *   the internal representation, acting as public interfaces. The methods they provide
 *   closely resemble the Move concepts their names suggest: move_local, borrow_field,
 *   pack, unpack, etc.
 *
 *   They are opaque to an external caller by design -- no knowledge about the internal
 *   representation is given and they can only be manipulated via the public methods,
 *   which is to ensure no arbitrary invalid states can be created unless some crucial
 *   internal invariants are violated.
 *
 **************************************************************************************/

/// A Move struct.
#[derive(Debug)]
pub struct Struct {
    fields: Vec<Value>,
}

// A vector. This is an alias for a Container for now but we may change
// it once Containers are restructured.
// It's used from vector native functions to get a vector and operate on that.
// There is an impl for Vector which implements the API private to this module.
#[derive(Debug)]
pub struct Vector(Container);

/// A reference to a Move struct that allows you to take a reference to one of its fields.
#[derive(Debug)]
pub struct StructRef(ContainerRef);

/// A generic Move reference that offers two functionalities: read_ref & write_ref.
#[derive(Debug)]
pub struct Reference(ReferenceImpl);

// A reference to a signer. Clients can attempt a cast to this struct if they are
// expecting a Signer on the stack or as an argument.
#[derive(Debug)]
pub struct SignerRef(ContainerRef);

// A reference to a vector. This is an alias for a ContainerRef for now but we may change
// it once Containers are restructured.
// It's used from vector native functions to get a reference to a vector and operate on that.
// There is an impl for VectorRef which implements the API private to this module.
#[derive(Debug)]
pub struct VectorRef(ContainerRef);

/// A special "slot" in global storage that can hold a resource. It also keeps track of the status
/// of the resource relative to the global state, which is necessary to compute the effects to emit
/// at the end of transaction execution.
#[derive(Debug)]
enum GlobalValueImpl {
    /// No resource resides in this slot or in storage.
    None,
    /// A resource has been published to this slot and it did not previously exist in storage. The
    /// invariant is that the value is a struct.
    Fresh { value: Value },
    /// A resource resides in this slot and also in storage. The status flag indicates whether
    /// it has potentially been altered.
    Cached {
        /// A struct value representing this resource (invariant).
        value: Value,
        status: Rc<RefCell<GlobalDataStatus>>,
    },
    /// A resource used to exist in storage but has been deleted by the current transaction.
    Deleted,
}

/// Represents a "slot" in global storage that can hold a resource. The resource is always a struct
/// or an enum.
///
/// IMPORTANT: [Clone] is not implemented for a reason, use [Copyable] trait for deep copies.
#[derive(Debug)]
pub struct GlobalValue(GlobalValueImpl);

/// Trait to represent copyable values so that [GlobalValue] can support copy-on-write. Note that
/// we explicitly do not implement [Clone].
pub trait Copyable: Sized {
    fn deep_copy(&self) -> PartialVMResult<Self>;
}

impl<T> Copyable for T
where
    T: Clone,
{
    fn deep_copy(&self) -> PartialVMResult<Self> {
        Ok(self.clone())
    }
}

impl Copyable for GlobalValue {
    fn deep_copy(&self) -> PartialVMResult<Self> {
        Ok(Self(match &self.0 {
            GlobalValueImpl::None => GlobalValueImpl::None,
            GlobalValueImpl::Fresh { value } => {
                let value = value.copy_value(1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))?;
                GlobalValueImpl::Fresh { value }
            },
            GlobalValueImpl::Cached { value, status } => {
                let value = value.copy_value(1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))?;
                let status = Rc::new(RefCell::new(*status.borrow()));
                GlobalValueImpl::Cached { value, status }
            },
            GlobalValueImpl::Deleted => GlobalValueImpl::Deleted,
        }))
    }
}

/// The locals for a function frame. It allows values to be read, written or taken
/// reference from.
#[derive(Debug)]
pub struct Locals(Rc<RefCell<Vec<Value>>>);

/***************************************************************************************
 *
 * Misc
 *
 *   Miscellaneous helper functions.
 *
 **************************************************************************************/

/// Value's kind dictates the rules how values can be referenced or stored in containers. For
/// example, primitive values like u8 cannot be stored in a generic [Container::Vec] and need to
/// be stored in specialized variant ([Container::VecU8]).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ValueKind {
    /// All primitive types which have a specialized vector container implementation.
    SpecializedVecPrimitive,
    /// All primitive types which do not have a specialized vector container implementation.
    NonSpecializedVecPrimitive,
    /// A container (struct, vector, locals).
    Container,
    /// Anything else, such as invalid local values or references.
    RefOrInvalid,
}

impl Value {
    /// Returns value's kind. This method must be kept in sync with checks below which return an
    /// error if value's kind is not valid for a specific use case.
    fn kind(&self) -> ValueKind {
        use Value::*;
        match self {
            U8(_) | U16(_) | U32(_) | U64(_) | U128(_) | U256(_) | I8(_) | I16(_) | I32(_)
            | I64(_) | I128(_) | I256(_) | Bool(_) | Address(_) => {
                ValueKind::SpecializedVecPrimitive
            },
            DelayedFieldID { .. } | ClosureValue(_) => ValueKind::NonSpecializedVecPrimitive,
            Container(_) => ValueKind::Container,
            ContainerRef(_) | IndexedRef(_) | Invalid => ValueKind::RefOrInvalid,
        }
    }

    /// Returns an error if value's kind is not valid for [Container::Vec].
    fn check_valid_for_value_vector(&self) -> PartialVMResult<()> {
        use ValueKind as K;

        match self.kind() {
            K::NonSpecializedVecPrimitive | K::Container => Ok(()),
            K::SpecializedVecPrimitive | K::RefOrInvalid => {
                Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("vector of `Value`s cannot contain {:?}", self)))
            },
        }
    }

    /// [IndexedRef] can only point to primitive types of a container. For non-specialized vectors,
    /// indexed ref cannot point to primitive types like u8 that have their specialized versions.
    fn check_valid_for_indexed_ref(&self, indexed_ref: &IndexedRef) -> PartialVMResult<()> {
        use ValueKind as K;

        let container = indexed_ref.container_ref.container();
        let is_ok = match self.kind() {
            K::NonSpecializedVecPrimitive => true,
            K::SpecializedVecPrimitive => !matches!(container, Container::Vec(_)),
            K::Container | K::RefOrInvalid => false,
        };
        if !is_ok {
            let msg = format!(
                "invalid IndexedRef element {:?} for container {:?}",
                self, container
            );
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(msg),
            );
        }
        Ok(())
    }
}

impl Container {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn len(&self) -> usize {
        match self {
            Self::Vec(r) => r.borrow().len(),
            Self::Struct(r) => r.borrow().len(),

            Self::VecU8(r) => r.borrow().len(),
            Self::VecU16(r) => r.borrow().len(),
            Self::VecU32(r) => r.borrow().len(),
            Self::VecU64(r) => r.borrow().len(),
            Self::VecU128(r) => r.borrow().len(),
            Self::VecU256(r) => r.borrow().len(),
            Self::VecI8(r) => r.borrow().len(),
            Self::VecI16(r) => r.borrow().len(),
            Self::VecI32(r) => r.borrow().len(),
            Self::VecI64(r) => r.borrow().len(),
            Self::VecI128(r) => r.borrow().len(),
            Self::VecI256(r) => r.borrow().len(),
            Self::VecBool(r) => r.borrow().len(),
            Self::VecAddress(r) => r.borrow().len(),

            Self::Locals(r) => r.borrow().len(),
        }
    }

    fn master_signer(x: AccountAddress) -> Self {
        Container::Struct(Rc::new(RefCell::new(vec![
            Value::U16(MASTER_SIGNER_VARIANT),
            Value::Address(x),
        ])))
    }
}

/***************************************************************************************
 *
 * Borrows (Internal)
 *
 *   Helper functions to handle Rust borrows. When borrowing from a RefCell, we want
 *   to return an error instead of panicking.
 *
 **************************************************************************************/

#[cfg_attr(feature = "force-inline", inline(always))]
fn take_unique_ownership<T: Debug>(r: Rc<RefCell<T>>) -> PartialVMResult<T> {
    match Rc::try_unwrap(r) {
        Ok(cell) => Ok(cell.into_inner()),
        Err(r) => Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(format!("moving value {:?} with dangling references", r))
                .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EREFERENCE_COUNTING_FAILURE),
        ),
    }
}

impl ContainerRef {
    fn container(&self) -> &Container {
        match self {
            Self::Local(container) | Self::Global { container, .. } => container,
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn mark_dirty(&self) {
        if let Self::Global { status, .. } = self {
            *status.borrow_mut() = GlobalDataStatus::Dirty
        }
    }
}

/***************************************************************************************
 *
 * Reference Conversions (Internal)
 *
 *   Helpers to obtain a Rust reference to a value via a VM reference. Required for
 *   equalities.
 *
 **************************************************************************************/
trait VMValueRef<T> {
    fn value_ref(&self) -> PartialVMResult<&T>;
}

macro_rules! impl_vm_value_ref {
    ($ty:ty, $tc:ident) => {
        impl VMValueRef<$ty> for Value {
            #[cfg_attr(feature = "inline-vm-casts", inline)]
            fn value_ref(&self) -> PartialVMResult<&$ty> {
                return match self {
                    Value::$tc(x) => Ok(x),
                    _ => __cannot_ref_cast(self),
                };
                #[cold]
                fn __cannot_ref_cast(v: &Value) -> PartialVMResult<&$ty> {
                    Err(
                        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                            "cannot take {:?} as &{}",
                            v,
                            stringify!($ty)
                        )),
                    )
                }
            }
        }
    };
}

impl_vm_value_ref!(u8, U8);
impl_vm_value_ref!(u16, U16);
impl_vm_value_ref!(u32, U32);
impl_vm_value_ref!(u64, U64);
impl_vm_value_ref!(u128, U128);
impl_vm_value_ref!(int256::U256, U256);
impl_vm_value_ref!(i8, I8);
impl_vm_value_ref!(i16, I16);
impl_vm_value_ref!(i32, I32);
impl_vm_value_ref!(i64, I64);
impl_vm_value_ref!(i128, I128);
impl_vm_value_ref!(int256::I256, I256);
impl_vm_value_ref!(bool, Bool);
impl_vm_value_ref!(AccountAddress, Address);

impl Value {
    fn as_value_ref<T>(&self) -> PartialVMResult<&T>
    where
        Self: VMValueRef<T>,
    {
        VMValueRef::value_ref(self)
    }
}

/***************************************************************************************
 *
 * Copy Value
 *
 *   Implementation of Move copy. Extra care needs to be taken when copying references.
 *   It is intentional we avoid implementing the standard library trait Clone, to prevent
 *   surprising behaviors from happening.
 *
 **************************************************************************************/
impl Value {
    // Note(inline): recursive function, but `#[cfg_attr(feature = "force-inline", inline(always))]` seems to improve perf slightly
    //               and doesn't add much compile time.
    #[inline(always)]
    fn copy_value(&self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<Self> {
        use Value::*;

        check_depth(depth, max_depth)?;
        Ok(match self {
            Invalid => Invalid,

            U8(x) => U8(*x),
            U16(x) => U16(*x),
            U32(x) => U32(*x),
            U64(x) => U64(*x),
            U128(x) => U128(*x),
            U256(x) => U256(*x),
            I8(x) => I8(*x),
            I16(x) => I16(*x),
            I32(x) => I32(*x),
            I64(x) => I64(*x),
            I128(x) => I128(*x),
            I256(x) => I256(*x),
            Bool(x) => Bool(*x),
            Address(x) => Address(*x),

            // Note: refs copy only clones Rc, so no need to increment depth.
            ContainerRef(r) => ContainerRef(r.copy_by_ref()),
            IndexedRef(r) => IndexedRef(r.copy_by_ref()),

            // When cloning a container, we need to make sure we make a deep copy of the data
            // instead of a shallow copy of the Rc. Note that we do not increment the depth here
            // because we have done it when entering this value. Inside the container, depth will
            // be further incremented for nested values.
            Container(c) => Container(c.copy_value(depth, max_depth)?),

            // Native values can be copied because this is how read_ref operates,
            // and copying is an internal API.
            DelayedFieldID { id } => DelayedFieldID { id: *id },

            ClosureValue(Closure(fun, captured)) => {
                let captured = captured
                    .iter()
                    .map(|v| v.copy_value(depth + 1, max_depth))
                    .collect::<PartialVMResult<_>>()?;
                ClosureValue(Closure(fun.clone_dyn()?, captured))
            },
        })
    }
}

impl Container {
    fn copy_value(&self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<Self> {
        let copy_rc_ref_vec_val = |r: &Rc<RefCell<Vec<Value>>>| {
            Ok(Rc::new(RefCell::new(
                r.borrow()
                    .iter()
                    .map(|v| v.copy_value(depth + 1, max_depth))
                    .collect::<PartialVMResult<_>>()?,
            )))
        };

        Ok(match self {
            Self::Vec(r) => Self::Vec(copy_rc_ref_vec_val(r)?),
            Self::Struct(r) => Self::Struct(copy_rc_ref_vec_val(r)?),

            Self::VecU8(r) => Self::VecU8(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecU16(r) => Self::VecU16(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecU32(r) => Self::VecU32(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecU64(r) => Self::VecU64(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecU128(r) => Self::VecU128(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecU256(r) => Self::VecU256(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI8(r) => Self::VecI8(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI16(r) => Self::VecI16(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI32(r) => Self::VecI32(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI64(r) => Self::VecI64(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI128(r) => Self::VecI128(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecI256(r) => Self::VecI256(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecBool(r) => Self::VecBool(Rc::new(RefCell::new(r.borrow().clone()))),
            Self::VecAddress(r) => Self::VecAddress(Rc::new(RefCell::new(r.borrow().clone()))),

            Self::Locals(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("cannot copy a Locals container".to_string()),
                )
            },
        })
    }

    // Note(inline): expensive to inline, +10s compile time
    fn copy_by_ref(&self) -> Self {
        match self {
            Self::Vec(r) => Self::Vec(Rc::clone(r)),
            Self::Struct(r) => Self::Struct(Rc::clone(r)),

            Self::VecU8(r) => Self::VecU8(Rc::clone(r)),
            Self::VecU16(r) => Self::VecU16(Rc::clone(r)),
            Self::VecU32(r) => Self::VecU32(Rc::clone(r)),
            Self::VecU64(r) => Self::VecU64(Rc::clone(r)),
            Self::VecU128(r) => Self::VecU128(Rc::clone(r)),
            Self::VecU256(r) => Self::VecU256(Rc::clone(r)),
            Self::VecI8(r) => Self::VecI8(Rc::clone(r)),
            Self::VecI16(r) => Self::VecI16(Rc::clone(r)),
            Self::VecI32(r) => Self::VecI32(Rc::clone(r)),
            Self::VecI64(r) => Self::VecI64(Rc::clone(r)),
            Self::VecI128(r) => Self::VecI128(Rc::clone(r)),
            Self::VecI256(r) => Self::VecI256(Rc::clone(r)),
            Self::VecBool(r) => Self::VecBool(Rc::clone(r)),
            Self::VecAddress(r) => Self::VecAddress(Rc::clone(r)),

            Self::Locals(r) => Self::Locals(Rc::clone(r)),
        }
    }
}

impl IndexedRef {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn copy_by_ref(&self) -> Self {
        Self {
            idx: self.idx,
            container_ref: self.container_ref.copy_by_ref(),
        }
    }
}

impl ContainerRef {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn copy_by_ref(&self) -> Self {
        match self {
            Self::Local(container) => Self::Local(container.copy_by_ref()),
            Self::Global { status, container } => Self::Global {
                status: Rc::clone(status),
                container: container.copy_by_ref(),
            },
        }
    }
}

#[cfg(test)]
impl Value {
    pub fn copy_value_with_depth(&self, max_depth: u64) -> PartialVMResult<Self> {
        self.copy_value(1, Some(max_depth))
    }
}

/***************************************************************************************
 *
 * Equality
 *
 *   Equality tests of Move values. Errors are raised when types mismatch.
 *
 *   It is intended to NOT use or even implement the standard library traits Eq and
 *   Partial Eq due to:
 *     1. They do not allow errors to be returned.
 *     2. They can be invoked without the user being noticed thanks to operator
 *        overloading.
 *
 *   Eq and Partial Eq must also NOT be derived for the reasons above plus that the
 *   derived implementation differs from the semantics we want.
 *
 **************************************************************************************/

impl Value {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn equals(&self, other: &Self) -> PartialVMResult<bool> {
        self.equals_with_depth(other, 1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn compare(&self, other: &Self) -> PartialVMResult<Ordering> {
        self.compare_with_depth(other, 1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn equals_with_depth(
        &self,
        other: &Self,
        depth: u64,
        max_depth: Option<u64>,
    ) -> PartialVMResult<bool> {
        use Value::*;

        check_depth(depth, max_depth)?;
        let res = match (self, other) {
            (U8(l), U8(r)) => l == r,
            (U16(l), U16(r)) => l == r,
            (U32(l), U32(r)) => l == r,
            (U64(l), U64(r)) => l == r,
            (U128(l), U128(r)) => l == r,
            (U256(l), U256(r)) => l == r,
            (I8(l), I8(r)) => l == r,
            (I16(l), I16(r)) => l == r,
            (I32(l), I32(r)) => l == r,
            (I64(l), I64(r)) => l == r,
            (I128(l), I128(r)) => l == r,
            (I256(l), I256(r)) => l == r,
            (Bool(l), Bool(r)) => l == r,
            (Address(l), Address(r)) => l == r,

            (Container(l), Container(r)) => l.equals(r, depth, max_depth)?,

            // We count references as +1 in nesting, hence increasing the depth.
            (ContainerRef(l), ContainerRef(r)) => l.equals(r, depth + 1, max_depth)?,
            (IndexedRef(l), IndexedRef(r)) => l.equals(r, depth + 1, max_depth)?,

            // Disallow equality for delayed values. The rationale behind this
            // semantics is that identifiers might not be deterministic, and
            // therefore equality can have different outcomes on different nodes
            // of the network. Note that the error returned here is not an
            // invariant violation but a runtime error.
            (DelayedFieldID { .. }, DelayedFieldID { .. }) => {
                return Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                    .with_message("cannot compare delayed values".to_string()))
            },

            (ClosureValue(Closure(fun1, captured1)), ClosureValue(Closure(fun2, captured2))) => {
                if fun1.cmp_dyn(fun2.as_ref())? == Ordering::Equal
                    && captured1.len() == captured2.len()
                {
                    for (v1, v2) in captured1.iter().zip(captured2.iter()) {
                        if !v1.equals_with_depth(v2, depth + 1, max_depth)? {
                            return Ok(false);
                        }
                    }
                    true
                } else {
                    false
                }
            },

            (Invalid, _)
            | (U8(_), _)
            | (U16(_), _)
            | (U32(_), _)
            | (U64(_), _)
            | (U128(_), _)
            | (U256(_), _)
            | (I8(_), _)
            | (I16(_), _)
            | (I32(_), _)
            | (I64(_), _)
            | (I128(_), _)
            | (I256(_), _)
            | (Bool(_), _)
            | (Address(_), _)
            | (Container(_), _)
            | (ContainerRef(_), _)
            | (IndexedRef(_), _)
            | (ClosureValue(_), _)
            | (DelayedFieldID { .. }, _) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "inconsistent argument types passed to equals check: {:?}, {:?}",
                        self, other
                    )),
                )
            },
        };

        Ok(res)
    }

    pub fn compare_with_depth(
        &self,
        other: &Self,
        depth: u64,
        max_depth: Option<u64>,
    ) -> PartialVMResult<Ordering> {
        use Value::*;

        check_depth(depth, max_depth)?;
        let res = match (self, other) {
            (U8(l), U8(r)) => l.cmp(r),
            (U16(l), U16(r)) => l.cmp(r),
            (U32(l), U32(r)) => l.cmp(r),
            (U64(l), U64(r)) => l.cmp(r),
            (U128(l), U128(r)) => l.cmp(r),
            (U256(l), U256(r)) => l.cmp(r),
            (I8(l), I8(r)) => l.cmp(r),
            (I16(l), I16(r)) => l.cmp(r),
            (I32(l), I32(r)) => l.cmp(r),
            (I64(l), I64(r)) => l.cmp(r),
            (I128(l), I128(r)) => l.cmp(r),
            (I256(l), I256(r)) => l.cmp(r),
            (Bool(l), Bool(r)) => l.cmp(r),
            (Address(l), Address(r)) => l.cmp(r),

            (Container(l), Container(r)) => l.compare(r, depth, max_depth)?,

            // We count references as +1 in nesting, hence increasing the depth.
            (ContainerRef(l), ContainerRef(r)) => l.compare(r, depth + 1, max_depth)?,
            (IndexedRef(l), IndexedRef(r)) => l.compare(r, depth + 1, max_depth)?,

            // Disallow comparison for delayed values.
            // (see `ValueImpl::equals` above for details on reasoning behind it)
            (DelayedFieldID { .. }, DelayedFieldID { .. }) => {
                return Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                    .with_message("cannot compare delayed values".to_string()))
            },

            (ClosureValue(Closure(fun1, captured1)), ClosureValue(Closure(fun2, captured2))) => {
                let o = fun1.cmp_dyn(fun2.as_ref())?;
                if o == Ordering::Equal {
                    for (v1, v2) in captured1.iter().zip(captured2.iter()) {
                        let o = v1.compare_with_depth(v2, depth + 1, max_depth)?;
                        if o != Ordering::Equal {
                            return Ok(o);
                        }
                    }
                    captured1.iter().len().cmp(&captured2.len())
                } else {
                    o
                }
            },

            (Invalid, _)
            | (U8(_), _)
            | (U16(_), _)
            | (U32(_), _)
            | (U64(_), _)
            | (U128(_), _)
            | (U256(_), _)
            | (I8(_), _)
            | (I16(_), _)
            | (I32(_), _)
            | (I64(_), _)
            | (I128(_), _)
            | (I256(_), _)
            | (Bool(_), _)
            | (Address(_), _)
            | (Container(_), _)
            | (ContainerRef(_), _)
            | (IndexedRef(_), _)
            | (ClosureValue(_), _)
            | (DelayedFieldID { .. }, _) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "inconsistent argument types passed to comparison: {:?}, {:?}",
                        self, other
                    )),
                )
            },
        };

        Ok(res)
    }

    // Test-only API to test depth checks.
    #[cfg(test)]
    pub fn equals_with_depth_for_test(
        &self,
        other: &Self,
        max_depth: u64,
    ) -> PartialVMResult<bool> {
        self.equals_with_depth(other, 1, Some(max_depth))
    }

    // Test-only API to test depth checks.
    #[cfg(test)]
    pub fn compare_with_depth_for_test(
        &self,
        other: &Self,
        max_depth: u64,
    ) -> PartialVMResult<Ordering> {
        self.compare_with_depth(other, 1, Some(max_depth))
    }
}

impl Container {
    fn equals(&self, other: &Self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<bool> {
        use Container::*;

        let res = match (self, other) {
            (Vec(l), Vec(r)) | (Struct(l), Struct(r)) => {
                let l = &l.borrow();
                let r = &r.borrow();

                if l.len() != r.len() {
                    return Ok(false);
                }
                for (v1, v2) in l.iter().zip(r.iter()) {
                    if !v1.equals_with_depth(v2, depth + 1, max_depth)? {
                        return Ok(false);
                    }
                }
                true
            },
            (VecU8(l), VecU8(r)) => l.borrow().eq(&*r.borrow()),
            (VecU16(l), VecU16(r)) => l.borrow().eq(&*r.borrow()),
            (VecU32(l), VecU32(r)) => l.borrow().eq(&*r.borrow()),
            (VecU64(l), VecU64(r)) => l.borrow().eq(&*r.borrow()),
            (VecU128(l), VecU128(r)) => l.borrow().eq(&*r.borrow()),
            (VecU256(l), VecU256(r)) => l.borrow().eq(&*r.borrow()),
            (VecI8(l), VecI8(r)) => l.borrow().eq(&*r.borrow()),
            (VecI16(l), VecI16(r)) => l.borrow().eq(&*r.borrow()),
            (VecI32(l), VecI32(r)) => l.borrow().eq(&*r.borrow()),
            (VecI64(l), VecI64(r)) => l.borrow().eq(&*r.borrow()),
            (VecI128(l), VecI128(r)) => l.borrow().eq(&*r.borrow()),
            (VecI256(l), VecI256(r)) => l.borrow().eq(&*r.borrow()),
            (VecBool(l), VecBool(r)) => l.borrow().eq(&*r.borrow()),
            (VecAddress(l), VecAddress(r)) => l.borrow().eq(&*r.borrow()),

            (Locals(_), _)
            | (Vec(_), _)
            | (Struct(_), _)
            | (VecU8(_), _)
            | (VecU16(_), _)
            | (VecU32(_), _)
            | (VecU64(_), _)
            | (VecU128(_), _)
            | (VecU256(_), _)
            | (VecI8(_), _)
            | (VecI16(_), _)
            | (VecI32(_), _)
            | (VecI64(_), _)
            | (VecI128(_), _)
            | (VecI256(_), _)
            | (VecBool(_), _)
            | (VecAddress(_), _) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "cannot compare container values: {:?}, {:?}",
                        self, other
                    )),
                )
            },
        };

        Ok(res)
    }

    fn compare(
        &self,
        other: &Self,
        depth: u64,
        max_depth: Option<u64>,
    ) -> PartialVMResult<Ordering> {
        use Container::*;

        let res = match (self, other) {
            (Vec(l), Vec(r)) | (Struct(l), Struct(r)) => {
                let l = &l.borrow();
                let r = &r.borrow();

                for (v1, v2) in l.iter().zip(r.iter()) {
                    let value_cmp = v1.compare_with_depth(v2, depth + 1, max_depth)?;
                    if value_cmp.is_ne() {
                        return Ok(value_cmp);
                    }
                }

                l.len().cmp(&r.len())
            },
            (VecU8(l), VecU8(r)) => l.borrow().cmp(&*r.borrow()),
            (VecU16(l), VecU16(r)) => l.borrow().cmp(&*r.borrow()),
            (VecU32(l), VecU32(r)) => l.borrow().cmp(&*r.borrow()),
            (VecU64(l), VecU64(r)) => l.borrow().cmp(&*r.borrow()),
            (VecU128(l), VecU128(r)) => l.borrow().cmp(&*r.borrow()),
            (VecU256(l), VecU256(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI8(l), VecI8(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI16(l), VecI16(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI32(l), VecI32(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI64(l), VecI64(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI128(l), VecI128(r)) => l.borrow().cmp(&*r.borrow()),
            (VecI256(l), VecI256(r)) => l.borrow().cmp(&*r.borrow()),
            (VecBool(l), VecBool(r)) => l.borrow().cmp(&*r.borrow()),
            (VecAddress(l), VecAddress(r)) => l.borrow().cmp(&*r.borrow()),

            (Locals(_), _)
            | (Vec(_), _)
            | (Struct(_), _)
            | (VecU8(_), _)
            | (VecU16(_), _)
            | (VecU32(_), _)
            | (VecU64(_), _)
            | (VecU128(_), _)
            | (VecU256(_), _)
            | (VecI8(_), _)
            | (VecI16(_), _)
            | (VecI32(_), _)
            | (VecI64(_), _)
            | (VecI128(_), _)
            | (VecI256(_), _)
            | (VecBool(_), _)
            | (VecAddress(_), _) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "cannot compare container values: {:?}, {:?}",
                        self, other
                    )),
                )
            },
        };

        Ok(res)
    }
}

impl ContainerRef {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn equals(&self, other: &Self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<bool> {
        // Note: the depth passed in accounts for the container.
        check_depth(depth, max_depth)?;
        self.container().equals(other.container(), depth, max_depth)
    }

    fn compare(
        &self,
        other: &Self,
        depth: u64,
        max_depth: Option<u64>,
    ) -> PartialVMResult<Ordering> {
        // Note: the depth passed in accounts for the container.
        check_depth(depth, max_depth)?;
        self.container()
            .compare(other.container(), depth, max_depth)
    }
}

impl IndexedRef {
    // note(inline): do not inline, too big
    fn equals(&self, other: &Self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<bool> {
        use Container::*;

        check_depth(depth, max_depth)?;
        let res = match (
            self.container_ref.container(),
            other.container_ref.container(),
        ) {
            // VecC <=> VecR impossible
            (Vec(r1), Vec(r2))
            | (Vec(r1), Struct(r2))
            | (Vec(r1), Locals(r2))
            | (Struct(r1), Vec(r2))
            | (Struct(r1), Struct(r2))
            | (Struct(r1), Locals(r2))
            | (Locals(r1), Vec(r2))
            | (Locals(r1), Struct(r2))
            | (Locals(r1), Locals(r2)) => r1.borrow()[self.idx].equals_with_depth(
                &r2.borrow()[other.idx],
                depth + 1,
                max_depth,
            )?,

            (VecU8(r1), VecU8(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecU16(r1), VecU16(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecU32(r1), VecU32(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecU64(r1), VecU64(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecU128(r1), VecU128(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecU256(r1), VecU256(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI8(r1), VecI8(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI16(r1), VecI16(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI32(r1), VecI32(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI64(r1), VecI64(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI128(r1), VecI128(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecI256(r1), VecI256(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecBool(r1), VecBool(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],
            (VecAddress(r1), VecAddress(r2)) => r1.borrow()[self.idx] == r2.borrow()[other.idx],

            // Equality between a generic and a specialized container.
            (Locals(r1), VecU8(r2)) | (Struct(r1), VecU8(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<u8>()? == r2.borrow()[other.idx]
            },
            (VecU8(r1), Locals(r2)) | (VecU8(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<u8>()?
            },

            (Locals(r1), VecU16(r2)) | (Struct(r1), VecU16(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<u16>()? == r2.borrow()[other.idx]
            },
            (VecU16(r1), Locals(r2)) | (VecU16(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<u16>()?
            },

            (Locals(r1), VecU32(r2)) | (Struct(r1), VecU32(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<u32>()? == r2.borrow()[other.idx]
            },
            (VecU32(r1), Locals(r2)) | (VecU32(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<u32>()?
            },

            (Locals(r1), VecU64(r2)) | (Struct(r1), VecU64(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<u64>()? == r2.borrow()[other.idx]
            },
            (VecU64(r1), Locals(r2)) | (VecU64(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<u64>()?
            },

            (Locals(r1), VecU128(r2)) | (Struct(r1), VecU128(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<u128>()? == r2.borrow()[other.idx]
            },
            (VecU128(r1), Locals(r2)) | (VecU128(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<u128>()?
            },

            (Locals(r1), VecU256(r2)) | (Struct(r1), VecU256(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<int256::U256>()? == r2.borrow()[other.idx]
            },
            (VecU256(r1), Locals(r2)) | (VecU256(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<int256::U256>()?
            },

            (Locals(r1), VecI8(r2)) | (Struct(r1), VecI8(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<i8>()? == r2.borrow()[other.idx]
            },
            (VecI8(r1), Locals(r2)) | (VecI8(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<i8>()?
            },

            (Locals(r1), VecI16(r2)) | (Struct(r1), VecI16(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<i16>()? == r2.borrow()[other.idx]
            },
            (VecI16(r1), Locals(r2)) | (VecI16(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<i16>()?
            },

            (Locals(r1), VecI32(r2)) | (Struct(r1), VecI32(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<i32>()? == r2.borrow()[other.idx]
            },
            (VecI32(r1), Locals(r2)) | (VecI32(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<i32>()?
            },

            (Locals(r1), VecI64(r2)) | (Struct(r1), VecI64(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<i64>()? == r2.borrow()[other.idx]
            },
            (VecI64(r1), Locals(r2)) | (VecI64(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<i64>()?
            },

            (Locals(r1), VecI128(r2)) | (Struct(r1), VecI128(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<i128>()? == r2.borrow()[other.idx]
            },
            (VecI128(r1), Locals(r2)) | (VecI128(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<i128>()?
            },

            (Locals(r1), VecI256(r2)) | (Struct(r1), VecI256(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<int256::I256>()? == r2.borrow()[other.idx]
            },
            (VecI256(r1), Locals(r2)) | (VecI256(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<int256::I256>()?
            },

            (Locals(r1), VecBool(r2)) | (Struct(r1), VecBool(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<bool>()? == r2.borrow()[other.idx]
            },
            (VecBool(r1), Locals(r2)) | (VecBool(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<bool>()?
            },

            (Locals(r1), VecAddress(r2)) | (Struct(r1), VecAddress(r2)) => {
                *r1.borrow()[self.idx].as_value_ref::<AccountAddress>()? == r2.borrow()[other.idx]
            },
            (VecAddress(r1), Locals(r2)) | (VecAddress(r1), Struct(r2)) => {
                r1.borrow()[self.idx] == *r2.borrow()[other.idx].as_value_ref::<AccountAddress>()?
            },

            // All other combinations are illegal.
            (Vec(_), _)
            | (VecU8(_), _)
            | (VecU16(_), _)
            | (VecU32(_), _)
            | (VecU64(_), _)
            | (VecU128(_), _)
            | (VecU256(_), _)
            | (VecI8(_), _)
            | (VecI16(_), _)
            | (VecI32(_), _)
            | (VecI64(_), _)
            | (VecI128(_), _)
            | (VecI256(_), _)
            | (VecBool(_), _)
            | (VecAddress(_), _) => {
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("cannot compare references {:?}, {:?}", self, other)))
            },
        };
        Ok(res)
    }

    fn compare(
        &self,
        other: &Self,
        depth: u64,
        max_depth: Option<u64>,
    ) -> PartialVMResult<Ordering> {
        use Container::*;

        let res = match (
            self.container_ref.container(),
            other.container_ref.container(),
        ) {
            // VecC <=> VecR impossible
            (Vec(r1), Vec(r2))
            | (Vec(r1), Struct(r2))
            | (Vec(r1), Locals(r2))
            | (Struct(r1), Vec(r2))
            | (Struct(r1), Struct(r2))
            | (Struct(r1), Locals(r2))
            | (Locals(r1), Vec(r2))
            | (Locals(r1), Struct(r2))
            | (Locals(r1), Locals(r2)) => r1.borrow()[self.idx].compare_with_depth(
                &r2.borrow()[other.idx],
                depth + 1,
                max_depth,
            )?,

            (VecU8(r1), VecU8(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecU16(r1), VecU16(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecU32(r1), VecU32(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecU64(r1), VecU64(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecU128(r1), VecU128(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecU256(r1), VecU256(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI8(r1), VecI8(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI16(r1), VecI16(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI32(r1), VecI32(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI64(r1), VecI64(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI128(r1), VecI128(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecI256(r1), VecI256(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecBool(r1), VecBool(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),
            (VecAddress(r1), VecAddress(r2)) => r1.borrow()[self.idx].cmp(&r2.borrow()[other.idx]),

            // Comparison between a generic and a specialized container.
            (Locals(r1), VecU8(r2)) | (Struct(r1), VecU8(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<u8>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU8(r1), Locals(r2)) | (VecU8(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<u8>()?)
            },

            (Locals(r1), VecU16(r2)) | (Struct(r1), VecU16(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<u16>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU16(r1), Locals(r2)) | (VecU16(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<u16>()?)
            },

            (Locals(r1), VecU32(r2)) | (Struct(r1), VecU32(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<u32>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU32(r1), Locals(r2)) | (VecU32(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<u32>()?)
            },

            (Locals(r1), VecU64(r2)) | (Struct(r1), VecU64(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<u64>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU64(r1), Locals(r2)) | (VecU64(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<u64>()?)
            },

            (Locals(r1), VecU128(r2)) | (Struct(r1), VecU128(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<u128>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU128(r1), Locals(r2)) | (VecU128(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<u128>()?)
            },

            (Locals(r1), VecU256(r2)) | (Struct(r1), VecU256(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<int256::U256>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecU256(r1), Locals(r2)) | (VecU256(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<int256::U256>()?)
            },

            (Locals(r1), VecI8(r2)) | (Struct(r1), VecI8(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<i8>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI8(r1), Locals(r2)) | (VecI8(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<i8>()?)
            },

            (Locals(r1), VecI16(r2)) | (Struct(r1), VecI16(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<i16>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI16(r1), Locals(r2)) | (VecI16(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<i16>()?)
            },

            (Locals(r1), VecI32(r2)) | (Struct(r1), VecI32(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<i32>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI32(r1), Locals(r2)) | (VecI32(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<i32>()?)
            },

            (Locals(r1), VecI64(r2)) | (Struct(r1), VecI64(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<i64>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI64(r1), Locals(r2)) | (VecI64(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<i64>()?)
            },

            (Locals(r1), VecI128(r2)) | (Struct(r1), VecI128(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<i128>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI128(r1), Locals(r2)) | (VecI128(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<i128>()?)
            },

            (Locals(r1), VecI256(r2)) | (Struct(r1), VecI256(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<int256::I256>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecI256(r1), Locals(r2)) | (VecI256(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<int256::I256>()?)
            },

            (Locals(r1), VecBool(r2)) | (Struct(r1), VecBool(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<bool>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecBool(r1), Locals(r2)) | (VecBool(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<bool>()?)
            },

            (Locals(r1), VecAddress(r2)) | (Struct(r1), VecAddress(r2)) => r1.borrow()[self.idx]
                .as_value_ref::<AccountAddress>()?
                .cmp(&r2.borrow()[other.idx]),
            (VecAddress(r1), Locals(r2)) | (VecAddress(r1), Struct(r2)) => {
                r1.borrow()[self.idx].cmp(r2.borrow()[other.idx].as_value_ref::<AccountAddress>()?)
            },

            // All other combinations are illegal.
            (Vec(_), _)
            | (VecU8(_), _)
            | (VecU16(_), _)
            | (VecU32(_), _)
            | (VecU64(_), _)
            | (VecU128(_), _)
            | (VecU256(_), _)
            | (VecI8(_), _)
            | (VecI16(_), _)
            | (VecI32(_), _)
            | (VecI64(_), _)
            | (VecI128(_), _)
            | (VecI256(_), _)
            | (VecBool(_), _)
            | (VecAddress(_), _) => {
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("cannot compare references {:?}, {:?}", self, other)))
            },
        };
        Ok(res)
    }
}

/***************************************************************************************
 *
 * Read Ref
 *
 *   Implementation of the Move operation read ref.
 *
 **************************************************************************************/

impl ContainerRef {
    fn read_ref(self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<Value> {
        Ok(Value::Container(
            self.container().copy_value(depth, max_depth)?,
        ))
    }
}

impl IndexedRef {
    fn read_ref(self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<Value> {
        use Container::*;

        let res = match self.container_ref.container() {
            Vec(r) => r.borrow()[self.idx].copy_value(depth + 1, max_depth)?,
            Struct(r) => r.borrow()[self.idx].copy_value(depth + 1, max_depth)?,

            VecU8(r) => Value::U8(r.borrow()[self.idx]),
            VecU16(r) => Value::U16(r.borrow()[self.idx]),
            VecU32(r) => Value::U32(r.borrow()[self.idx]),
            VecU64(r) => Value::U64(r.borrow()[self.idx]),
            VecU128(r) => Value::U128(r.borrow()[self.idx]),
            VecU256(r) => Value::U256(r.borrow()[self.idx]),
            VecI8(r) => Value::I8(r.borrow()[self.idx]),
            VecI16(r) => Value::I16(r.borrow()[self.idx]),
            VecI32(r) => Value::I32(r.borrow()[self.idx]),
            VecI64(r) => Value::I64(r.borrow()[self.idx]),
            VecI128(r) => Value::I128(r.borrow()[self.idx]),
            VecI256(r) => Value::I256(r.borrow()[self.idx]),
            VecBool(r) => Value::Bool(r.borrow()[self.idx]),
            VecAddress(r) => Value::Address(r.borrow()[self.idx]),

            Locals(r) => r.borrow()[self.idx].copy_value(depth + 1, max_depth)?,
        };
        res.check_valid_for_indexed_ref(&self)?;
        Ok(res)
    }
}

impl ReferenceImpl {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn read_ref(self, depth: u64, max_depth: Option<u64>) -> PartialVMResult<Value> {
        match self {
            Self::ContainerRef(r) => r.read_ref(depth, max_depth),
            Self::IndexedRef(r) => r.read_ref(depth, max_depth),
        }
    }
}

impl StructRef {
    pub fn read_ref(self) -> PartialVMResult<Value> {
        self.0.read_ref(1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))
    }

    #[cfg(test)]
    pub fn read_ref_with_depth(self, max_depth: u64) -> PartialVMResult<Value> {
        self.0.read_ref(1, Some(max_depth))
    }
}

impl Reference {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn read_ref(self) -> PartialVMResult<Value> {
        self.0.read_ref(1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))
    }

    #[cfg(test)]
    pub fn read_ref_with_depth(self, max_depth: u64) -> PartialVMResult<Value> {
        self.0.read_ref(1, Some(max_depth))
    }
}

/***************************************************************************************
 *
 * Write Ref
 *
 *   Implementation of the Move operation write ref.
 *
 **************************************************************************************/

impl ContainerRef {
    fn write_ref(self, v: Value) -> PartialVMResult<()> {
        match v {
            Value::Container(c) => {
                macro_rules! assign {
                    ($r1:expr, $tc:ident) => {{
                        let r = match c {
                            Container::$tc(v) => v,
                            _ => {
                                return Err(PartialVMError::new(
                                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                                )
                                .with_message(
                                    "failed to write_ref: container type mismatch".to_string(),
                                )
                                .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE))
                            },
                        };
                        *$r1.borrow_mut() = take_unique_ownership(r)?;
                    }};
                }

                match self.container() {
                    Container::Struct(r) => assign!(r, Struct),
                    Container::Vec(r) => assign!(r, Vec),
                    Container::VecU8(r) => assign!(r, VecU8),
                    Container::VecU16(r) => assign!(r, VecU16),
                    Container::VecU32(r) => assign!(r, VecU32),
                    Container::VecU64(r) => assign!(r, VecU64),
                    Container::VecU128(r) => assign!(r, VecU128),
                    Container::VecU256(r) => assign!(r, VecU256),
                    Container::VecI8(r) => assign!(r, VecI8),
                    Container::VecI16(r) => assign!(r, VecI16),
                    Container::VecI32(r) => assign!(r, VecI32),
                    Container::VecI64(r) => assign!(r, VecI64),
                    Container::VecI128(r) => assign!(r, VecI128),
                    Container::VecI256(r) => assign!(r, VecI256),
                    Container::VecBool(r) => assign!(r, VecBool),
                    Container::VecAddress(r) => assign!(r, VecAddress),
                    Container::Locals(_) => {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message("cannot overwrite Container::Locals".to_string()))
                    },
                }
                self.mark_dirty();
            },
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "cannot write value {:?} to container ref {:?}",
                            v, self
                        )),
                )
            },
        }
        Ok(())
    }
}

impl IndexedRef {
    fn write_ref(self, x: Value) -> PartialVMResult<()> {
        x.check_valid_for_indexed_ref(&self)?;
        match (self.container_ref.container(), &x) {
            (Container::Locals(r), _) | (Container::Vec(r), _) | (Container::Struct(r), _) => {
                let mut v = r.borrow_mut();
                v[self.idx] = x;
            },
            (Container::VecU8(r), Value::U8(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecU16(r), Value::U16(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecU32(r), Value::U32(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecU64(r), Value::U64(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecU128(r), Value::U128(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecU256(r), Value::U256(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI8(r), Value::I8(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI16(r), Value::I16(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI32(r), Value::I32(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI64(r), Value::I64(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI128(r), Value::I128(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecI256(r), Value::I256(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecBool(r), Value::Bool(x)) => r.borrow_mut()[self.idx] = *x,
            (Container::VecAddress(r), Value::Address(x)) => r.borrow_mut()[self.idx] = *x,

            (Container::VecU8(_), _)
            | (Container::VecU16(_), _)
            | (Container::VecU32(_), _)
            | (Container::VecU64(_), _)
            | (Container::VecU128(_), _)
            | (Container::VecU256(_), _)
            | (Container::VecI8(_), _)
            | (Container::VecI16(_), _)
            | (Container::VecI32(_), _)
            | (Container::VecI64(_), _)
            | (Container::VecI128(_), _)
            | (Container::VecI256(_), _)
            | (Container::VecBool(_), _)
            | (Container::VecAddress(_), _) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "cannot write value {:?} to indexed ref {:?}",
                        x, self
                    )),
                )
            },
        }
        self.container_ref.mark_dirty();
        Ok(())
    }
}

impl ReferenceImpl {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn write_ref(self, x: Value) -> PartialVMResult<()> {
        match self {
            Self::ContainerRef(r) => r.write_ref(x),
            Self::IndexedRef(r) => r.write_ref(x),
        }
    }
}

impl Reference {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn write_ref(self, x: Value) -> PartialVMResult<()> {
        self.0.write_ref(x)
    }
}

/**************************************************************************************
 *
 * Helpers: from primitive
 *
 *************************************************************************************/
trait VMValueFromPrimitive<T> {
    fn from_primitive(val: T) -> Self;
}

macro_rules! impl_vm_value_from_primitive {
    ($ty:ty, $tc:ident) => {
        impl VMValueFromPrimitive<$ty> for Value {
            fn from_primitive(val: $ty) -> Self {
                Self::$tc(val)
            }
        }
    };
}

impl_vm_value_from_primitive!(u8, U8);
impl_vm_value_from_primitive!(u16, U16);
impl_vm_value_from_primitive!(u32, U32);
impl_vm_value_from_primitive!(u64, U64);
impl_vm_value_from_primitive!(u128, U128);
impl_vm_value_from_primitive!(int256::U256, U256);
impl_vm_value_from_primitive!(i8, I8);
impl_vm_value_from_primitive!(i16, I16);
impl_vm_value_from_primitive!(i32, I32);
impl_vm_value_from_primitive!(i64, I64);
impl_vm_value_from_primitive!(i128, I128);
impl_vm_value_from_primitive!(int256::I256, I256);
impl_vm_value_from_primitive!(bool, Bool);
impl_vm_value_from_primitive!(AccountAddress, Address);

/**************************************************************************************
 *
 * Swap reference (Move)
 *
 *   Implementation of the Move operation to swap contents of a reference.
 *
 *************************************************************************************/
impl Container {
    /// Swaps contents of two mutable references.
    ///
    /// Precondition for this funciton is that `self` and `other` are required to be
    /// distinct references.
    /// Move will guarantee that invariant, because it prevents from having two
    /// mutable references to the same value.
    fn swap_contents(&self, other: &Self) -> PartialVMResult<()> {
        use Container::*;

        match (self, other) {
            (Vec(l), Vec(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (Struct(l), Struct(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),

            (VecBool(l), VecBool(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecAddress(l), VecAddress(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),

            (VecU8(l), VecU8(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecU16(l), VecU16(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecU32(l), VecU32(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecU64(l), VecU64(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecU128(l), VecU128(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecU256(l), VecU256(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),

            (VecI8(l), VecI8(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecI16(l), VecI16(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecI32(l), VecI32(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecI64(l), VecI64(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecI128(l), VecI128(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),
            (VecI256(l), VecI256(r)) => mem::swap(&mut *l.borrow_mut(), &mut *r.borrow_mut()),

            (
                Locals(_) | Vec(_) | Struct(_) | VecBool(_) | VecAddress(_) | VecU8(_) | VecU16(_)
                | VecU32(_) | VecU64(_) | VecU128(_) | VecU256(_) | VecI8(_) | VecI16(_)
                | VecI32(_) | VecI64(_) | VecI128(_) | VecI256(_),
                _,
            ) => {
                return Err(
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                        "cannot swap container values: {:?}, {:?}",
                        self, other
                    )),
                )
            },
        }

        Ok(())
    }
}

impl ContainerRef {
    fn swap_values(self, other: Self) -> PartialVMResult<()> {
        self.container().swap_contents(other.container())?;

        self.mark_dirty();
        other.mark_dirty();

        Ok(())
    }
}

impl IndexedRef {
    fn swap_values(self, other: Self) -> PartialVMResult<()> {
        use Container::*;

        macro_rules! swap {
            ($r1:ident, $r2:ident) => {{
                if Rc::ptr_eq($r1, $r2) {
                    if self.idx == other.idx {
                        return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                            .with_message(format!(
                                "cannot swap references to the same item {:?}",
                                self
                            )));
                    }

                    $r1.borrow_mut().swap(self.idx, other.idx);
                } else {
                    mem::swap(
                        &mut $r1.borrow_mut()[self.idx],
                        &mut $r2.borrow_mut()[other.idx],
                    )
                }
            }};
        }

        macro_rules! swap_general_with_specialized {
            ($r1:ident, $r2:ident) => {{
                let mut r1 = $r1.borrow_mut();
                let mut r2 = $r2.borrow_mut();

                let v1 = *r1[self.idx].as_value_ref()?;
                r1[self.idx] = Value::from_primitive(r2[other.idx]);
                r2[other.idx] = v1;
            }};
        }

        macro_rules! swap_specialized_with_general {
            ($r1:ident, $r2:ident) => {{
                let mut r1 = $r1.borrow_mut();
                let mut r2 = $r2.borrow_mut();

                let v2 = *r2[other.idx].as_value_ref()?;
                r2[other.idx] = Value::from_primitive(r1[self.idx]);
                r1[self.idx] = v2;
            }};
        }

        match (
            self.container_ref.container(),
            other.container_ref.container(),
        ) {
            // Case 1: (generic, generic)
            (Vec(r1), Vec(r2))
            | (Vec(r1), Struct(r2))
            | (Vec(r1), Locals(r2))
            | (Struct(r1), Vec(r2))
            | (Struct(r1), Struct(r2))
            | (Struct(r1), Locals(r2))
            | (Locals(r1), Vec(r2))
            | (Locals(r1), Struct(r2))
            | (Locals(r1), Locals(r2)) => swap!(r1, r2),

            // Case 2: (specialized, specialized)
            (VecU8(r1), VecU8(r2)) => swap!(r1, r2),
            (VecU16(r1), VecU16(r2)) => swap!(r1, r2),
            (VecU32(r1), VecU32(r2)) => swap!(r1, r2),
            (VecU64(r1), VecU64(r2)) => swap!(r1, r2),
            (VecU128(r1), VecU128(r2)) => swap!(r1, r2),
            (VecU256(r1), VecU256(r2)) => swap!(r1, r2),
            (VecI8(r1), VecI8(r2)) => swap!(r1, r2),
            (VecI16(r1), VecI16(r2)) => swap!(r1, r2),
            (VecI32(r1), VecI32(r2)) => swap!(r1, r2),
            (VecI64(r1), VecI64(r2)) => swap!(r1, r2),
            (VecI128(r1), VecI128(r2)) => swap!(r1, r2),
            (VecI256(r1), VecI256(r2)) => swap!(r1, r2),
            (VecBool(r1), VecBool(r2)) => swap!(r1, r2),
            (VecAddress(r1), VecAddress(r2)) => swap!(r1, r2),

            // Case 3: (generic, specialized) or (specialized, generic)
            (Locals(r1) | Struct(r1), VecU8(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU8(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecU16(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU16(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecU32(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU32(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecU64(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU64(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecU128(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU128(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecU256(r2)) => swap_general_with_specialized!(r1, r2),
            (VecU256(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI8(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI8(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI16(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI16(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI32(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI32(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI64(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI64(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI128(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI128(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecI256(r2)) => swap_general_with_specialized!(r1, r2),
            (VecI256(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecBool(r2)) => swap_general_with_specialized!(r1, r2),
            (VecBool(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            (Locals(r1) | Struct(r1), VecAddress(r2)) => swap_general_with_specialized!(r1, r2),
            (VecAddress(r1), Locals(r2) | Struct(r2)) => swap_specialized_with_general!(r1, r2),

            // All other combinations are illegal.
            (Vec(_), _)
            | (VecU8(_), _)
            | (VecU16(_), _)
            | (VecU32(_), _)
            | (VecU64(_), _)
            | (VecU128(_), _)
            | (VecU256(_), _)
            | (VecI8(_), _)
            | (VecI16(_), _)
            | (VecI32(_), _)
            | (VecI64(_), _)
            | (VecI128(_), _)
            | (VecI256(_), _)
            | (VecBool(_), _)
            | (VecAddress(_), _) => {
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("cannot swap references {:?}, {:?}", self, other)))
            },
        }

        self.container_ref.mark_dirty();
        other.container_ref.mark_dirty();

        Ok(())
    }
}

impl ReferenceImpl {
    /// Swap contents of two passed mutable references.
    ///
    /// Precondition for this function is that `self` and `other` references are required to
    /// be distinct.
    /// Move will guaranteee that invariant, because it prevents from having two mutable
    /// references to the same value.
    fn swap_values(self, other: Self) -> PartialVMResult<()> {
        use ReferenceImpl::*;

        match (self, other) {
            (ContainerRef(r1), ContainerRef(r2)) => r1.swap_values(r2),
            (IndexedRef(r1), IndexedRef(r2)) => r1.swap_values(r2),

            (ContainerRef(_), IndexedRef(_)) | (IndexedRef(_), ContainerRef(_)) => {
                Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message("cannot swap references: reference type mismatch".to_string()))
            },
        }
    }
}

impl Reference {
    pub fn swap_values(self, other: Self) -> PartialVMResult<()> {
        self.0.swap_values(other.0)
    }
}

/***************************************************************************************
 *
 * Borrows (Move)
 *
 *   Implementation of borrowing in Move: borrow field, borrow local and infrastructure
 *   to support borrowing an element from a vector.
 *
 **************************************************************************************/

impl ContainerRef {
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn borrow_elem(&self, idx: usize) -> PartialVMResult<Value> {
        let len = self.container().len();
        if idx >= len {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "index out of bounds when borrowing container element: got: {}, len: {}",
                        idx, len
                    ),
                ),
            );
        }

        macro_rules! container_ref {
            ($container:ident) => {
                Value::ContainerRef(match self {
                    Self::Local(_) => Self::Local($container.copy_by_ref()),
                    Self::Global { status, .. } => Self::Global {
                        status: Rc::clone(status),
                        container: $container.copy_by_ref(),
                    },
                })
            };
        }

        macro_rules! indexed_ref {
            () => {
                Value::IndexedRef(IndexedRef {
                    idx,
                    container_ref: self.copy_by_ref(),
                })
            };
        }

        Ok(match self.container() {
            // Borrowing from vector produces IndexedRef only for delayed fields or closures. Other
            // primitive fields must be handled by specialized containers. If the element is also a
            // container, a ContainerRef is returned.
            Container::Vec(r) => {
                let v = r.borrow();
                match &v[idx] {
                    Value::Container(container) => container_ref!(container),
                    Value::ClosureValue(_) | Value::DelayedFieldID { .. } => indexed_ref!(),

                    Value::U8(_)
                    | Value::U16(_)
                    | Value::U32(_)
                    | Value::U64(_)
                    | Value::U128(_)
                    | Value::U256(_)
                    | Value::I8(_)
                    | Value::I16(_)
                    | Value::I32(_)
                    | Value::I64(_)
                    | Value::I128(_)
                    | Value::I256(_)
                    | Value::Bool(_)
                    | Value::Address(_)
                    | Value::ContainerRef(_)
                    | Value::Invalid
                    | Value::IndexedRef(_) => {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(format!("cannot borrow vector element {:?}", &v[idx])))
                    },
                }
            },

            // Borrowing from locals or structs produces IndexedRef only for primitive types. If
            // element is also a container, we must produce ContainerRef.
            Container::Locals(r) | Container::Struct(r) => {
                let v = r.borrow();
                match &v[idx] {
                    Value::Container(container) => container_ref!(container),
                    Value::U8(_)
                    | Value::U16(_)
                    | Value::U32(_)
                    | Value::U64(_)
                    | Value::U128(_)
                    | Value::U256(_)
                    | Value::I8(_)
                    | Value::I16(_)
                    | Value::I32(_)
                    | Value::I64(_)
                    | Value::I128(_)
                    | Value::I256(_)
                    | Value::Bool(_)
                    | Value::Address(_)
                    | Value::ClosureValue(_)
                    | Value::DelayedFieldID { .. } => indexed_ref!(),

                    Value::ContainerRef(_) | Value::Invalid | Value::IndexedRef(_) => {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(format!(
                            "cannot borrow struct / locals element {:?}",
                            &v[idx]
                        )))
                    },
                }
            },

            // Borrowing a primitive element from specialized container always produces IndexedRef.
            Container::VecU8(_)
            | Container::VecU16(_)
            | Container::VecU32(_)
            | Container::VecU64(_)
            | Container::VecU128(_)
            | Container::VecU256(_)
            | Container::VecI8(_)
            | Container::VecI16(_)
            | Container::VecI32(_)
            | Container::VecI64(_)
            | Container::VecI128(_)
            | Container::VecI256(_)
            | Container::VecAddress(_)
            | Container::VecBool(_) => indexed_ref!(),
        })
    }
}

impl StructRef {
    pub fn borrow_field(&self, idx: usize) -> PartialVMResult<Value> {
        self.0.borrow_elem(idx)
    }

    pub fn borrow_variant_field(
        &self,
        allowed: &[VariantIndex],
        idx: usize,
        variant_to_str: &impl Fn(VariantIndex) -> String,
    ) -> PartialVMResult<Value> {
        let tag = self.get_variant_tag()?;
        if allowed.contains(&tag) {
            Ok(self.0.borrow_elem(idx + 1)?)
        } else {
            Err(
                PartialVMError::new(StatusCode::STRUCT_VARIANT_MISMATCH).with_message(format!(
                    "expected enum variant {}, found `{}`",
                    allowed.iter().cloned().map(variant_to_str).join(" or "),
                    variant_to_str(tag)
                )),
            )
        }
    }

    pub fn test_variant(&self, variant: VariantIndex) -> PartialVMResult<Value> {
        let tag = self.get_variant_tag()?;
        Ok(Value::bool(variant == tag))
    }

    fn get_variant_tag(&self) -> PartialVMResult<VariantIndex> {
        match self.0.container() {
            Container::Struct(vals) => {
                let vals = vals.borrow();
                vals.first()
                    .and_then(|v| match v {
                        Value::U16(x) => Some(*x),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    })
            },
            Container::Locals(_)
            | Container::Vec(_)
            | Container::VecU8(_)
            | Container::VecU64(_)
            | Container::VecU128(_)
            | Container::VecBool(_)
            | Container::VecAddress(_)
            | Container::VecU16(_)
            | Container::VecU32(_)
            | Container::VecU256(_)
            | Container::VecI8(_)
            | Container::VecI16(_)
            | Container::VecI32(_)
            | Container::VecI64(_)
            | Container::VecI128(_)
            | Container::VecI256(_) => Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )),
        }
    }
}

impl Locals {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn borrow_loc(&self, idx: usize) -> PartialVMResult<Value> {
        let v = self.0.borrow();
        if idx >= v.len() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "index out of bounds when borrowing local: got: {}, len: {}",
                        idx,
                        v.len()
                    ),
                ),
            );
        }

        match &v[idx] {
            Value::Container(c) => Ok(Value::ContainerRef(ContainerRef::Local(c.copy_by_ref()))),

            Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_)
            | Value::U128(_)
            | Value::U256(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::I128(_)
            | Value::I256(_)
            | Value::Bool(_)
            | Value::Address(_)
            | Value::ClosureValue(_)
            | Value::DelayedFieldID { .. } => Ok(Value::IndexedRef(IndexedRef {
                idx,
                container_ref: ContainerRef::Local(Container::Locals(Rc::clone(&self.0))),
            })),

            Value::ContainerRef(_) | Value::Invalid | Value::IndexedRef(_) => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("cannot borrow local {:?}", &v[idx])),
            ),
        }
    }
}

impl SignerRef {
    pub fn borrow_signer(&self) -> PartialVMResult<Value> {
        self.0.borrow_elem(1)
    }

    pub fn is_permissioned(&self) -> PartialVMResult<bool> {
        match &self.0 {
            ContainerRef::Local(Container::Struct(s)) => {
                Ok(*s.borrow()[0].as_value_ref::<u16>()? == PERMISSIONED_SIGNER_VARIANT)
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("unexpected signer value: {:?}", self)),
            ),
        }
    }

    /// Get the permission address associated with a signer.
    /// Needs to make sure the signer passed in is a permissioned signer.
    pub fn permission_address(&self) -> PartialVMResult<Value> {
        match &self.0 {
            ContainerRef::Local(Container::Struct(s)) => Ok(Value::address(
                *s.borrow()
                    .get(PERMISSION_ADDRESS_FIELD_OFFSET)
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message(format!("unexpected signer value: {:?}", self))
                    })?
                    .as_value_ref::<AccountAddress>()?,
            )),
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("unexpected signer value: {:?}", self)),
            ),
        }
    }
}

/***************************************************************************************
 *
 * Locals
 *
 *   Public APIs for Locals to support reading, writing and moving of values.
 *
 **************************************************************************************/
impl Locals {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn new(n: usize) -> Self {
        Self(Rc::new(RefCell::new(
            iter::repeat_with(|| Value::Invalid).take(n).collect(),
        )))
    }

    #[cfg_attr(feature = "inline-locals", inline(always))]
    pub fn copy_loc(&self, idx: usize) -> PartialVMResult<Value> {
        let locals = self.0.borrow();
        match locals.get(idx) {
            Some(Value::Invalid) => Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )
            .with_message(format!("cannot copy invalid value at index {}", idx))),
            Some(v) => Ok(v.copy_value(1, Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))?),
            None => Err(Self::local_index_out_of_bounds(idx, locals.len())),
        }
    }

    #[cfg_attr(feature = "inline-locals", inline(always))]
    pub fn move_loc(&mut self, idx: usize) -> PartialVMResult<Value> {
        let mut locals = self.0.borrow_mut();
        match locals.get_mut(idx) {
            Some(Value::Invalid) => Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )
            .with_message(format!("cannot move invalid value at index {}", idx))),
            Some(v) => Ok(std::mem::replace(v, Value::Invalid)),
            None => Err(Self::local_index_out_of_bounds(idx, locals.len())),
        }
    }

    #[cfg_attr(feature = "inline-locals", inline(always))]
    pub fn store_loc(&mut self, idx: usize, x: Value) -> PartialVMResult<()> {
        let mut locals = self.0.borrow_mut();
        match locals.get_mut(idx) {
            Some(v) => {
                *v = x;
            },
            None => {
                return Err(Self::local_index_out_of_bounds(idx, locals.len()));
            },
        }
        Ok(())
    }

    /// Drop all Move values onto a different Vec to avoid leaking memory.
    /// References are excluded since they may point to invalid data.
    #[cfg_attr(feature = "inline-locals", inline(always))]
    pub fn drop_all_values(&mut self) -> Vec<Value> {
        let mut locals = self.0.borrow_mut();
        let mut res = Vec::with_capacity(locals.len());

        for local in locals.iter_mut() {
            match &local {
                Value::Invalid => (),
                Value::ContainerRef(_) | Value::IndexedRef(_) => {
                    *local = Value::Invalid;
                },
                _ => res.push(std::mem::replace(local, Value::Invalid)),
            }
        }

        res
    }

    #[cfg_attr(feature = "inline-locals", inline(always))]
    pub fn is_invalid(&self, idx: usize) -> PartialVMResult<bool> {
        let locals = self.0.borrow();
        match locals.get(idx) {
            Some(Value::Invalid) => Ok(true),
            Some(_) => Ok(false),
            None => Err(Self::local_index_out_of_bounds(idx, locals.len())),
        }
    }

    #[cold]
    fn local_index_out_of_bounds(idx: usize, num_locals: usize) -> PartialVMError {
        PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION).with_message(format!(
            "local index out of bounds: got {}, len: {}",
            idx, num_locals
        ))
    }
}

/***************************************************************************************
 *
 * Public Value Constructors
 *
 *   Constructors to allow values to be created outside this module.
 *
 **************************************************************************************/
impl Value {
    pub fn delayed_value(id: DelayedFieldID) -> Self {
        Value::DelayedFieldID { id }
    }

    pub fn u8(x: u8) -> Self {
        Value::U8(x)
    }

    pub fn u16(x: u16) -> Self {
        Value::U16(x)
    }

    pub fn u32(x: u32) -> Self {
        Value::U32(x)
    }

    pub fn u64(x: u64) -> Self {
        Value::U64(x)
    }

    pub fn u128(x: u128) -> Self {
        Value::U128(x)
    }

    pub fn u256(x: int256::U256) -> Self {
        Value::U256(x)
    }

    pub fn i8(x: i8) -> Self {
        Value::I8(x)
    }

    pub fn i16(x: i16) -> Self {
        Value::I16(x)
    }

    pub fn i32(x: i32) -> Self {
        Value::I32(x)
    }

    pub fn i64(x: i64) -> Self {
        Value::I64(x)
    }

    pub fn i128(x: i128) -> Self {
        Value::I128(x)
    }

    pub fn i256(x: int256::I256) -> Self {
        Value::I256(x)
    }

    pub fn bool(x: bool) -> Self {
        Value::Bool(x)
    }

    pub fn address(x: AccountAddress) -> Self {
        Value::Address(x)
    }

    pub fn master_signer(x: AccountAddress) -> Self {
        Value::Container(Container::master_signer(x))
    }

    pub fn permissioned_signer(x: AccountAddress, perm_storage_address: AccountAddress) -> Self {
        Self::struct_(Struct::pack_variant(PERMISSIONED_SIGNER_VARIANT, vec![
            Value::address(x),
            Value::address(perm_storage_address),
        ]))
    }

    /// Create a "unowned" reference to a signer value (&signer) for populating the &signer in
    /// execute function
    pub fn master_signer_reference(x: AccountAddress) -> Self {
        Value::ContainerRef(ContainerRef::Local(Container::master_signer(x)))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn struct_(s: Struct) -> Self {
        Value::Container(Container::Struct(Rc::new(RefCell::new(s.fields))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u8(it: impl IntoIterator<Item = u8>) -> Self {
        Value::Container(Container::VecU8(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u16(it: impl IntoIterator<Item = u16>) -> Self {
        Value::Container(Container::VecU16(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u32(it: impl IntoIterator<Item = u32>) -> Self {
        Value::Container(Container::VecU32(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u64(it: impl IntoIterator<Item = u64>) -> Self {
        Value::Container(Container::VecU64(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u128(it: impl IntoIterator<Item = u128>) -> Self {
        Value::Container(Container::VecU128(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_u256(it: impl IntoIterator<Item = int256::U256>) -> Self {
        Value::Container(Container::VecU256(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i8(it: impl IntoIterator<Item = i8>) -> Self {
        Value::Container(Container::VecI8(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i16(it: impl IntoIterator<Item = i16>) -> Self {
        Value::Container(Container::VecI16(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i32(it: impl IntoIterator<Item = i32>) -> Self {
        Value::Container(Container::VecI32(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i64(it: impl IntoIterator<Item = i64>) -> Self {
        Value::Container(Container::VecI64(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i128(it: impl IntoIterator<Item = i128>) -> Self {
        Value::Container(Container::VecI128(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_i256(it: impl IntoIterator<Item = int256::I256>) -> Self {
        Value::Container(Container::VecI256(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_bool(it: impl IntoIterator<Item = bool>) -> Self {
        Value::Container(Container::VecBool(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn vector_address(it: impl IntoIterator<Item = AccountAddress>) -> Self {
        Value::Container(Container::VecAddress(Rc::new(RefCell::new(
            it.into_iter().collect(),
        ))))
    }

    /// Creates a vector of values.
    ///
    /// Use with caution. While there is a check for each value that its type is valid (i.e., it
    /// cannot be a primitive like u8 for which there are specialized vectors, or a reference), it
    /// is the caller's responsibility to ensure that the values have the same types and the final
    /// collection is homogeneous.
    pub fn vector_unchecked(it: impl IntoIterator<Item = Value>) -> PartialVMResult<Self> {
        let values = it
            .into_iter()
            .map(|v| {
                v.check_valid_for_value_vector()?;
                Ok(v)
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok(Self::Container(Container::Vec(Rc::new(RefCell::new(
            values,
        )))))
    }

    pub fn closure(
        fun: Box<dyn AbstractFunction>,
        captured: impl IntoIterator<Item = Value>,
    ) -> Self {
        Value::ClosureValue(Closure::pack(fun, captured))
    }
}

/***************************************************************************************
 *
 * Casting
 *
 *   Due to the public value types being opaque to an external user, the following
 *   public APIs are required to enable conversion between types in order to gain access
 *   to specific operations certain more refined types offer.
 *   For example, one must convert a `Value` to a `Struct` before unpack can be called.
 *
 *   It is expected that the caller will keep track of the invariants and guarantee
 *   the conversion will succeed. An error will be raised in case of a violation.
 *
 **************************************************************************************/
// Note(inline): Kinda expensive to inline all the cast functions.
//               Together they add a few seconds of compile time.

pub trait VMValueCast<T> {
    fn cast(self) -> PartialVMResult<T>;
}

macro_rules! impl_vm_value_cast {
    ($ty:ty, $tc:ident) => {
        impl VMValueCast<$ty> for Value {
            #[cfg_attr(feature = "inline-vm-casts", inline)]
            fn cast(self) -> PartialVMResult<$ty> {
                return match self {
                    Value::$tc(x) => Ok(x),
                    v => __cannot_cast(v),
                };
                #[cold]
                fn __cannot_cast(v: Value) -> PartialVMResult<$ty> {
                    Err(
                        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                            "cannot cast {:?} to {}",
                            v,
                            stringify!($ty)
                        )),
                    )
                }
            }
        }
    };
}

impl_vm_value_cast!(u8, U8);
impl_vm_value_cast!(u16, U16);
impl_vm_value_cast!(u32, U32);
impl_vm_value_cast!(u64, U64);
impl_vm_value_cast!(u128, U128);
impl_vm_value_cast!(int256::U256, U256);
impl_vm_value_cast!(i8, I8);
impl_vm_value_cast!(i16, I16);
impl_vm_value_cast!(i32, I32);
impl_vm_value_cast!(i64, I64);
impl_vm_value_cast!(i128, I128);
impl_vm_value_cast!(int256::I256, I256);
impl_vm_value_cast!(bool, Bool);
impl_vm_value_cast!(AccountAddress, Address);
impl_vm_value_cast!(ContainerRef, ContainerRef);
impl_vm_value_cast!(IndexedRef, IndexedRef);

impl VMValueCast<DelayedFieldID> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<DelayedFieldID> {
        match self {
            Value::DelayedFieldID { id } => Ok(id),
            v => Err(
                PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                    "cannot cast non-delayed value {:?} into identifier",
                    v
                )),
            ),
        }
    }
}

impl VMValueCast<Reference> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Reference> {
        match self {
            Value::ContainerRef(r) => Ok(Reference(ReferenceImpl::ContainerRef(r))),
            Value::IndexedRef(r) => Ok(Reference(ReferenceImpl::IndexedRef(r))),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to reference", v,))),
        }
    }
}

impl VMValueCast<Container> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Container> {
        match self {
            Value::Container(c) => Ok(c),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to container", v,))),
        }
    }
}

impl VMValueCast<Struct> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Struct> {
        match self {
            Value::Container(Container::Struct(r)) => Ok(Struct {
                fields: take_unique_ownership(r)?,
            }),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to struct", v,))),
        }
    }
}

impl VMValueCast<StructRef> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<StructRef> {
        Ok(StructRef(VMValueCast::cast(self)?))
    }
}

impl VMValueCast<Vec<u8>> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Vec<u8>> {
        match self {
            Value::Container(Container::VecU8(r)) => take_unique_ownership(r),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to vector<u8>", v,))),
        }
    }
}

impl VMValueCast<Vec<u64>> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Vec<u64>> {
        match self {
            Value::Container(Container::VecU64(r)) => take_unique_ownership(r),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to vector<u64>", v,))),
        }
    }
}

impl VMValueCast<Vec<Value>> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Vec<Value>> {
        match self {
            Value::Container(Container::Vec(c)) => {
                Ok(take_unique_ownership(c)?.into_iter().collect())
            },
            Value::Address(_)
            | Value::Bool(_)
            | Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_)
            | Value::U128(_)
            | Value::U256(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::I128(_)
            | Value::I256(_) => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(
                    "cannot cast a specialized vector into a non-specialized one".to_string(),
                )),
            v => Err(
                PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                    "cannot cast {:?} to vector<non-specialized-type>",
                    v,
                )),
            ),
        }
    }
}

impl VMValueCast<SignerRef> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<SignerRef> {
        match self {
            Value::ContainerRef(r) => Ok(SignerRef(r)),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to Signer reference", v,))),
        }
    }
}

impl VMValueCast<VectorRef> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<VectorRef> {
        match self {
            Value::ContainerRef(r) => Ok(VectorRef(r)),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to vector reference", v,))),
        }
    }
}

impl VMValueCast<Vector> for Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    fn cast(self) -> PartialVMResult<Vector> {
        match self {
            Value::Container(c) => Ok(Vector(c)),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to vector", v,))),
        }
    }
}

impl Value {
    #[cfg_attr(feature = "inline-vm-casts", inline)]
    pub fn value_as<T>(self) -> PartialVMResult<T>
    where
        Self: VMValueCast<T>,
    {
        VMValueCast::cast(self)
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, Value::Invalid)
    }

    pub fn is_zero(self) -> bool {
        match self {
            Self::U8(x) => x == 0,
            Self::U16(x) => x == 0,
            Self::U32(x) => x == 0,
            Self::U64(x) => x == 0,
            Self::U128(x) => x == 0,
            Self::U256(x) => x == int256::U256::ZERO,
            Self::I8(x) => x == 0,
            Self::I16(x) => x == 0,
            Self::I32(x) => x == 0,
            Self::I64(x) => x == 0,
            Self::I128(x) => x == 0,
            Self::I256(x) => x == int256::I256::ZERO,
            _ => false,
        }
    }
}

/***************************************************************************************
 *
 * Integer Operations
 *
 *   Arithmetic operations and conversions for integer values.
 *
 **************************************************************************************/
impl Value {
    pub fn add_checked(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match (self, other) {
            (U8(l), U8(r)) => u8::checked_add(l, r).map(U8),
            (U16(l), U16(r)) => u16::checked_add(l, r).map(U16),
            (U32(l), U32(r)) => u32::checked_add(l, r).map(U32),
            (U64(l), U64(r)) => u64::checked_add(l, r).map(U64),
            (U128(l), U128(r)) => u128::checked_add(l, r).map(U128),
            (U256(l), U256(r)) => int256::U256::checked_add(l, r).map(U256),
            (I8(l), I8(r)) => i8::checked_add(l, r).map(I8),
            (I16(l), I16(r)) => i16::checked_add(l, r).map(I16),
            (I32(l), I32(r)) => i32::checked_add(l, r).map(I32),
            (I64(l), I64(r)) => i64::checked_add(l, r).map(I64),
            (I128(l), I128(r)) => i128::checked_add(l, r).map(I128),
            (I256(l), I256(r)) => int256::I256::checked_add(l, r).map(I256),
            (l, r) => {
                let msg = format!("Cannot add {:?} and {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                .with_message("Addition overflow".to_string())
        })
    }

    pub fn sub_checked(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match (self, other) {
            (U8(l), U8(r)) => u8::checked_sub(l, r).map(U8),
            (U16(l), U16(r)) => u16::checked_sub(l, r).map(U16),
            (U32(l), U32(r)) => u32::checked_sub(l, r).map(U32),
            (U64(l), U64(r)) => u64::checked_sub(l, r).map(U64),
            (U128(l), U128(r)) => u128::checked_sub(l, r).map(U128),
            (U256(l), U256(r)) => int256::U256::checked_sub(l, r).map(U256),
            (I8(l), I8(r)) => i8::checked_sub(l, r).map(I8),
            (I16(l), I16(r)) => i16::checked_sub(l, r).map(I16),
            (I32(l), I32(r)) => i32::checked_sub(l, r).map(I32),
            (I64(l), I64(r)) => i64::checked_sub(l, r).map(I64),
            (I128(l), I128(r)) => i128::checked_sub(l, r).map(I128),
            (I256(l), I256(r)) => int256::I256::checked_sub(l, r).map(I256),
            (l, r) => {
                let msg = format!("Cannot sub {:?} from {:?}", r, l);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                .with_message("Subtraction overflow".to_string())
        })
    }

    pub fn mul_checked(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match (self, other) {
            (U8(l), U8(r)) => u8::checked_mul(l, r).map(U8),
            (U16(l), U16(r)) => u16::checked_mul(l, r).map(U16),
            (U32(l), U32(r)) => u32::checked_mul(l, r).map(U32),
            (U64(l), U64(r)) => u64::checked_mul(l, r).map(U64),
            (U128(l), U128(r)) => u128::checked_mul(l, r).map(U128),
            (U256(l), U256(r)) => int256::U256::checked_mul(l, r).map(U256),
            (I8(l), I8(r)) => i8::checked_mul(l, r).map(I8),
            (I16(l), I16(r)) => i16::checked_mul(l, r).map(I16),
            (I32(l), I32(r)) => i32::checked_mul(l, r).map(I32),
            (I64(l), I64(r)) => i64::checked_mul(l, r).map(I64),
            (I128(l), I128(r)) => i128::checked_mul(l, r).map(I128),
            (I256(l), I256(r)) => int256::I256::checked_mul(l, r).map(I256),
            (l, r) => {
                let msg = format!("Cannot mul {:?} and {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                .with_message("Multiplication overflow".to_string())
        })
    }

    pub fn div_checked(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match (self, &other) {
            (U8(l), U8(r)) => u8::checked_div(l, *r).map(U8),
            (U16(l), U16(r)) => u16::checked_div(l, *r).map(U16),
            (U32(l), U32(r)) => u32::checked_div(l, *r).map(U32),
            (U64(l), U64(r)) => u64::checked_div(l, *r).map(U64),
            (U128(l), U128(r)) => u128::checked_div(l, *r).map(U128),
            (U256(l), U256(r)) => int256::U256::checked_div(l, *r).map(U256),
            (I8(l), I8(r)) => i8::checked_div(l, *r).map(I8),
            (I16(l), I16(r)) => i16::checked_div(l, *r).map(I16),
            (I32(l), I32(r)) => i32::checked_div(l, *r).map(I32),
            (I64(l), I64(r)) => i64::checked_div(l, *r).map(I64),
            (I128(l), I128(r)) => i128::checked_div(l, *r).map(I128),
            (I256(l), I256(r)) => int256::I256::checked_div(l, *r).map(I256),
            (l, r) => {
                let msg = format!("Cannot div {:?} by {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            let msg = if other.is_zero() {
                "Division by zero".to_string()
            } else {
                "Division overflow".to_string() // This happens when dividing the minimum negative value by -1
            };
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(msg)
        })
    }

    pub fn rem_checked(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match (self, other) {
            (U8(l), U8(r)) => u8::checked_rem(l, r).map(U8),
            (U16(l), U16(r)) => u16::checked_rem(l, r).map(U16),
            (U32(l), U32(r)) => u32::checked_rem(l, r).map(U32),
            (U64(l), U64(r)) => u64::checked_rem(l, r).map(U64),
            (U128(l), U128(r)) => u128::checked_rem(l, r).map(U128),
            (U256(l), U256(r)) => int256::U256::checked_rem(l, r).map(U256),
            (I8(l), I8(r)) => i8::checked_rem(l, r).map(I8),
            (I16(l), I16(r)) => i16::checked_rem(l, r).map(I16),
            (I32(l), I32(r)) => i32::checked_rem(l, r).map(I32),
            (I64(l), I64(r)) => i64::checked_rem(l, r).map(I64),
            (I128(l), I128(r)) => i128::checked_rem(l, r).map(I128),
            (I256(l), I256(r)) => int256::I256::checked_rem(l, r).map(I256),
            (l, r) => {
                let msg = format!("Cannot rem {:?} by {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                .with_message("Integer remainder by zero".to_string())
        })
    }

    pub fn negate_checked(self) -> PartialVMResult<Self> {
        use Value::*;
        let res = match self {
            I8(x) => x.checked_neg().map(I8),
            I16(x) => x.checked_neg().map(I16),
            I32(x) => x.checked_neg().map(I32),
            I64(x) => x.checked_neg().map(I64),
            I128(x) => x.checked_neg().map(I128),
            I256(x) => x.checked_neg().map(I256),
            _ => {
                let msg = format!("Cannot negate {:?}", self);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        };
        res.ok_or_else(|| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                .with_message("Integer negation overflow".to_string())
        })
    }

    pub fn bit_or(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        Ok(match (self, other) {
            (U8(l), U8(r)) => U8(l | r),
            (U16(l), U16(r)) => U16(l | r),
            (U32(l), U32(r)) => U32(l | r),
            (U64(l), U64(r)) => U64(l | r),
            (U128(l), U128(r)) => U128(l | r),
            (U256(l), U256(r)) => U256(l | r),
            (l, r) => {
                let msg = format!("Cannot bit_or {:?} and {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn bit_and(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        Ok(match (self, other) {
            (U8(l), U8(r)) => U8(l & r),
            (U16(l), U16(r)) => U16(l & r),
            (U32(l), U32(r)) => U32(l & r),
            (U64(l), U64(r)) => U64(l & r),
            (U128(l), U128(r)) => U128(l & r),
            (U256(l), U256(r)) => U256(l & r),
            (l, r) => {
                let msg = format!("Cannot bit_and {:?} and {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn bit_xor(self, other: Self) -> PartialVMResult<Self> {
        use Value::*;
        Ok(match (self, other) {
            (U8(l), U8(r)) => U8(l ^ r),
            (U16(l), U16(r)) => U16(l ^ r),
            (U32(l), U32(r)) => U32(l ^ r),
            (U64(l), U64(r)) => U64(l ^ r),
            (U128(l), U128(r)) => U128(l ^ r),
            (U256(l), U256(r)) => U256(l ^ r),
            (l, r) => {
                let msg = format!("Cannot bit_xor {:?} and {:?}", l, r);
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn shl_checked(self, n_bits: u8) -> PartialVMResult<Self> {
        use Value::*;

        Ok(match self {
            U8(x) if n_bits < 8 => U8(x << n_bits),
            U16(x) if n_bits < 16 => U16(x << n_bits),
            U32(x) if n_bits < 32 => U32(x << n_bits),
            U64(x) if n_bits < 64 => U64(x << n_bits),
            U128(x) if n_bits < 128 => U128(x << n_bits),
            U256(x) => U256(x << int256::U256::from(n_bits)),
            _ => {
                return Err(PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                    .with_message("Shift Left overflow".to_string()));
            },
        })
    }

    pub fn shr_checked(self, n_bits: u8) -> PartialVMResult<Self> {
        use Value::*;

        Ok(match self {
            U8(x) if n_bits < 8 => U8(x >> n_bits),
            U16(x) if n_bits < 16 => U16(x >> n_bits),
            U32(x) if n_bits < 32 => U32(x >> n_bits),
            U64(x) if n_bits < 64 => U64(x >> n_bits),
            U128(x) if n_bits < 128 => U128(x >> n_bits),
            U256(x) => U256(x >> int256::U256::from(n_bits)),
            _ => {
                return Err(PartialVMError::new(StatusCode::ARITHMETIC_ERROR)
                    .with_message("Shift Right overflow".to_string()));
            },
        })
    }

    pub fn lt(self, other: Self) -> PartialVMResult<bool> {
        use Value::*;

        Ok(match (self, other) {
            (U8(l), U8(r)) => l < r,
            (U16(l), U16(r)) => l < r,
            (U32(l), U32(r)) => l < r,
            (U64(l), U64(r)) => l < r,
            (U128(l), U128(r)) => l < r,
            (U256(l), U256(r)) => l < r,
            (I8(l), I8(r)) => l < r,
            (I16(l), I16(r)) => l < r,
            (I32(l), I32(r)) => l < r,
            (I64(l), I64(r)) => l < r,
            (I128(l), I128(r)) => l < r,
            (I256(l), I256(r)) => l < r,
            (l, r) => {
                let msg = format!(
                    "Cannot compare {:?} and {:?}: incompatible integer types",
                    l, r
                );
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn le(self, other: Self) -> PartialVMResult<bool> {
        use Value::*;

        Ok(match (self, other) {
            (U8(l), U8(r)) => l <= r,
            (U16(l), U16(r)) => l <= r,
            (U32(l), U32(r)) => l <= r,
            (U64(l), U64(r)) => l <= r,
            (U128(l), U128(r)) => l <= r,
            (U256(l), U256(r)) => l <= r,
            (I8(l), I8(r)) => l <= r,
            (I16(l), I16(r)) => l <= r,
            (I32(l), I32(r)) => l <= r,
            (I64(l), I64(r)) => l <= r,
            (I128(l), I128(r)) => l <= r,
            (I256(l), I256(r)) => l <= r,

            (l, r) => {
                let msg = format!(
                    "Cannot compare {:?} and {:?}: incompatible integer types",
                    l, r
                );
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn gt(self, other: Self) -> PartialVMResult<bool> {
        use Value::*;

        Ok(match (self, other) {
            (U8(l), U8(r)) => l > r,
            (U16(l), U16(r)) => l > r,
            (U32(l), U32(r)) => l > r,
            (U64(l), U64(r)) => l > r,
            (U128(l), U128(r)) => l > r,
            (U256(l), U256(r)) => l > r,
            (I8(l), I8(r)) => l > r,
            (I16(l), I16(r)) => l > r,
            (I32(l), I32(r)) => l > r,
            (I64(l), I64(r)) => l > r,
            (I128(l), I128(r)) => l > r,
            (I256(l), I256(r)) => l > r,
            (l, r) => {
                let msg = format!(
                    "Cannot compare {:?} and {:?}: incompatible integer types",
                    l, r
                );
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }

    pub fn ge(self, other: Self) -> PartialVMResult<bool> {
        use Value::*;

        Ok(match (self, other) {
            (U8(l), U8(r)) => l >= r,
            (U16(l), U16(r)) => l >= r,
            (U32(l), U32(r)) => l >= r,
            (U64(l), U64(r)) => l >= r,
            (U128(l), U128(r)) => l >= r,
            (U256(l), U256(r)) => l >= r,
            (I8(l), I8(r)) => l >= r,
            (I16(l), I16(r)) => l >= r,
            (I32(l), I32(r)) => l >= r,
            (I64(l), I64(r)) => l >= r,
            (I128(l), I128(r)) => l >= r,
            (I256(l), I256(r)) => l >= r,
            (l, r) => {
                let msg = format!(
                    "Cannot compare {:?} and {:?}: incompatible integer types",
                    l, r
                );
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg));
            },
        })
    }
}

//* ==== a list of macros to help integer type casting ====
//*  Please do not merge them, as that will introduce extra runtime checks

/// Cast unsigned to unsigned or signed to signed where the target type is larger than the source type.
/// No checks are needed.
macro_rules! cast_int_widening {
    ($source:ty, $target:ty, $value:expr) => {{
        Ok($value as $target)
    }};
}

/// Cast unsigned to unsigned or signed to signed where the target type is smaller than the source type.
/// Value must fit into the target type.
macro_rules! cast_int_narrowing {
    ($source:ty, $target:ty, $value:expr) => {{
        if $value > (<$target>::MAX as $source) || $value < (<$target>::MIN as $source) {
            Err(
                PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(format!(
                    "Cannot cast {}({}) to {}",
                    stringify!($source),
                    $value,
                    stringify!($target)
                )),
            )
        } else {
            Ok($value as $target)
        }
    }};
}

/// Cast signed to unsigned, where the target type is smaller than the source type.
/// Value must be non-negative and fit into the target type.
macro_rules! cast_int_i2u_narrowing {
    ($source:ty, $target:ty, $value:expr) => {{
        if $value < 0 || $value > (<$target>::MAX as $source) {
            Err(
                PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(format!(
                    "Cannot cast {}({}) to {}",
                    stringify!($source),
                    $value,
                    stringify!($target)
                )),
            )
        } else {
            Ok($value as $target)
        }
    }};
}

/// Cast signed to unsigned, where the target type is larger than the source type.
/// Value must be non-negative.
macro_rules! cast_int_i2u_widening {
    ($source:ty, $target:ty, $value:expr) => {{
        if $value < 0 {
            Err(
                PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(format!(
                    "Cannot cast {}({}) to {}",
                    stringify!($source),
                    $value,
                    stringify!($target)
                )),
            )
        } else {
            Ok($value as $target)
        }
    }};
}

/// Cast unsigned to signed, where the target type is larger than the source type.
/// No checks needed
macro_rules! cast_int_u2i_widening {
    ($source:ty, $target:ty, $value:expr) => {{
        Ok($value as $target)
    }};
}

/// Cast unsigned to signed, where the target type is smaller than the source type.
/// Value must fit into the target type.
macro_rules! cast_int_u2i_narrowing {
    ($source:ty, $target:ty, $value:expr) => {{
        if $value > (<$target>::MAX as $source) {
            Err(
                PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(format!(
                    "Cannot cast {}({}) to {}",
                    stringify!($source),
                    $value,
                    stringify!($target)
                )),
            )
        } else {
            Ok($value as $target)
        }
    }};
}

/// Cast for types which do not support `as` but `try_from` instead. We prefer native
/// `as` since it is just a reinterpret-cast and likely faster.
macro_rules! cast_int_with_try_from {
    ($source:ty, $target:ty, $value:expr) => {{
        <$target>::try_from($value).map_err(|_| {
            PartialVMError::new(StatusCode::ARITHMETIC_ERROR).with_message(format!(
                "Cannot cast {}({}) to {}",
                stringify!($source),
                $value,
                stringify!($target)
            ))
        })
    }};
}

impl Value {
    fn no_int_cast_err<T>(v: Self) -> PartialVMResult<T> {
        let msg = format!("Cannot cast {:?}: not an integer", v);
        Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(msg))
    }

    pub fn cast_u8(self) -> PartialVMResult<u8> {
        use Value::*;

        match self {
            U8(x) => Ok(x),
            U16(x) => cast_int_narrowing!(u16, u8, x),
            U32(x) => cast_int_narrowing!(u32, u8, x),
            U64(x) => cast_int_narrowing!(u64, u8, x),
            U128(x) => cast_int_narrowing!(u128, u8, x),
            U256(x) => cast_int_with_try_from!(U256, u8, x),
            I8(x) => cast_int_i2u_widening!(i8, u8, x),
            I16(x) => cast_int_i2u_narrowing!(i16, u8, x),
            I32(x) => cast_int_i2u_narrowing!(i32, u8, x),
            I64(x) => cast_int_i2u_narrowing!(i64, u8, x),
            I128(x) => cast_int_i2u_narrowing!(i128, u8, x),
            I256(x) => cast_int_with_try_from!(I256, u8, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_u16(self) -> PartialVMResult<u16> {
        use Value::*;

        match self {
            U8(x) => cast_int_widening!(u8, u16, x),
            U16(x) => Ok(x),
            U32(x) => cast_int_narrowing!(u32, u16, x),
            U64(x) => cast_int_narrowing!(u64, u16, x),
            U128(x) => cast_int_narrowing!(u128, u16, x),
            U256(x) => cast_int_with_try_from!(U256, u16, x),
            I8(x) => cast_int_i2u_widening!(i8, u16, x),
            I16(x) => cast_int_i2u_widening!(i16, u16, x),
            I32(x) => cast_int_i2u_narrowing!(i32, u16, x),
            I64(x) => cast_int_i2u_narrowing!(i64, u16, x),
            I128(x) => cast_int_i2u_narrowing!(i128, u16, x),
            I256(x) => cast_int_with_try_from!(I256, u16, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_u32(self) -> PartialVMResult<u32> {
        use Value::*;

        match self {
            U8(x) => cast_int_widening!(u8, u32, x),
            U16(x) => cast_int_widening!(u16, u32, x),
            U32(x) => Ok(x),
            U64(x) => cast_int_narrowing!(u64, u32, x),
            U128(x) => cast_int_narrowing!(u128, u32, x),
            U256(x) => cast_int_with_try_from!(U256, u32, x),
            I8(x) => cast_int_i2u_widening!(i8, u32, x),
            I16(x) => cast_int_i2u_widening!(i16, u32, x),
            I32(x) => cast_int_i2u_widening!(i32, u32, x),
            I64(x) => cast_int_i2u_narrowing!(i64, u32, x),
            I128(x) => cast_int_i2u_narrowing!(i128, u32, x),
            I256(x) => cast_int_with_try_from!(I256, u32, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_u64(self) -> PartialVMResult<u64> {
        use Value::*;

        match self {
            U8(x) => cast_int_widening!(u8, u64, x),
            U16(x) => cast_int_widening!(u16, u64, x),
            U32(x) => cast_int_widening!(u32, u64, x),
            U64(x) => Ok(x),
            U128(x) => cast_int_narrowing!(u128, u64, x),
            U256(x) => cast_int_with_try_from!(U256, u64, x),
            I8(x) => cast_int_i2u_widening!(i8, u64, x),
            I16(x) => cast_int_i2u_widening!(i16, u64, x),
            I32(x) => cast_int_i2u_widening!(i32, u64, x),
            I64(x) => cast_int_i2u_widening!(i64, u64, x),
            I128(x) => cast_int_i2u_narrowing!(i128, u64, x),
            I256(x) => cast_int_with_try_from!(I256, u64, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_u128(self) -> PartialVMResult<u128> {
        use Value::*;

        match self {
            U8(x) => cast_int_widening!(u8, u128, x),
            U16(x) => cast_int_widening!(u16, u128, x),
            U32(x) => cast_int_widening!(u32, u128, x),
            U64(x) => cast_int_widening!(u64, u128, x),
            U128(x) => Ok(x),
            U256(x) => cast_int_with_try_from!(U256, u128, x),
            I8(x) => cast_int_i2u_widening!(i8, u128, x),
            I16(x) => cast_int_i2u_widening!(i16, u128, x),
            I32(x) => cast_int_i2u_widening!(i32, u128, x),
            I64(x) => cast_int_i2u_widening!(i64, u128, x),
            I128(x) => cast_int_i2u_widening!(i128, u128, x),
            I256(x) => cast_int_with_try_from!(I256, u128, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_u256(self) -> PartialVMResult<int256::U256> {
        use Value::*;

        Ok(match self {
            U8(x) => int256::U256::from(x),
            U16(x) => int256::U256::from(x),
            U32(x) => int256::U256::from(x),
            U64(x) => int256::U256::from(x),
            U128(x) => int256::U256::from(x),
            U256(x) => x,
            I8(x) => cast_int_with_try_from!(i8, int256::U256, x)?,
            I16(x) => cast_int_with_try_from!(i16, int256::U256, x)?,
            I32(x) => cast_int_with_try_from!(i32, int256::U256, x)?,
            I64(x) => cast_int_with_try_from!(i64, int256::U256, x)?,
            I128(x) => cast_int_with_try_from!(i128, int256::U256, x)?,
            I256(x) => cast_int_with_try_from!(I256, int256::U256, x)?,
            v => Self::no_int_cast_err(v)?,
        })
    }

    pub fn cast_i8(self) -> PartialVMResult<i8> {
        use Value::*;

        match self {
            U8(x) => cast_int_u2i_narrowing!(u8, i8, x),
            U16(x) => cast_int_u2i_narrowing!(u16, i8, x),
            U32(x) => cast_int_u2i_narrowing!(u32, i8, x),
            U64(x) => cast_int_u2i_narrowing!(u64, i8, x),
            U128(x) => cast_int_u2i_narrowing!(u128, i8, x),
            U256(x) => cast_int_with_try_from!(U256, i8, x),
            I8(x) => Ok(x),
            I16(x) => cast_int_narrowing!(i16, i8, x),
            I32(x) => cast_int_narrowing!(i32, i8, x),
            I64(x) => cast_int_narrowing!(i64, i8, x),
            I128(x) => cast_int_narrowing!(i128, i8, x),
            I256(x) => cast_int_with_try_from!(I256, i8, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_i16(self) -> PartialVMResult<i16> {
        use Value::*;

        match self {
            U8(x) => cast_int_u2i_widening!(u8, i16, x),
            U16(x) => cast_int_u2i_narrowing!(u16, i16, x),
            U32(x) => cast_int_u2i_narrowing!(u32, i16, x),
            U64(x) => cast_int_u2i_narrowing!(u64, i16, x),
            U128(x) => cast_int_u2i_narrowing!(u128, i16, x),
            U256(x) => cast_int_with_try_from!(U256, i16, x),
            I8(x) => cast_int_widening!(i8, i16, x),
            I16(x) => Ok(x),
            I32(x) => cast_int_narrowing!(i32, i16, x),
            I64(x) => cast_int_narrowing!(i64, i16, x),
            I128(x) => cast_int_narrowing!(i128, i16, x),
            I256(x) => cast_int_with_try_from!(I256, i16, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_i32(self) -> PartialVMResult<i32> {
        use Value::*;

        match self {
            U8(x) => cast_int_u2i_widening!(u8, i32, x),
            U16(x) => cast_int_u2i_widening!(u16, i32, x),
            U32(x) => cast_int_u2i_narrowing!(u32, i32, x),
            U64(x) => cast_int_u2i_narrowing!(u64, i32, x),
            U128(x) => cast_int_u2i_narrowing!(u128, i32, x),
            U256(x) => cast_int_with_try_from!(U256, i32, x),
            I8(x) => cast_int_widening!(i8, i32, x),
            I16(x) => cast_int_widening!(i16, i32, x),
            I32(x) => Ok(x),
            I64(x) => cast_int_narrowing!(i64, i32, x),
            I128(x) => cast_int_narrowing!(i128, i32, x),
            I256(x) => cast_int_with_try_from!(I256, i32, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_i64(self) -> PartialVMResult<i64> {
        use Value::*;

        match self {
            U8(x) => cast_int_u2i_widening!(u8, i64, x),
            U16(x) => cast_int_u2i_widening!(u16, i64, x),
            U32(x) => cast_int_u2i_widening!(u32, i64, x),
            U64(x) => cast_int_u2i_narrowing!(u64, i64, x),
            U128(x) => cast_int_u2i_narrowing!(u128, i64, x),
            U256(x) => cast_int_with_try_from!(U256, i64, x),
            I8(x) => cast_int_widening!(i8, i64, x),
            I16(x) => cast_int_widening!(i16, i64, x),
            I32(x) => cast_int_widening!(i32, i64, x),
            I64(x) => Ok(x),
            I128(x) => cast_int_narrowing!(i128, i64, x),
            I256(x) => cast_int_with_try_from!(I256, i64, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_i128(self) -> PartialVMResult<i128> {
        use Value::*;

        match self {
            U8(x) => cast_int_u2i_widening!(u8, i128, x),
            U16(x) => cast_int_u2i_widening!(u16, i128, x),
            U32(x) => cast_int_u2i_widening!(u32, i128, x),
            U64(x) => cast_int_u2i_widening!(u64, i128, x),
            U128(x) => cast_int_u2i_narrowing!(u128, i128, x),
            U256(x) => cast_int_with_try_from!(U256, i128, x),
            I8(x) => cast_int_widening!(i8, i128, x),
            I16(x) => cast_int_widening!(i16, i128, x),
            I32(x) => cast_int_widening!(i32, i128, x),
            I64(x) => cast_int_widening!(i64, i128, x),
            I128(x) => Ok(x),
            I256(x) => cast_int_with_try_from!(I256, i128, x),
            v => Self::no_int_cast_err(v),
        }
    }

    pub fn cast_i256(self) -> PartialVMResult<int256::I256> {
        use Value::*;

        match self {
            U8(x) => Ok(int256::I256::from(x)),
            U16(x) => Ok(int256::I256::from(x)),
            U32(x) => Ok(int256::I256::from(x)),
            U64(x) => Ok(int256::I256::from(x)),
            U128(x) => Ok(int256::I256::from(x)),
            U256(x) => cast_int_with_try_from!(int256::U256, int256::I256, x),
            I8(x) => Ok(int256::I256::from(x)),
            I16(x) => Ok(int256::I256::from(x)),
            I32(x) => Ok(int256::I256::from(x)),
            I64(x) => Ok(int256::I256::from(x)),
            I128(x) => Ok(int256::I256::from(x)),
            I256(x) => Ok(x),
            v => Self::no_int_cast_err(v),
        }
    }
}

/***************************************************************************************
*
* Vector
*
*   Implemented as a built-in data type.
*
**************************************************************************************/

pub const INDEX_OUT_OF_BOUNDS: u64 = NFE_VECTOR_ERROR_BASE + 1;
pub const POP_EMPTY_VEC: u64 = NFE_VECTOR_ERROR_BASE + 2;
pub const VEC_UNPACK_PARITY_MISMATCH: u64 = NFE_VECTOR_ERROR_BASE + 3;

// Note(inline): Inlining all vector functions adds ~10s to compile time.

// TODO: this check seems to be obsolete if paranoid mode is on,
//   and should either be removed or move over to runtime_type_checks?
#[cfg_attr(feature = "force-inline", inline(always))]
fn check_elem_layout(ty: &Type, v: &Container) -> PartialVMResult<()> {
    match (ty, v) {
        (Type::U8, Container::VecU8(_))
        | (Type::U64, Container::VecU64(_))
        | (Type::U16, Container::VecU16(_))
        | (Type::U32, Container::VecU32(_))
        | (Type::U128, Container::VecU128(_))
        | (Type::U256, Container::VecU256(_))
        | (Type::I8, Container::VecI8(_))
        | (Type::I64, Container::VecI64(_))
        | (Type::I16, Container::VecI16(_))
        | (Type::I32, Container::VecI32(_))
        | (Type::I128, Container::VecI128(_))
        | (Type::I256, Container::VecI256(_))
        | (Type::Bool, Container::VecBool(_))
        | (Type::Address, Container::VecAddress(_))
        | (Type::Signer, Container::Struct(_)) => Ok(()),

        (Type::Vector(_), Container::Vec(_)) => Ok(()),

        (Type::Struct { .. }, Container::Vec(_))
        | (Type::Signer, Container::Vec(_))
        | (Type::StructInstantiation { .. }, Container::Vec(_))
        | (Type::Function { .. }, Container::Vec(_)) => Ok(()),

        (Type::Reference(_), _) | (Type::MutableReference(_), _) | (Type::TyParam(_), _) => Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(format!("invalid type param for vector: {:?}", ty)),
        ),

        (Type::U8, _)
        | (Type::U64, _)
        | (Type::U16, _)
        | (Type::U32, _)
        | (Type::U128, _)
        | (Type::U256, _)
        | (Type::I8, _)
        | (Type::I64, _)
        | (Type::I16, _)
        | (Type::I32, _)
        | (Type::I128, _)
        | (Type::I256, _)
        | (Type::Bool, _)
        | (Type::Address, _)
        | (Type::Signer, _)
        | (Type::Vector(_), _)
        | (Type::Struct { .. }, _)
        | (Type::StructInstantiation { .. }, _)
        | (Type::Function { .. }, _) => Err(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )
        .with_message(format!(
            "vector elem layout mismatch, expected {:?}, got {:?}",
            ty, v
        ))),
    }
}

impl VectorRef {
    // note(inline): too big and too cold to inline
    pub fn length_as_usize(&self) -> PartialVMResult<usize> {
        let c: &Container = self.0.container();

        let len = match c {
            Container::VecU8(r) => r.borrow().len(),
            Container::VecU16(r) => r.borrow().len(),
            Container::VecU32(r) => r.borrow().len(),
            Container::VecU64(r) => r.borrow().len(),
            Container::VecU128(r) => r.borrow().len(),
            Container::VecU256(r) => r.borrow().len(),
            Container::VecI8(r) => r.borrow().len(),
            Container::VecI16(r) => r.borrow().len(),
            Container::VecI32(r) => r.borrow().len(),
            Container::VecI64(r) => r.borrow().len(),
            Container::VecI128(r) => r.borrow().len(),
            Container::VecI256(r) => r.borrow().len(),
            Container::VecBool(r) => r.borrow().len(),
            Container::VecAddress(r) => r.borrow().len(),
            Container::Vec(r) => r.borrow().len(),
            Container::Locals(_) | Container::Struct(_) => unreachable!(),
        };
        Ok(len)
    }

    #[inline]
    pub fn len(&self) -> PartialVMResult<Value> {
        Ok(Value::u64(self.length_as_usize()? as u64))
    }

    // note(inline): too big and too cold to inline
    pub fn push_back(&self, e: Value) -> PartialVMResult<()> {
        let c = self.0.container();

        match c {
            Container::VecU8(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecU16(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecU32(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecU64(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecU128(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecU256(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI8(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI16(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI32(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI64(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI128(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecI256(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecBool(r) => r.borrow_mut().push(e.value_as()?),
            Container::VecAddress(r) => r.borrow_mut().push(e.value_as()?),
            Container::Vec(r) => r.borrow_mut().push(e),
            Container::Locals(_) | Container::Struct(_) => unreachable!(),
        }

        self.0.mark_dirty();
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn borrow_elem(&self, idx: usize) -> PartialVMResult<Value> {
        let c = self.0.container();
        if idx >= c.len() {
            return Err(PartialVMError::new(StatusCode::VECTOR_OPERATION_ERROR)
                .with_sub_status(INDEX_OUT_OF_BOUNDS));
        }
        self.0.borrow_elem(idx)
    }

    /// Returns a RefCell reference to the underlying vector of a `&vector<u8>` value.
    pub fn as_bytes_ref(&self) -> std::cell::Ref<'_, Vec<u8>> {
        let c = self.0.container();
        match c {
            Container::VecU8(r) => r.borrow(),
            _ => panic!("can only be called on vector<u8>"),
        }
    }

    // note(inline): too big and too cold to inline
    pub fn pop(&self) -> PartialVMResult<Value> {
        let c = self.0.container();

        macro_rules! err_pop_empty_vec {
            () => {
                return Err(PartialVMError::new(StatusCode::VECTOR_OPERATION_ERROR)
                    .with_sub_status(POP_EMPTY_VEC))
            };
        }

        let res = match c {
            Container::VecU8(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u8(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecU16(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u16(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecU32(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u32(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecU64(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u64(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecU128(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u128(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecU256(r) => match r.borrow_mut().pop() {
                Some(x) => Value::u256(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI8(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i8(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI16(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i16(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI32(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i32(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI64(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i64(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI128(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i128(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecI256(r) => match r.borrow_mut().pop() {
                Some(x) => Value::i256(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecBool(r) => match r.borrow_mut().pop() {
                Some(x) => Value::bool(x),
                None => err_pop_empty_vec!(),
            },
            Container::VecAddress(r) => match r.borrow_mut().pop() {
                Some(x) => Value::address(x),
                None => err_pop_empty_vec!(),
            },
            Container::Vec(r) => match r.borrow_mut().pop() {
                Some(x) => x,
                None => err_pop_empty_vec!(),
            },
            Container::Locals(_) | Container::Struct(_) => unreachable!(),
        };

        self.0.mark_dirty();
        Ok(res)
    }

    pub fn swap(&self, idx1: usize, idx2: usize) -> PartialVMResult<()> {
        let c = self.0.container();

        macro_rules! swap {
            ($v:expr) => {{
                let mut v = $v.borrow_mut();
                if idx1 >= v.len() || idx2 >= v.len() {
                    return Err(PartialVMError::new(StatusCode::VECTOR_OPERATION_ERROR)
                        .with_sub_status(INDEX_OUT_OF_BOUNDS));
                }
                v.swap(idx1, idx2);
            }};
        }

        match c {
            Container::VecU8(r) => swap!(r),
            Container::VecU16(r) => swap!(r),
            Container::VecU32(r) => swap!(r),
            Container::VecU64(r) => swap!(r),
            Container::VecU128(r) => swap!(r),
            Container::VecU256(r) => swap!(r),
            Container::VecI8(r) => swap!(r),
            Container::VecI16(r) => swap!(r),
            Container::VecI32(r) => swap!(r),
            Container::VecI64(r) => swap!(r),
            Container::VecI128(r) => swap!(r),
            Container::VecI256(r) => swap!(r),
            Container::VecBool(r) => swap!(r),
            Container::VecAddress(r) => swap!(r),
            Container::Vec(r) => swap!(r),
            Container::Locals(_) | Container::Struct(_) => unreachable!(),
        }

        self.0.mark_dirty();
        Ok(())
    }

    /// Moves range of elements `[removal_position, removal_position + length)` from vector `from`,
    /// to vector `to`, inserting them starting at the `insert_position`.
    /// In the `from` vector, elements after the selected range are moved left to fill the hole
    /// (i.e. range is removed, while the order of the rest of the elements is kept)
    /// In the `to` vector, elements after the `insert_position` are moved to the right to make space for new elements
    /// (i.e. range is inserted, while the order of the rest of the elements is kept).
    ///
    /// Precondition for this function is that `from` and `to` vectors are required to be distinct
    /// Move will guaranteee that invariant, because it prevents from having two mutable references to the same value.
    pub fn move_range(
        from_self: &Self,
        removal_position: usize,
        length: usize,
        to_self: &Self,
        insert_position: usize,
        type_param: &Type,
    ) -> PartialVMResult<()> {
        let from_c = from_self.0.container();
        let to_c = to_self.0.container();

        // potentially unnecessary as native call should've checked the types already
        // (unlike other vector functions that are bytecodes)
        // TODO: potentially unnecessary, can be removed - as these are only required for
        // bytecode instructions, as types are checked when native functions are called.
        check_elem_layout(type_param, from_c)?;
        check_elem_layout(type_param, to_c)?;

        macro_rules! move_range {
            ($from:expr, $to:expr) => {{
                let mut from_v = $from.borrow_mut();
                let mut to_v = $to.borrow_mut();

                if removal_position.checked_add(length).map_or(true, |end| end > from_v.len())
                        || insert_position > to_v.len() {
                    return Err(PartialVMError::new(StatusCode::VECTOR_OPERATION_ERROR)
                        .with_sub_status(INDEX_OUT_OF_BOUNDS));
                }

                // Short-circuit with faster implementation some of the common cases.
                // This includes all non-direct calls to move-range (i.e. insert/remove/append/split_off inside vector).
                if length == 1 {
                    to_v.insert(insert_position, from_v.remove(removal_position));
                } else if removal_position == 0 && length == from_v.len() && insert_position == to_v.len() {
                    to_v.append(&mut from_v);
                } else if (removal_position + length == from_v.len() && insert_position == to_v.len()) {
                    to_v.append(&mut from_v.split_off(removal_position));
                } else {
                    to_v.splice(insert_position..insert_position, from_v.splice(removal_position..(removal_position + length), []));
                }
            }};
        }

        match (from_c, to_c) {
            (Container::VecU8(from_r), Container::VecU8(to_r)) => move_range!(from_r, to_r),
            (Container::VecU16(from_r), Container::VecU16(to_r)) => move_range!(from_r, to_r),
            (Container::VecU32(from_r), Container::VecU32(to_r)) => move_range!(from_r, to_r),
            (Container::VecU64(from_r), Container::VecU64(to_r)) => move_range!(from_r, to_r),
            (Container::VecU128(from_r), Container::VecU128(to_r)) => move_range!(from_r, to_r),
            (Container::VecU256(from_r), Container::VecU256(to_r)) => move_range!(from_r, to_r),
            (Container::VecI8(from_r), Container::VecI8(to_r)) => move_range!(from_r, to_r),
            (Container::VecI16(from_r), Container::VecI16(to_r)) => move_range!(from_r, to_r),
            (Container::VecI32(from_r), Container::VecI32(to_r)) => move_range!(from_r, to_r),
            (Container::VecI64(from_r), Container::VecI64(to_r)) => move_range!(from_r, to_r),
            (Container::VecI128(from_r), Container::VecI128(to_r)) => move_range!(from_r, to_r),
            (Container::VecI256(from_r), Container::VecI256(to_r)) => move_range!(from_r, to_r),
            (Container::VecBool(from_r), Container::VecBool(to_r)) => move_range!(from_r, to_r),
            (Container::VecAddress(from_r), Container::VecAddress(to_r)) => {
                move_range!(from_r, to_r)
            },
            (Container::Vec(from_r), Container::Vec(to_r)) => move_range!(from_r, to_r),
            (_, _) => unreachable!(),
        }

        from_self.0.mark_dirty();
        to_self.0.mark_dirty();
        Ok(())
    }
}

impl Vector {
    // note(inline): LLVM won't inline it, even with #[inline(always)], and shouldn't, we don't want to bloat execute_code_impl
    pub fn pack(type_param: &Type, elements: Vec<Value>) -> PartialVMResult<Value> {
        let container = match type_param {
            Type::U8 => Value::vector_u8(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::U16 => Value::vector_u16(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::U32 => Value::vector_u32(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::U64 => Value::vector_u64(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::U128 => Value::vector_u128(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::U256 => Value::vector_u256(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I8 => Value::vector_i8(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I16 => Value::vector_i16(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I32 => Value::vector_i32(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I64 => Value::vector_i64(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I128 => Value::vector_i128(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::I256 => Value::vector_i256(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::Bool => Value::vector_bool(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            Type::Address => Value::vector_address(
                elements
                    .into_iter()
                    .map(|v| v.value_as())
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),

            Type::Signer
            | Type::Vector(_)
            | Type::Struct { .. }
            | Type::StructInstantiation { .. }
            | Type::Function { .. } => {
                Value::Container(Container::Vec(Rc::new(RefCell::new(elements))))
            },

            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("invalid type param for vector: {:?}", type_param)),
                )
            },
        };

        Ok(container)
    }

    pub fn unpack_unchecked(self) -> PartialVMResult<Vec<Value>> {
        let elements: Vec<_> = match self.0 {
            Container::VecU8(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u8)
                .collect(),
            Container::VecU16(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u16)
                .collect(),
            Container::VecU32(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u32)
                .collect(),
            Container::VecU64(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u64)
                .collect(),
            Container::VecU128(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u128)
                .collect(),
            Container::VecU256(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::u256)
                .collect(),
            Container::VecI8(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i8)
                .collect(),
            Container::VecI16(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i16)
                .collect(),
            Container::VecI32(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i32)
                .collect(),
            Container::VecI64(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i64)
                .collect(),
            Container::VecI128(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i128)
                .collect(),
            Container::VecI256(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::i256)
                .collect(),
            Container::VecBool(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::bool)
                .collect(),
            Container::VecAddress(r) => take_unique_ownership(r)?
                .into_iter()
                .map(Value::address)
                .collect(),
            Container::Vec(r) => take_unique_ownership(r)?.into_iter().collect(),
            Container::Locals(_) | Container::Struct(_) => {
                return Err(PartialVMError::new_invariant_violation(
                    "Unexpected non-vector container",
                ))
            },
        };
        Ok(elements)
    }

    pub fn unpack(self, expected_num: u64) -> PartialVMResult<Vec<Value>> {
        let elements = self.unpack_unchecked()?;
        if expected_num as usize == elements.len() {
            Ok(elements)
        } else {
            Err(PartialVMError::new(StatusCode::VECTOR_OPERATION_ERROR)
                .with_sub_status(VEC_UNPACK_PARITY_MISMATCH))
        }
    }

    pub fn to_vec_u8(self) -> PartialVMResult<Vec<u8>> {
        check_elem_layout(&Type::U8, &self.0)?;
        if let Container::VecU8(r) = self.0 {
            Ok(take_unique_ownership(r)?.into_iter().collect())
        } else {
            Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("expected vector<u8>".to_string()),
            )
        }
    }
}

/***************************************************************************************
 *
 * Struct Operations
 *
 *   Public APIs for Struct.
 *
 **************************************************************************************/
impl Struct {
    pub fn pack<I: IntoIterator<Item = Value>>(vals: I) -> Self {
        Self {
            fields: vals.into_iter().collect(),
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn unpack(self) -> PartialVMResult<impl Iterator<Item = Value>> {
        Ok(self.fields.into_iter())
    }

    pub fn pack_variant<I: IntoIterator<Item = Value>>(variant: VariantIndex, vals: I) -> Self {
        Self {
            fields: iter::once(Value::u16(variant)).chain(vals).collect(),
        }
    }

    pub fn unpack_variant(
        self,
        variant: VariantIndex,
        variant_to_str: impl Fn(VariantIndex) -> String,
    ) -> PartialVMResult<impl Iterator<Item = Value>> {
        let (tag, values) = self.unpack_with_tag()?;
        if tag == variant {
            Ok(values)
        } else {
            Err(
                PartialVMError::new(StatusCode::STRUCT_VARIANT_MISMATCH).with_message(format!(
                    "expected enum variant {}, found {}",
                    variant_to_str(variant),
                    variant_to_str(tag)
                )),
            )
        }
    }

    pub fn unpack_with_tag(self) -> PartialVMResult<(VariantIndex, impl Iterator<Item = Value>)> {
        let Self { fields } = self;
        if fields.is_empty() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("invalid empty enum variant".to_string()),
            );
        }
        let mut values = fields.into_iter();
        let tag_value = values.next().unwrap();
        let tag = tag_value.value_as::<u16>()?;
        Ok((tag, values))
    }
}

/***************************************************************************************
 *
 * Global Value Operations
 *
 *   Public APIs for GlobalValue. They allow global values to be created from external
 *   source (a.k.a. storage), and references to be taken from them. At the end of the
 *   transaction execution the dirty ones can be identified and wrote back to storage.
 *
 **************************************************************************************/
#[allow(clippy::unnecessary_wraps)]
impl GlobalValueImpl {
    fn expect_struct_fields(value: &Value) -> &Rc<RefCell<Vec<Value>>> {
        match value {
            Value::Container(Container::Struct(fields)) => fields,
            _ => unreachable!("Global values must be structs"),
        }
    }

    fn cached(value: Value, status: GlobalDataStatus) -> Result<Self, (PartialVMError, Value)> {
        match &value {
            Value::Container(Container::Struct(_)) => {
                let status = Rc::new(RefCell::new(status));
                Ok(Self::Cached { value, status })
            },
            _ => Err((
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("failed to publish cached: not a resource".to_string()),
                value,
            )),
        }
    }

    fn fresh(value: Value) -> Result<Self, (PartialVMError, Value)> {
        match &value {
            Value::Container(Container::Struct(_)) => Ok(Self::Fresh { value }),
            _ => Err((
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("failed to publish fresh: not a resource".to_string()),
                value,
            )),
        }
    }

    fn move_from(&mut self) -> PartialVMResult<Value> {
        let value = match self {
            Self::None | Self::Deleted => {
                return Err(PartialVMError::new(StatusCode::MISSING_DATA))
            },
            Self::Fresh { .. } => match mem::replace(self, Self::None) {
                Self::Fresh { value } => value,
                _ => unreachable!(),
            },
            Self::Cached { .. } => match mem::replace(self, Self::Deleted) {
                Self::Cached { value, .. } => value,
                _ => unreachable!(),
            },
        };
        let fields = Self::expect_struct_fields(&value);
        if Rc::strong_count(fields) != 1 {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("moving global resource with dangling reference".to_string())
                    .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EREFERENCE_COUNTING_FAILURE),
            );
        }
        Ok(value)
    }

    fn move_to(&mut self, val: Value) -> Result<(), (PartialVMError, Value)> {
        match self {
            Self::Fresh { .. } | Self::Cached { .. } => {
                return Err((
                    PartialVMError::new(StatusCode::RESOURCE_ALREADY_EXISTS),
                    val,
                ))
            },
            Self::None => *self = Self::fresh(val)?,
            Self::Deleted => *self = Self::cached(val, GlobalDataStatus::Dirty)?,
        }
        Ok(())
    }

    fn exists(&self) -> bool {
        match self {
            Self::Fresh { .. } | Self::Cached { .. } => true,
            Self::None | Self::Deleted => false,
        }
    }

    fn borrow_global(&self) -> PartialVMResult<Value> {
        match self {
            Self::None | Self::Deleted => Err(PartialVMError::new(StatusCode::MISSING_DATA)),
            Self::Fresh { value } => {
                let fields = Self::expect_struct_fields(value);
                Ok(Value::ContainerRef(ContainerRef::Local(Container::Struct(
                    Rc::clone(fields),
                ))))
            },
            Self::Cached { value, status } => {
                let fields = Self::expect_struct_fields(value);
                Ok(Value::ContainerRef(ContainerRef::Global {
                    container: Container::Struct(Rc::clone(fields)),
                    status: Rc::clone(status),
                }))
            },
        }
    }

    fn into_effect(self) -> Option<Op<Value>> {
        match self {
            Self::None => None,
            Self::Deleted => Some(Op::Delete),
            Self::Fresh { value } => Some(Op::New(value)),
            Self::Cached { value, status } => match &*status.borrow() {
                GlobalDataStatus::Dirty => Some(Op::Modify(value)),
                GlobalDataStatus::Clean => None,
            },
        }
    }

    fn is_mutated(&self) -> bool {
        match self {
            Self::None => false,
            Self::Deleted => true,
            Self::Fresh { .. } => true,
            Self::Cached { status, .. } => match &*status.borrow() {
                GlobalDataStatus::Dirty => true,
                GlobalDataStatus::Clean => false,
            },
        }
    }
}

impl GlobalValue {
    pub fn none() -> Self {
        Self(GlobalValueImpl::None)
    }

    pub fn cached(val: Value) -> PartialVMResult<Self> {
        Ok(Self(
            GlobalValueImpl::cached(val, GlobalDataStatus::Clean).map_err(|(err, _)| err)?,
        ))
    }

    pub fn move_from(&mut self) -> PartialVMResult<Value> {
        self.0.move_from()
    }

    pub fn move_to(&mut self, val: Value) -> Result<(), (PartialVMError, Value)> {
        self.0.move_to(val)
    }

    pub fn borrow_global(&self) -> PartialVMResult<Value> {
        self.0.borrow_global()
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    pub fn into_effect(self) -> Option<Op<Value>> {
        self.0.into_effect()
    }

    pub fn into_effect_with_layout(
        self,
        layout: TriompheArc<MoveTypeLayout>,
    ) -> Option<Op<(Value, TriompheArc<MoveTypeLayout>)>> {
        self.0.into_effect().map(|op| op.map(|v| (v, layout)))
    }

    pub fn effect(&self) -> Option<Op<&Value>> {
        match &self.0 {
            GlobalValueImpl::None => None,
            GlobalValueImpl::Deleted => Some(Op::Delete),
            GlobalValueImpl::Fresh { value } => Some(Op::New(value)),
            GlobalValueImpl::Cached { value, status } => match &*status.borrow() {
                GlobalDataStatus::Dirty => Some(Op::Modify(value)),
                GlobalDataStatus::Clean => None,
            },
        }
    }

    pub fn is_mutated(&self) -> bool {
        self.0.is_mutated()
    }
}

/***************************************************************************************
*
* Debug
*
*   Implementation of the Debug trait for VM Values.
*
**************************************************************************************/

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "Invalid"),

            Self::U8(x) => write!(f, "U8({:?})", x),
            Self::U16(x) => write!(f, "U16({:?})", x),
            Self::U32(x) => write!(f, "U32({:?})", x),
            Self::U64(x) => write!(f, "U64({:?})", x),
            Self::U128(x) => write!(f, "U128({:?})", x),
            Self::U256(x) => write!(f, "U256({:?})", x),
            Self::I8(x) => write!(f, "I8({:?})", x),
            Self::I16(x) => write!(f, "I16({:?})", x),
            Self::I32(x) => write!(f, "I32({:?})", x),
            Self::I64(x) => write!(f, "I64({:?})", x),
            Self::I128(x) => write!(f, "I128({:?})", x),
            Self::I256(x) => write!(f, "I256({:?})", x),
            Self::Bool(x) => write!(f, "Bool({:?})", x),
            Self::Address(addr) => write!(f, "Address({:?})", addr),

            Self::Container(r) => write!(f, "Container({:?})", r),

            Self::ContainerRef(r) => write!(f, "ContainerRef({:?})", r),
            Self::IndexedRef(r) => write!(f, "IndexedRef({:?})", r),

            Self::ClosureValue(c) => write!(f, "Function({:?})", c),

            // Debug information must be deterministic, so we cannot print
            // inner fields.
            Self::DelayedFieldID { .. } => write!(f, "Delayed(?)"),
        }
    }
}

/***************************************************************************************
*
* Display
*
*   Implementation of the Display trait for VM Values. These are supposed to be more
*   friendly & readable than the generated Debug dump.
*
**************************************************************************************/

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "Invalid"),

            Self::U8(x) => write!(f, "U8({})", x),
            Self::U16(x) => write!(f, "U16({})", x),
            Self::U32(x) => write!(f, "U32({})", x),
            Self::U64(x) => write!(f, "U64({})", x),
            Self::U128(x) => write!(f, "U128({})", x),
            Self::U256(x) => write!(f, "U256({})", x),
            Self::I8(x) => write!(f, "I8({})", x),
            Self::I16(x) => write!(f, "I16({})", x),
            Self::I32(x) => write!(f, "I32({})", x),
            Self::I64(x) => write!(f, "I64({})", x),
            Self::I128(x) => write!(f, "I128({})", x),
            Self::I256(x) => write!(f, "I256({})", x),
            Self::Bool(x) => write!(f, "{}", x),
            Self::Address(addr) => write!(f, "Address({})", addr.short_str_lossless()),

            Self::Container(r) => write!(f, "{}", r),

            Self::ContainerRef(r) => write!(f, "{}", r),
            Self::IndexedRef(r) => write!(f, "{}", r),

            Self::ClosureValue(c) => write!(f, "{}", c),

            // Display information must be deterministic, so we cannot print
            // inner fields.
            Self::DelayedFieldID { .. } => write!(f, "Delayed(?)"),
        }
    }
}

fn display_list_of_items<T, I>(items: I, f: &mut fmt::Formatter) -> fmt::Result
where
    T: Display,
    I: IntoIterator<Item = T>,
{
    write!(f, "[")?;
    let mut items = items.into_iter();
    if let Some(x) = items.next() {
        write!(f, "{}", x)?;
        for x in items {
            write!(f, ", {}", x)?;
        }
    }
    write!(f, "]")
}

impl Display for ContainerRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Local(_) => write!(f, "(&container)"),
            Self::Global {
                status,
                container: _,
            } => write!(f, "(&container -- {:?})", &*status.borrow()),
        }
    }
}

impl Display for IndexedRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", self.container_ref, self.idx)
    }
}

impl Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(container: ")?;

        match self {
            Self::Locals(r) | Self::Vec(r) | Self::Struct(r) => {
                display_list_of_items(r.borrow().iter(), f)
            },
            Self::VecU8(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecU16(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecU32(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecU64(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecU128(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecU256(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI8(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI16(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI32(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI64(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI128(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecI256(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecBool(r) => display_list_of_items(r.borrow().iter(), f),
            Self::VecAddress(r) => display_list_of_items(r.borrow().iter(), f),
        }?;

        write!(f, ")")
    }
}

impl Display for Locals {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .borrow()
                .iter()
                .enumerate()
                .map(|(idx, val)| format!("[{}] {}", idx, val))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[allow(dead_code)]
pub mod debug {
    use super::*;
    use std::fmt::Write;

    fn print_delayed_value<B: Write>(buf: &mut B) -> PartialVMResult<()> {
        debug_write!(buf, "<?>")
    }

    fn print_invalid<B: Write>(buf: &mut B) -> PartialVMResult<()> {
        debug_write!(buf, "-")
    }

    fn print_u8<B: Write>(buf: &mut B, x: &u8) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_u16<B: Write>(buf: &mut B, x: &u16) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_u32<B: Write>(buf: &mut B, x: &u32) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_u64<B: Write>(buf: &mut B, x: &u64) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_u128<B: Write>(buf: &mut B, x: &u128) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_u256<B: Write>(buf: &mut B, x: &int256::U256) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i8<B: Write>(buf: &mut B, x: &i8) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i16<B: Write>(buf: &mut B, x: &i16) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i32<B: Write>(buf: &mut B, x: &i32) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i64<B: Write>(buf: &mut B, x: &i64) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i128<B: Write>(buf: &mut B, x: &i128) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_i256<B: Write>(buf: &mut B, x: &int256::I256) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_bool<B: Write>(buf: &mut B, x: &bool) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x)
    }

    fn print_address<B: Write>(buf: &mut B, x: &AccountAddress) -> PartialVMResult<()> {
        debug_write!(buf, "{}", x.to_hex())
    }

    fn print_closure<B: Write>(buf: &mut B, c: &Closure) -> PartialVMResult<()> {
        debug_write!(buf, "{}", c)
    }

    fn print_value_impl<B: Write>(buf: &mut B, val: &Value) -> PartialVMResult<()> {
        match val {
            Value::Invalid => print_invalid(buf),

            Value::U8(x) => print_u8(buf, x),
            Value::U16(x) => print_u16(buf, x),
            Value::U32(x) => print_u32(buf, x),
            Value::U64(x) => print_u64(buf, x),
            Value::U128(x) => print_u128(buf, x),
            Value::U256(x) => print_u256(buf, x),
            Value::I8(x) => print_i8(buf, x),
            Value::I16(x) => print_i16(buf, x),
            Value::I32(x) => print_i32(buf, x),
            Value::I64(x) => print_i64(buf, x),
            Value::I128(x) => print_i128(buf, x),
            Value::I256(x) => print_i256(buf, x),
            Value::Bool(x) => print_bool(buf, x),
            Value::Address(x) => print_address(buf, x),

            Value::Container(c) => print_container(buf, c),

            Value::ContainerRef(r) => print_container_ref(buf, r),
            Value::IndexedRef(r) => print_indexed_ref(buf, r),

            Value::ClosureValue(c) => print_closure(buf, c),

            Value::DelayedFieldID { .. } => print_delayed_value(buf),
        }
    }

    fn print_list<'a, B, I, X, F>(
        buf: &mut B,
        begin: &str,
        items: I,
        print: F,
        end: &str,
    ) -> PartialVMResult<()>
    where
        B: Write,
        X: 'a,
        I: IntoIterator<Item = &'a X>,
        F: Fn(&mut B, &X) -> PartialVMResult<()>,
    {
        debug_write!(buf, "{}", begin)?;
        let mut it = items.into_iter();
        if let Some(x) = it.next() {
            print(buf, x)?;
            for x in it {
                debug_write!(buf, ", ")?;
                print(buf, x)?;
            }
        }
        debug_write!(buf, "{}", end)?;
        Ok(())
    }

    fn print_container<B: Write>(buf: &mut B, c: &Container) -> PartialVMResult<()> {
        match c {
            Container::Vec(r) => print_list(buf, "[", r.borrow().iter(), print_value_impl, "]"),

            Container::Struct(r) => {
                print_list(buf, "{ ", r.borrow().iter(), print_value_impl, " }")
            },

            Container::VecU8(r) => print_list(buf, "[", r.borrow().iter(), print_u8, "]"),
            Container::VecU16(r) => print_list(buf, "[", r.borrow().iter(), print_u16, "]"),
            Container::VecU32(r) => print_list(buf, "[", r.borrow().iter(), print_u32, "]"),
            Container::VecU64(r) => print_list(buf, "[", r.borrow().iter(), print_u64, "]"),
            Container::VecU128(r) => print_list(buf, "[", r.borrow().iter(), print_u128, "]"),
            Container::VecU256(r) => print_list(buf, "[", r.borrow().iter(), print_u256, "]"),
            Container::VecI8(r) => print_list(buf, "[", r.borrow().iter(), print_i8, "]"),
            Container::VecI16(r) => print_list(buf, "[", r.borrow().iter(), print_i16, "]"),
            Container::VecI32(r) => print_list(buf, "[", r.borrow().iter(), print_i32, "]"),
            Container::VecI64(r) => print_list(buf, "[", r.borrow().iter(), print_i64, "]"),
            Container::VecI128(r) => print_list(buf, "[", r.borrow().iter(), print_i128, "]"),
            Container::VecI256(r) => print_list(buf, "[", r.borrow().iter(), print_i256, "]"),
            Container::VecBool(r) => print_list(buf, "[", r.borrow().iter(), print_bool, "]"),
            Container::VecAddress(r) => print_list(buf, "[", r.borrow().iter(), print_address, "]"),

            Container::Locals(_) => Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )
            .with_message("debug print - invalid container: Locals".to_string())),
        }
    }

    fn print_container_ref<B: Write>(buf: &mut B, r: &ContainerRef) -> PartialVMResult<()> {
        debug_write!(buf, "(&) ")?;
        print_container(buf, r.container())
    }

    fn print_slice_elem<B, X, F>(buf: &mut B, v: &[X], idx: usize, print: F) -> PartialVMResult<()>
    where
        B: Write,
        F: FnOnce(&mut B, &X) -> PartialVMResult<()>,
    {
        match v.get(idx) {
            Some(x) => print(buf, x),
            None => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("ref index out of bounds".to_string()),
            ),
        }
    }

    fn print_indexed_ref<B: Write>(buf: &mut B, r: &IndexedRef) -> PartialVMResult<()> {
        let idx = r.idx;
        match r.container_ref.container() {
            Container::Locals(r) | Container::Vec(r) | Container::Struct(r) => {
                print_slice_elem(buf, &r.borrow(), idx, print_value_impl)
            },

            Container::VecU8(r) => print_slice_elem(buf, &r.borrow(), idx, print_u8),
            Container::VecU16(r) => print_slice_elem(buf, &r.borrow(), idx, print_u16),
            Container::VecU32(r) => print_slice_elem(buf, &r.borrow(), idx, print_u32),
            Container::VecU64(r) => print_slice_elem(buf, &r.borrow(), idx, print_u64),
            Container::VecU128(r) => print_slice_elem(buf, &r.borrow(), idx, print_u128),
            Container::VecU256(r) => print_slice_elem(buf, &r.borrow(), idx, print_u256),
            Container::VecI8(r) => print_slice_elem(buf, &r.borrow(), idx, print_i8),
            Container::VecI16(r) => print_slice_elem(buf, &r.borrow(), idx, print_i16),
            Container::VecI32(r) => print_slice_elem(buf, &r.borrow(), idx, print_i32),
            Container::VecI64(r) => print_slice_elem(buf, &r.borrow(), idx, print_i64),
            Container::VecI128(r) => print_slice_elem(buf, &r.borrow(), idx, print_i128),
            Container::VecI256(r) => print_slice_elem(buf, &r.borrow(), idx, print_i256),
            Container::VecBool(r) => print_slice_elem(buf, &r.borrow(), idx, print_bool),
            Container::VecAddress(r) => print_slice_elem(buf, &r.borrow(), idx, print_address),
        }
    }

    pub fn print_locals<B: Write>(
        buf: &mut B,
        locals: &Locals,
        compact: bool,
    ) -> PartialVMResult<()> {
        // REVIEW: The number of spaces in the indent is currently hard coded.
        for (idx, val) in locals.0.borrow().iter().enumerate() {
            if compact && matches!(val, Value::Invalid) {
                continue;
            }
            debug_write!(buf, "            [{}] ", idx)?;
            print_value_impl(buf, val)?;
            debug_writeln!(buf)?;
        }
        Ok(())
    }

    pub fn print_value<B: Write>(buf: &mut B, val: &Value) -> PartialVMResult<()> {
        print_value_impl(buf, val)
    }
}

/***************************************************************************************
 *
 * Serialization & Deserialization
 *
 *   BCS implementation for VM values. Note although values are represented as Rust
 *   enums that carry type info in the tags, we should NOT rely on them for
 *   serialization:
 *     1) Depending on the specific internal representation, it may be impossible to
 *        reconstruct the layout from a value. For example, one cannot tell if a general
 *        container is a struct or a value.
 *     2) Even if 1) is not a problem at a certain time, we may change to a different
 *        internal representation that breaks the 1-1 mapping. Extremely speaking, if
 *        we switch to untagged unions one day, none of the type info will be carried
 *        by the value.
 *
 *   Therefore the appropriate & robust way to implement serialization & deserialization
 *   is to involve an explicit representation of the type layout.
 *
 **************************************************************************************/

// Wrapper around value with additional information which can be used by the
// serializer.
pub(crate) struct SerializationReadyValue<'c, 'l, 'v, L, V> {
    // Contains the current (possibly custom) serialization context.
    pub(crate) ctx: &'c ValueSerDeContext<'c>,
    // Layout for guiding serialization.
    pub(crate) layout: &'l L,
    // Value to serialize.
    pub(crate) value: &'v V,
    pub(crate) depth: u64,
}

fn invariant_violation<S: serde::Serializer>(message: String) -> S::Error {
    S::Error::custom(
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(message),
    )
}

impl serde::Serialize for SerializationReadyValue<'_, '_, '_, MoveTypeLayout, Value> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use MoveTypeLayout as L;

        self.ctx.check_depth(self.depth).map_err(S::Error::custom)?;
        match (self.layout, self.value) {
            // Primitive types.
            (L::U8, Value::U8(x)) => serializer.serialize_u8(*x),
            (L::U16, Value::U16(x)) => serializer.serialize_u16(*x),
            (L::U32, Value::U32(x)) => serializer.serialize_u32(*x),
            (L::U64, Value::U64(x)) => serializer.serialize_u64(*x),
            (L::U128, Value::U128(x)) => serializer.serialize_u128(*x),
            (L::U256, Value::U256(x)) => x.serialize(serializer),
            (L::I8, Value::I8(x)) => serializer.serialize_i8(*x),
            (L::I16, Value::I16(x)) => serializer.serialize_i16(*x),
            (L::I32, Value::I32(x)) => serializer.serialize_i32(*x),
            (L::I64, Value::I64(x)) => serializer.serialize_i64(*x),
            (L::I128, Value::I128(x)) => serializer.serialize_i128(*x),
            (L::I256, Value::I256(x)) => x.serialize(serializer),
            (L::Bool, Value::Bool(x)) => serializer.serialize_bool(*x),
            (L::Address, Value::Address(x)) => x.serialize(serializer),

            // Structs.
            (L::Struct(struct_layout), Value::Container(Container::Struct(r))) => {
                (SerializationReadyValue {
                    ctx: self.ctx,
                    layout: struct_layout,
                    value: &*r.borrow(),
                    // Note: for struct, we increment depth for fields in the corresponding
                    // serializer.
                    depth: self.depth,
                })
                .serialize(serializer)
            },

            // Functions.
            (L::Function, Value::ClosureValue(clos)) => SerializationReadyValue {
                ctx: self.ctx,
                layout: &(),
                value: clos,
                // Note: for functions, we increment depth for captured arguments in the
                // corresponding serializer.
                depth: self.depth,
            }
            .serialize(serializer),

            // Vectors.
            (L::Vector(layout), Value::Container(c)) => {
                let layout = layout.as_ref();
                match (layout, c) {
                    (L::U8, Container::VecU8(r)) => r.borrow().serialize(serializer),
                    (L::U16, Container::VecU16(r)) => r.borrow().serialize(serializer),
                    (L::U32, Container::VecU32(r)) => r.borrow().serialize(serializer),
                    (L::U64, Container::VecU64(r)) => r.borrow().serialize(serializer),
                    (L::U128, Container::VecU128(r)) => r.borrow().serialize(serializer),
                    (L::U256, Container::VecU256(r)) => r.borrow().serialize(serializer),
                    (L::I8, Container::VecI8(r)) => r.borrow().serialize(serializer),
                    (L::I16, Container::VecI16(r)) => r.borrow().serialize(serializer),
                    (L::I32, Container::VecI32(r)) => r.borrow().serialize(serializer),
                    (L::I64, Container::VecI64(r)) => r.borrow().serialize(serializer),
                    (L::I128, Container::VecI128(r)) => r.borrow().serialize(serializer),
                    (L::I256, Container::VecI256(r)) => r.borrow().serialize(serializer),
                    (L::Bool, Container::VecBool(r)) => r.borrow().serialize(serializer),
                    (L::Address, Container::VecAddress(r)) => r.borrow().serialize(serializer),
                    (_, Container::Vec(r)) => {
                        let v = r.borrow();
                        let mut t = serializer.serialize_seq(Some(v.len()))?;
                        for value in v.iter() {
                            t.serialize_element(&SerializationReadyValue {
                                ctx: self.ctx,
                                layout,
                                value,
                                depth: self.depth + 1,
                            })?;
                        }
                        t.end()
                    },
                    (layout, container) => Err(invariant_violation::<S>(format!(
                        "cannot serialize container {:?} as {:?}",
                        container, layout
                    ))),
                }
            },

            // Signer.
            (L::Signer, Value::Container(Container::Struct(r))) => {
                if self.ctx.legacy_signer {
                    // Only allow serialization of master signer.
                    if *r.borrow()[0].as_value_ref::<u16>().map_err(|_| {
                        invariant_violation::<S>(format!(
                            "First field of a signer needs to be an enum descriminator, got {:?}",
                            self.value
                        ))
                    })? != MASTER_SIGNER_VARIANT
                    {
                        return Err(S::Error::custom(PartialVMError::new(StatusCode::ABORTED)));
                    }
                    r.borrow()
                        .get(MASTER_ADDRESS_FIELD_OFFSET)
                        .ok_or_else(|| {
                            invariant_violation::<S>(format!(
                                "cannot serialize container {:?} as {:?}",
                                self.value, self.layout
                            ))
                        })?
                        .as_value_ref::<AccountAddress>()
                        .map_err(|_| {
                            invariant_violation::<S>(format!(
                                "cannot serialize container {:?} as {:?}",
                                self.value, self.layout
                            ))
                        })?
                        .serialize(serializer)
                } else {
                    (SerializationReadyValue {
                        ctx: self.ctx,
                        layout: &MoveStructLayout::signer_serialization_layout(),
                        value: &*r.borrow(),
                        depth: self.depth,
                    })
                    .serialize(serializer)
                }
            },

            // Delayed values. For their serialization, we must have custom
            // serialization available, otherwise an error is returned.
            (L::Native(kind, layout), Value::DelayedFieldID { id }) => {
                match &self.ctx.delayed_fields_extension {
                    Some(delayed_fields_extension) => {
                        delayed_fields_extension
                            .inc_and_check_delayed_fields_count()
                            .map_err(S::Error::custom)?;

                        let value = match delayed_fields_extension.mapping {
                            Some(mapping) => mapping
                                .identifier_to_value(layout, *id)
                                .map_err(|e| S::Error::custom(format!("{}", e)))?,
                            None => id.try_into_move_value(layout).map_err(|_| {
                                S::Error::custom(format!(
                                    "Custom serialization failed for {:?} with layout {}",
                                    kind, layout
                                ))
                            })?,
                        };

                        // The resulting value should not contain any delayed fields, we disallow
                        // this by using a context without the delayed field extension.
                        let ctx = self.ctx.clone_without_delayed_fields();
                        let value = SerializationReadyValue {
                            ctx: &ctx,
                            layout: layout.as_ref(),
                            value: &value,
                            depth: self.depth,
                        };
                        value.serialize(serializer)
                    },
                    None => {
                        // If no delayed field extension, it is not known how the delayed value
                        // should be serialized. So, just return an error.
                        Err(invariant_violation::<S>(format!(
                            "no custom serializer for delayed value ({:?}) with layout {}",
                            kind, layout
                        )))
                    },
                }
            },

            // All other cases should not be possible.
            (layout, value) => Err(invariant_violation::<S>(format!(
                "cannot serialize value {:?} as {:?}",
                value, layout
            ))),
        }
    }
}

impl serde::Serialize for SerializationReadyValue<'_, '_, '_, MoveStructLayout, Vec<Value>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut values = self.value.as_slice();
        if let Some((tag, variant_layouts)) = try_get_variant_field_layouts(self.layout, values) {
            let tag_idx = tag as usize;
            let variant_tag = tag_idx as u32;
            let variant_names = value::variant_name_placeholder((tag + 1) as usize)
                .map_err(|e| serde::ser::Error::custom(format!("{}", e)))?;
            let variant_name = variant_names[tag_idx];
            values = &values[1..];
            if variant_layouts.len() != values.len() {
                return Err(invariant_violation::<S>(format!(
                    "cannot serialize struct value {:?} as {:?} -- number of fields mismatch",
                    self.value, self.layout
                )));
            }
            match values.len() {
                0 => serializer.serialize_unit_variant(
                    value::MOVE_ENUM_NAME,
                    variant_tag,
                    variant_name,
                ),
                1 => serializer.serialize_newtype_variant(
                    value::MOVE_ENUM_NAME,
                    variant_tag,
                    variant_name,
                    &SerializationReadyValue {
                        ctx: self.ctx,
                        layout: &variant_layouts[0],
                        value: &values[0],
                        depth: self.depth + 1,
                    },
                ),
                _ => {
                    let mut t = serializer.serialize_tuple_variant(
                        value::MOVE_ENUM_NAME,
                        variant_tag,
                        variant_name,
                        values.len(),
                    )?;
                    for (layout, value) in variant_layouts.iter().zip(values) {
                        t.serialize_field(&SerializationReadyValue {
                            ctx: self.ctx,
                            layout,
                            value,
                            depth: self.depth + 1,
                        })?
                    }
                    t.end()
                },
            }
        } else {
            let field_layouts = self.layout.fields(None);
            let mut t = serializer.serialize_tuple(values.len())?;
            if field_layouts.len() != values.len() {
                return Err(invariant_violation::<S>(format!(
                    "cannot serialize struct value {:?} as {:?} -- number of fields mismatch",
                    self.value, self.layout
                )));
            }
            for (field_layout, value) in field_layouts.iter().zip(values.iter()) {
                t.serialize_element(&SerializationReadyValue {
                    ctx: self.ctx,
                    layout: field_layout,
                    value,
                    depth: self.depth + 1,
                })?;
            }
            t.end()
        }
    }
}

// Seed used by deserializer to ensure there is information about the value
// being deserialized.
pub(crate) struct DeserializationSeed<'c, L> {
    // Holds extensions external to the deserializer.
    pub(crate) ctx: &'c ValueSerDeContext<'c>,
    // Layout to guide deserialization.
    pub(crate) layout: L,
}

impl<'d> serde::de::DeserializeSeed<'d> for DeserializationSeed<'_, &MoveTypeLayout> {
    type Value = Value;

    fn deserialize<D: serde::de::Deserializer<'d>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        use MoveTypeLayout as L;

        match self.layout {
            // Primitive types.
            L::Bool => bool::deserialize(deserializer).map(Value::bool),
            L::U8 => u8::deserialize(deserializer).map(Value::u8),
            L::U16 => u16::deserialize(deserializer).map(Value::u16),
            L::U32 => u32::deserialize(deserializer).map(Value::u32),
            L::U64 => u64::deserialize(deserializer).map(Value::u64),
            L::U128 => u128::deserialize(deserializer).map(Value::u128),
            L::U256 => int256::U256::deserialize(deserializer).map(Value::u256),
            L::I8 => i8::deserialize(deserializer).map(Value::i8),
            L::I16 => i16::deserialize(deserializer).map(Value::i16),
            L::I32 => i32::deserialize(deserializer).map(Value::i32),
            L::I64 => i64::deserialize(deserializer).map(Value::i64),
            L::I128 => i128::deserialize(deserializer).map(Value::i128),
            L::I256 => int256::I256::deserialize(deserializer).map(Value::i256),
            L::Address => AccountAddress::deserialize(deserializer).map(Value::address),
            L::Signer => {
                if self.ctx.legacy_signer {
                    Err(D::Error::custom(
                        "Cannot deserialize signer into value".to_string(),
                    ))
                } else {
                    let seed = DeserializationSeed {
                        ctx: self.ctx,
                        layout: &MoveStructLayout::signer_serialization_layout(),
                    };
                    Ok(Value::struct_(seed.deserialize(deserializer)?))
                }
            },

            // Structs.
            L::Struct(struct_layout) => {
                let seed = DeserializationSeed {
                    ctx: self.ctx,
                    layout: struct_layout,
                };
                Ok(Value::struct_(seed.deserialize(deserializer)?))
            },

            // Vectors.
            L::Vector(layout) => Ok(match layout.as_ref() {
                L::U8 => Value::vector_u8(Vec::deserialize(deserializer)?),
                L::U16 => Value::vector_u16(Vec::deserialize(deserializer)?),
                L::U32 => Value::vector_u32(Vec::deserialize(deserializer)?),
                L::U64 => Value::vector_u64(Vec::deserialize(deserializer)?),
                L::U128 => Value::vector_u128(Vec::deserialize(deserializer)?),
                L::U256 => Value::vector_u256(Vec::deserialize(deserializer)?),
                L::I8 => Value::vector_i8(Vec::deserialize(deserializer)?),
                L::I16 => Value::vector_i16(Vec::deserialize(deserializer)?),
                L::I32 => Value::vector_i32(Vec::deserialize(deserializer)?),
                L::I64 => Value::vector_i64(Vec::deserialize(deserializer)?),
                L::I128 => Value::vector_i128(Vec::deserialize(deserializer)?),
                L::I256 => Value::vector_i256(Vec::deserialize(deserializer)?),
                L::Bool => Value::vector_bool(Vec::deserialize(deserializer)?),
                L::Address => Value::vector_address(Vec::deserialize(deserializer)?),
                layout => {
                    let seed = DeserializationSeed {
                        ctx: self.ctx,
                        layout,
                    };
                    let vector = deserializer.deserialize_seq(VectorElementVisitor(seed))?;
                    Value::Container(Container::Vec(Rc::new(RefCell::new(vector))))
                },
            }),

            // Functions
            L::Function => {
                let seed = DeserializationSeed {
                    ctx: self.ctx,
                    layout: (),
                };
                let closure = deserializer.deserialize_seq(ClosureVisitor(seed))?;
                Ok(Value::ClosureValue(closure))
            },

            // Delayed values should always use custom deserialization.
            L::Native(kind, layout) => {
                match &self.ctx.delayed_fields_extension {
                    Some(delayed_fields_extension) => {
                        delayed_fields_extension
                            .inc_and_check_delayed_fields_count()
                            .map_err(D::Error::custom)?;

                        let value = DeserializationSeed {
                            ctx: &self.ctx.clone_without_delayed_fields(),
                            layout: layout.as_ref(),
                        }
                        .deserialize(deserializer)?;
                        let id = match delayed_fields_extension.mapping {
                            Some(mapping) => mapping
                                .value_to_identifier(kind, layout, value)
                                .map_err(|e| D::Error::custom(format!("{}", e)))?,
                            None => {
                                let (id, _) =
                                    DelayedFieldID::try_from_move_value(layout, value, &())
                                        .map_err(|_| {
                                            D::Error::custom(format!(
                                        "Custom deserialization failed for {:?} with layout {}",
                                        kind, layout
                                    ))
                                        })?;
                                id
                            },
                        };
                        Ok(Value::delayed_value(id))
                    },
                    None => {
                        // If no custom deserializer, it is not known how the
                        // delayed value should be deserialized. Just like with
                        // serialization, we return an error.
                        Err(D::Error::custom(
                            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                                .with_message(format!(
                                    "no custom deserializer for native value ({:?}) with layout {}",
                                    kind, layout
                                )),
                        ))
                    },
                }
            },
        }
    }
}

impl<'d> serde::de::DeserializeSeed<'d> for DeserializationSeed<'_, &MoveStructLayout> {
    type Value = Struct;

    fn deserialize<D: serde::de::Deserializer<'d>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        match &self.layout {
            MoveStructLayout::Runtime(field_layouts) => {
                let fields = deserializer.deserialize_tuple(
                    field_layouts.len(),
                    StructFieldVisitor(self.ctx, field_layouts),
                )?;
                Ok(Struct::pack(fields))
            },
            MoveStructLayout::RuntimeVariants(variants) => {
                if variants.len() > (u16::MAX as usize) {
                    return Err(D::Error::custom("variant count out of range"));
                }
                let variant_names = value::variant_name_placeholder(variants.len())
                    .map_err(|e| D::Error::custom(format!("{}", e)))?;
                let fields = deserializer.deserialize_enum(
                    value::MOVE_ENUM_NAME,
                    variant_names,
                    StructVariantVisitor(self.ctx, variants),
                )?;
                Ok(Struct::pack(fields))
            },
            MoveStructLayout::WithFields(_)
            | MoveStructLayout::WithTypes { .. }
            | MoveStructLayout::WithVariants(_) => {
                Err(D::Error::custom("cannot deserialize from decorated type"))
            },
        }
    }
}

struct VectorElementVisitor<'c, 'l>(DeserializationSeed<'c, &'l MoveTypeLayout>);

impl<'d, 'c, 'l> serde::de::Visitor<'d> for VectorElementVisitor<'c, 'l> {
    type Value = Vec<Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Vector")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut vals = Vec::new();
        while let Some(elem) = seq.next_element_seed(DeserializationSeed {
            ctx: self.0.ctx,
            layout: self.0.layout,
        })? {
            vals.push(elem)
        }
        Ok(vals)
    }
}

struct StructFieldVisitor<'c, 'l>(&'c ValueSerDeContext<'c>, &'l [MoveTypeLayout]);

impl<'d, 'c, 'l> serde::de::Visitor<'d> for StructFieldVisitor<'c, 'l> {
    type Value = Vec<Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut val = Vec::new();
        for (i, field_layout) in self.1.iter().enumerate() {
            if let Some(elem) = seq.next_element_seed(DeserializationSeed {
                ctx: self.0,
                layout: field_layout,
            })? {
                val.push(elem)
            } else {
                return Err(A::Error::invalid_length(i, &self));
            }
        }
        Ok(val)
    }
}

struct StructVariantVisitor<'c, 'l>(&'c ValueSerDeContext<'c>, &'l [Vec<MoveTypeLayout>]);

impl<'d, 'c, 'l> serde::de::Visitor<'d> for StructVariantVisitor<'c, 'l> {
    type Value = Vec<Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Variant")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'d>,
    {
        let (tag, rest) = data.variant()?;
        if tag as usize >= self.1.len() {
            Err(A::Error::invalid_length(0, &self))
        } else {
            let mut values = vec![Value::u16(tag)];
            let fields = &self.1[tag as usize];
            match fields.len() {
                0 => {
                    rest.unit_variant()?;
                    Ok(values)
                },
                1 => {
                    values.push(rest.newtype_variant_seed(DeserializationSeed {
                        ctx: self.0,
                        layout: &fields[0],
                    })?);
                    Ok(values)
                },
                _ => {
                    values.append(
                        &mut rest
                            .tuple_variant(fields.len(), StructFieldVisitor(self.0, fields))?,
                    );
                    Ok(values)
                },
            }
        }
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut val = Vec::new();

        // First deserialize the variant tag.
        // Note this is actually directly serialized as u16, but this is equivalent
        // to MoveTypeLayout::U16, so we can reuse the custom deserializer seed.
        let variant_tag = match seq.next_element_seed(DeserializationSeed {
            ctx: self.0,
            layout: &MoveTypeLayout::U16,
        })? {
            Some(elem) => {
                let variant_tag = if let Ok(tag) = elem.value_as::<u16>() {
                    tag as usize
                } else {
                    // This shouldn't happen but be robust and produce an error
                    return Err(A::Error::invalid_value(
                        Unexpected::Other("not a valid enum variant tag"),
                        &self,
                    ));
                };
                if variant_tag >= self.1.len() {
                    return Err(A::Error::invalid_value(Unexpected::StructVariant, &self));
                }
                variant_tag
            },
            None => return Err(A::Error::invalid_length(0, &self)),
        };

        val.push(Value::u16(variant_tag as u16));

        // Based on the validated variant tag, we know the field types
        for (i, field_layout) in self.1[variant_tag].iter().enumerate() {
            if let Some(elem) = seq.next_element_seed(DeserializationSeed {
                ctx: self.0,
                layout: field_layout,
            })? {
                val.push(elem)
            } else {
                return Err(A::Error::invalid_length(i, &self));
            }
        }
        Ok(val)
    }
}

/***************************************************************************************
*
* Constants
*
*   Implementation of deserialization of constant data into a runtime value
*
**************************************************************************************/

impl Value {
    fn constant_sig_token_to_layout(constant_signature: &SignatureToken) -> Option<MoveTypeLayout> {
        use MoveTypeLayout as L;
        use SignatureToken as S;

        Some(match constant_signature {
            S::Bool => L::Bool,
            S::U8 => L::U8,
            S::U16 => L::U16,
            S::U32 => L::U32,
            S::U64 => L::U64,
            S::U128 => L::U128,
            S::U256 => L::U256,
            S::I8 => L::I8,
            S::I16 => L::I16,
            S::I32 => L::I32,
            S::I64 => L::I64,
            S::I128 => L::I128,
            S::I256 => L::I256,
            S::Address => L::Address,
            S::Signer => return None,
            S::Vector(inner) => L::Vector(Box::new(Self::constant_sig_token_to_layout(inner)?)),
            // Not yet supported
            S::Struct(_) | S::StructInstantiation(_, _) | S::Function(..) => return None,
            // Not allowed/Not meaningful
            S::TypeParameter(_) | S::Reference(_) | S::MutableReference(_) => return None,
        })
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn deserialize_constant(constant: &Constant) -> Option<Value> {
        let layout = Self::constant_sig_token_to_layout(&constant.type_)?;
        // INVARIANT:
        //   For constants, layout depth is bounded and cannot contain function values. Hence,
        //   serialization depth is bounded. We still enable depth checks as a precaution.
        ValueSerDeContext::new(Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH))
            .deserialize(&constant.data, &layout)
    }
}

/***************************************************************************************
*
* Destructors
*
**************************************************************************************/
// Locals may contain reference values that points to the same cotnainer through Rc, hencing forming
// a cycle. Therefore values need to be manually taken out of the Locals in order to not leak memory.
impl Drop for Locals {
    #[cfg_attr(feature = "inline-locals", inline(always))]
    fn drop(&mut self) {
        let mut locals = self.0.borrow_mut();
        for local in locals.iter_mut() {
            match &local {
                Value::Invalid => (),
                _ => {
                    *local = Value::Invalid;
                },
            }
        }
    }
}

/***************************************************************************************
*
* Views
*
**************************************************************************************/
impl Container {
    fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: u64) -> PartialVMResult<()> {
        use Container::*;

        match self {
            Locals(_) => unreachable!("Should not ba able to visit a Locals container directly"),
            Vec(r) => {
                let r = r.borrow();
                if visitor.visit_vec(depth, r.len())? {
                    for val in r.iter() {
                        val.visit_impl(visitor, depth + 1)?;
                    }
                }
                Ok(())
            },
            Struct(r) => {
                let r = r.borrow();
                if visitor.visit_struct(depth, r.len())? {
                    for val in r.iter() {
                        val.visit_impl(visitor, depth + 1)?;
                    }
                }
                Ok(())
            },
            VecU8(r) => visitor.visit_vec_u8(depth, &r.borrow()),
            VecU16(r) => visitor.visit_vec_u16(depth, &r.borrow()),
            VecU32(r) => visitor.visit_vec_u32(depth, &r.borrow()),
            VecU64(r) => visitor.visit_vec_u64(depth, &r.borrow()),
            VecU128(r) => visitor.visit_vec_u128(depth, &r.borrow()),
            VecU256(r) => visitor.visit_vec_u256(depth, &r.borrow()),
            VecI8(r) => visitor.visit_vec_i8(depth, &r.borrow()),
            VecI16(r) => visitor.visit_vec_i16(depth, &r.borrow()),
            VecI32(r) => visitor.visit_vec_i32(depth, &r.borrow()),
            VecI64(r) => visitor.visit_vec_i64(depth, &r.borrow()),
            VecI128(r) => visitor.visit_vec_i128(depth, &r.borrow()),
            VecI256(r) => visitor.visit_vec_i256(depth, &r.borrow()),
            VecBool(r) => visitor.visit_vec_bool(depth, &r.borrow()),
            VecAddress(r) => visitor.visit_vec_address(depth, &r.borrow()),
        }
    }

    fn visit_indexed(
        &self,
        visitor: &mut impl ValueVisitor,
        depth: u64,
        idx: usize,
    ) -> PartialVMResult<()> {
        use Container::*;

        match self {
            Locals(r) | Vec(r) | Struct(r) => r.borrow()[idx].visit_impl(visitor, depth + 1),
            VecU8(vals) => visitor.visit_u8(depth + 1, vals.borrow()[idx]),
            VecU16(vals) => visitor.visit_u16(depth + 1, vals.borrow()[idx]),
            VecU32(vals) => visitor.visit_u32(depth + 1, vals.borrow()[idx]),
            VecU64(vals) => visitor.visit_u64(depth + 1, vals.borrow()[idx]),
            VecU128(vals) => visitor.visit_u128(depth + 1, vals.borrow()[idx]),
            VecU256(vals) => visitor.visit_u256(depth + 1, vals.borrow()[idx]),
            VecI8(vals) => visitor.visit_i8(depth + 1, vals.borrow()[idx]),
            VecI16(vals) => visitor.visit_i16(depth + 1, vals.borrow()[idx]),
            VecI32(vals) => visitor.visit_i32(depth + 1, vals.borrow()[idx]),
            VecI64(vals) => visitor.visit_i64(depth + 1, vals.borrow()[idx]),
            VecI128(vals) => visitor.visit_i128(depth + 1, vals.borrow()[idx]),
            VecI256(vals) => visitor.visit_i256(depth + 1, vals.borrow()[idx]),
            VecBool(vals) => visitor.visit_bool(depth + 1, vals.borrow()[idx]),
            VecAddress(vals) => visitor.visit_address(depth + 1, vals.borrow()[idx]),
        }
    }
}

impl Closure {
    fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: u64) -> PartialVMResult<()> {
        let Self(_, captured) = self;
        if visitor.visit_closure(depth, captured.len())? {
            for val in captured {
                val.visit_impl(visitor, depth + 1)?;
            }
        }
        Ok(())
    }
}

impl ContainerRef {
    fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: u64) -> PartialVMResult<()> {
        use ContainerRef::*;

        let (container, is_global) = match self {
            Local(container) => (container, false),
            Global { container, .. } => (container, false),
        };

        if visitor.visit_ref(depth, is_global)? {
            container.visit_impl(visitor, depth + 1)?;
        }
        Ok(())
    }
}

impl IndexedRef {
    fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: u64) -> PartialVMResult<()> {
        use ContainerRef::*;

        let (container, is_global) = match &self.container_ref {
            Local(container) => (container, false),
            Global { container, .. } => (container, false),
        };

        if visitor.visit_ref(depth, is_global)? {
            container.visit_indexed(visitor, depth, self.idx)?;
        }
        Ok(())
    }
}

impl Value {
    fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: u64) -> PartialVMResult<()> {
        use Value::*;

        match self {
            Invalid => unreachable!("Should not be able to visit an invalid value"),
            U8(val) => visitor.visit_u8(depth, *val),
            U16(val) => visitor.visit_u16(depth, *val),
            U32(val) => visitor.visit_u32(depth, *val),
            U64(val) => visitor.visit_u64(depth, *val),
            U128(val) => visitor.visit_u128(depth, *val),
            U256(val) => visitor.visit_u256(depth, *val),
            I8(val) => visitor.visit_i8(depth, *val),
            I16(val) => visitor.visit_i16(depth, *val),
            I32(val) => visitor.visit_i32(depth, *val),
            I64(val) => visitor.visit_i64(depth, *val),
            I128(val) => visitor.visit_i128(depth, *val),
            I256(val) => visitor.visit_i256(depth, *val),
            Bool(val) => visitor.visit_bool(depth, *val),
            Address(val) => visitor.visit_address(depth, *val),
            Container(c) => c.visit_impl(visitor, depth),
            ContainerRef(r) => r.visit_impl(visitor, depth),
            IndexedRef(r) => r.visit_impl(visitor, depth),
            ClosureValue(c) => c.visit_impl(visitor, depth),
            DelayedFieldID { id } => visitor.visit_delayed(depth, *id),
        }
    }
}

impl ValueView for Value {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        self.visit_impl(visitor, 0)
    }
}

impl ValueView for Struct {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        if visitor.visit_struct(0, self.fields.len())? {
            for val in self.fields.iter() {
                val.visit_impl(visitor, 1)?;
            }
        }
        Ok(())
    }
}

impl ValueView for Vector {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        self.0.visit_impl(visitor, 0)
    }
}

impl ValueView for Reference {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        use ReferenceImpl::*;

        match &self.0 {
            ContainerRef(r) => r.visit_impl(visitor, 0),
            IndexedRef(r) => r.visit_impl(visitor, 0),
        }
    }
}

impl ValueView for VectorRef {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        self.0.visit_impl(visitor, 0)
    }
}

impl ValueView for StructRef {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        self.0.visit_impl(visitor, 0)
    }
}

// Note: We may want to add more helpers to retrieve value views behind references here.

impl Struct {
    pub fn field_views(&self) -> impl ExactSizeIterator<Item = impl ValueView + '_> + Clone {
        self.fields.iter()
    }
}

impl Vector {
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn elem_views(&self) -> impl ExactSizeIterator<Item = impl ValueView + '_> + Clone {
        struct ElemView<'b> {
            container: &'b Container,
            idx: usize,
        }

        impl ValueView for ElemView<'_> {
            fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
                self.container.visit_indexed(visitor, 0, self.idx)
            }
        }

        let len = self.0.len();

        (0..len).map(|idx| ElemView {
            container: &self.0,
            idx,
        })
    }
}

impl Reference {
    pub fn value_view(&self) -> impl ValueView + '_ {
        struct ValueBehindRef<'b>(&'b ReferenceImpl);

        impl ValueView for ValueBehindRef<'_> {
            fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
                use ReferenceImpl::*;

                match self.0 {
                    ContainerRef(r) => r.container().visit_impl(visitor, 0),
                    IndexedRef(r) => r.container_ref.container().visit_indexed(visitor, 0, r.idx),
                }
            }
        }

        ValueBehindRef(&self.0)
    }
}

impl GlobalValue {
    pub fn view(&self) -> Option<impl ValueView + '_> {
        use GlobalValueImpl as G;

        struct Wrapper<'b>(&'b Rc<RefCell<Vec<Value>>>);

        impl ValueView for Wrapper<'_> {
            fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
                let r = self.0.borrow();
                if visitor.visit_struct(0, r.len())? {
                    for val in r.iter() {
                        val.visit_impl(visitor, 1)?;
                    }
                }
                Ok(())
            }
        }

        match &self.0 {
            G::None | G::Deleted => None,
            G::Cached { value, .. } | G::Fresh { value } => {
                Some(Wrapper(GlobalValueImpl::expect_struct_fields(value)))
            },
        }
    }
}

/***************************************************************************************
 *
 * Prop Testing
 *
 *   Random generation of values that fit into a given layout.
 *
 **************************************************************************************/
#[cfg(feature = "fuzzing")]
pub mod prop {
    use super::*;
    use crate::values::function_values_impl::mock;
    #[allow(unused_imports)]
    use move_core_types::{
        ability::AbilitySet,
        function::ClosureMask,
        language_storage::{FunctionParamOrReturnTag, FunctionTag, TypeTag},
        value::{MoveStruct, MoveValue},
    };
    use proptest::{collection::vec, prelude::*};

    fn type_tag_strategy() -> impl Strategy<Value = TypeTag> {
        use move_core_types::language_storage::{FunctionTag, StructTag};
        use proptest::prelude::any;

        let leaf = prop_oneof![
            1 => Just(TypeTag::Bool),
            1 => Just(TypeTag::U8),
            1 => Just(TypeTag::U16),
            1 => Just(TypeTag::U32),
            1 => Just(TypeTag::U64),
            1 => Just(TypeTag::U128),
            1 => Just(TypeTag::U256),
            1 => Just(TypeTag::I8),
            1 => Just(TypeTag::I16),
            1 => Just(TypeTag::I32),
            1 => Just(TypeTag::I64),
            1 => Just(TypeTag::I128),
            1 => Just(TypeTag::I256),
            1 => Just(TypeTag::Address),
            1 => Just(TypeTag::Signer),
        ];

        prop_oneof![
            3 => leaf.clone(), // Direct leaf types at top level
            2 => leaf.clone().prop_recursive(4, 16, 2, |inner| {
                prop_oneof![
                    1 => inner.clone().prop_map(|ty| TypeTag::Vector(Box::new(ty))),
                    1 => any::<StructTag>().prop_map(|struct_tag| {
                         TypeTag::Struct(Box::new(struct_tag))
                     }),
                ]
            }),
            1 => (vec(leaf.clone(), 0..=2), vec(leaf, 0..=2), any::<AbilitySet>()).prop_map(|(args, results, abilities)| {
                TypeTag::Function(Box::new(FunctionTag {
                    args: args.into_iter().map(FunctionParamOrReturnTag::Value).collect(),
                    results: results.into_iter().map(FunctionParamOrReturnTag::Value).collect(),
                    abilities,
                }))
            }),
        ]
    }

    pub fn value_strategy_with_layout(
        layout: &MoveTypeLayout,
    ) -> impl Strategy<Value = Value> + use<> {
        use MoveTypeLayout as L;

        match layout {
            L::U8 => any::<u8>().prop_map(Value::u8).boxed(),
            L::U16 => any::<u16>().prop_map(Value::u16).boxed(),
            L::U32 => any::<u32>().prop_map(Value::u32).boxed(),
            L::U64 => any::<u64>().prop_map(Value::u64).boxed(),
            L::U128 => any::<u128>().prop_map(Value::u128).boxed(),
            L::U256 => any::<int256::U256>().prop_map(Value::u256).boxed(),
            L::I8 => any::<i8>().prop_map(Value::i8).boxed(),
            L::I16 => any::<i16>().prop_map(Value::i16).boxed(),
            L::I32 => any::<i32>().prop_map(Value::i32).boxed(),
            L::I64 => any::<i64>().prop_map(Value::i64).boxed(),
            L::I128 => any::<i128>().prop_map(Value::i128).boxed(),
            L::I256 => any::<int256::I256>().prop_map(Value::i256).boxed(),
            L::Bool => any::<bool>().prop_map(Value::bool).boxed(),
            L::Address => any::<AccountAddress>().prop_map(Value::address).boxed(),
            L::Signer => any::<AccountAddress>()
                .prop_map(Value::master_signer)
                .boxed(),

            L::Vector(layout) => match &**layout {
                L::U8 => vec(any::<u8>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU8(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::U16 => vec(any::<u16>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU16(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::U32 => vec(any::<u32>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU32(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::U64 => vec(any::<u64>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU64(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::U128 => vec(any::<u128>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU128(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::U256 => vec(any::<int256::U256>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecU256(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I8 => vec(any::<i8>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI8(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I16 => vec(any::<i16>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI16(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I32 => vec(any::<i32>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI32(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I64 => vec(any::<i64>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI64(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I128 => vec(any::<i128>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI128(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::I256 => vec(any::<int256::I256>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecI256(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::Bool => vec(any::<bool>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecBool(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                L::Address => vec(any::<AccountAddress>(), 0..10)
                    .prop_map(|vals| {
                        Value::Container(Container::VecAddress(Rc::new(RefCell::new(vals))))
                    })
                    .boxed(),
                layout => vec(value_strategy_with_layout(layout), 0..10)
                    .prop_map(|vals| Value::Container(Container::Vec(Rc::new(RefCell::new(vals)))))
                    .boxed(),
            },
            L::Struct(_struct_layout @ MoveStructLayout::RuntimeVariants(variants)) => {
                // Randomly choose a variant index
                let variant_count = variants.len();
                let variants = variants.clone();
                (0..variant_count as u16)
                    .prop_flat_map(move |variant_tag| {
                        let variant_layouts = variants[variant_tag as usize].clone();
                        variant_layouts
                            .iter()
                            .map(value_strategy_with_layout)
                            .collect::<Vec<_>>()
                            .prop_map(move |vals| {
                                Value::struct_(Struct::pack_variant(variant_tag, vals))
                            })
                    })
                    .boxed()
            },

            L::Struct(struct_layout) => struct_layout
                .fields(None)
                .iter()
                .map(value_strategy_with_layout)
                .collect::<Vec<_>>()
                .prop_map(move |vals| Value::struct_(Struct::pack(vals)))
                .boxed(),

            L::Function => {
                (
                    "[a-z][a-z0-9_]{0,8}",
                    any::<u8>().prop_map(|bits| ClosureMask::new((bits % 16) as u64)),
                )
                    .prop_flat_map(|(name, mask)| {
                        let num_captured = mask.captured_count() as usize;

                        // Generate random type arguments (0-3 type args)
                        let ty_args_strategy = vec(type_tag_strategy(), 0..=3);

                        // Generate random layouts for each captured value
                        let captured_layouts_strategy = vec(layout_strategy(), num_captured);

                        (ty_args_strategy, captured_layouts_strategy).prop_flat_map(
                            move |(ty_args, captured_layouts)| {
                                // Then recursively generate values matching those layouts
                                let name = name.clone();
                                let captured_strategies = captured_layouts
                                    .iter()
                                    .map(value_strategy_with_layout)
                                    .collect::<Vec<_>>();

                                captured_strategies.prop_map(move |captured_values| {
                                    let fun = mock::MockAbstractFunction::new(
                                        &name,
                                        ty_args.clone(),
                                        mask,
                                        captured_layouts.clone(),
                                    );
                                    Value::closure(Box::new(fun), captured_values)
                                })
                            },
                        )
                    })
                    .boxed()
            },

            // TODO[agg_v2](cleanup): double check what we should do here (i.e. if we should
            //  even skip these kinds of layouts, or if need to construct a delayed value)?
            L::Native(_, layout) => value_strategy_with_layout(layout.as_ref()),
        }
    }

    pub fn layout_strategy() -> impl Strategy<Value = MoveTypeLayout> {
        use MoveTypeLayout as L;

        // Non-recursive leafs
        let leaf = prop_oneof![
            1 => Just(L::U8),
            1 => Just(L::U16),
            1 => Just(L::U32),
            1 => Just(L::U64),
            1 => Just(L::U128),
            1 => Just(L::U256),
            1 => Just(L::I8),
            1 => Just(L::I16),
            1 => Just(L::I32),
            1 => Just(L::I64),
            1 => Just(L::I128),
            1 => Just(L::I256),
            1 => Just(L::Bool),
            1 => Just(L::Address),
        ];

        // Return a random layout strategy
        prop_oneof![
            1 => leaf.clone(),
            // Recursive leafs are 4x more likely than non-recursive leafs
            4 => leaf.prop_recursive(8, 32, 2, |inner| {
                prop_oneof![
                    1 => inner.clone().prop_map(|layout| L::Vector(Box::new(layout))),
                    1 => vec(inner.clone(), 0..=5).prop_map(|f_layouts| {
                            L::Struct(MoveStructLayout::new(f_layouts))}),
                    1 => vec(vec(inner, 0..=3), 1..=4).prop_map(|variant_layouts| {
                            L::Struct(MoveStructLayout::new_variants(variant_layouts))}),
                ]
            }),
            2 => Just(L::Function),
        ]
    }

    pub fn layout_and_value_strategy() -> impl Strategy<Value = (MoveTypeLayout, Value)> {
        layout_strategy().no_shrink().prop_flat_map(|layout| {
            let value_strategy = value_strategy_with_layout(&layout);
            (Just(layout), value_strategy)
        })
    }
}

#[cfg(any(test, feature = "fuzzing", feature = "testing"))]
impl Value {
    // TODO: Consider removing this API, or at least it should return a Result!
    pub fn as_move_value(&self, layout: &MoveTypeLayout) -> MoveValue {
        use crate::values::function_values_impl::mock::MockAbstractFunction;
        use MoveTypeLayout as L;

        if let L::Native(kind, layout) = layout {
            panic!(
                "impossible to get native layout ({:?}) with {}",
                kind, layout
            )
        }

        match (layout, &self) {
            (L::U8, Value::U8(x)) => MoveValue::U8(*x),
            (L::U16, Value::U16(x)) => MoveValue::U16(*x),
            (L::U32, Value::U32(x)) => MoveValue::U32(*x),
            (L::U64, Value::U64(x)) => MoveValue::U64(*x),
            (L::U128, Value::U128(x)) => MoveValue::U128(*x),
            (L::U256, Value::U256(x)) => MoveValue::U256(*x),
            (L::I8, Value::I8(x)) => MoveValue::I8(*x),
            (L::I16, Value::I16(x)) => MoveValue::I16(*x),
            (L::I32, Value::I32(x)) => MoveValue::I32(*x),
            (L::I64, Value::I64(x)) => MoveValue::I64(*x),
            (L::I128, Value::I128(x)) => MoveValue::I128(*x),
            (L::I256, Value::I256(x)) => MoveValue::I256(*x),
            (L::Bool, Value::Bool(x)) => MoveValue::Bool(*x),
            (L::Address, Value::Address(x)) => MoveValue::Address(*x),

            (L::Struct(struct_layout), Value::Container(Container::Struct(r))) => {
                let values_ref = r.borrow();
                let values = values_ref.as_slice();
                if let Some((tag, variant_layouts)) =
                    try_get_variant_field_layouts(struct_layout, values)
                {
                    MoveValue::Struct(MoveStruct::new_variant(
                        tag,
                        values
                            .iter()
                            // Skip the tag value
                            .skip(1)
                            .zip(variant_layouts.iter())
                            .map(|(v, field_layout)| v.as_move_value(field_layout))
                            .collect(),
                    ))
                } else {
                    MoveValue::Struct(MoveStruct::new(
                        values
                            .iter()
                            .zip(struct_layout.fields(None))
                            .map(|(v, field_layout)| v.as_move_value(field_layout))
                            .collect(),
                    ))
                }
            },

            (L::Vector(inner_layout), Value::Container(c)) => MoveValue::Vector(match c {
                Container::VecU8(r) => r.borrow().iter().map(|u| MoveValue::U8(*u)).collect(),
                Container::VecU16(r) => r.borrow().iter().map(|u| MoveValue::U16(*u)).collect(),
                Container::VecU32(r) => r.borrow().iter().map(|u| MoveValue::U32(*u)).collect(),
                Container::VecU64(r) => r.borrow().iter().map(|u| MoveValue::U64(*u)).collect(),
                Container::VecU128(r) => r.borrow().iter().map(|u| MoveValue::U128(*u)).collect(),
                Container::VecU256(r) => r.borrow().iter().map(|u| MoveValue::U256(*u)).collect(),
                Container::VecI8(r) => r.borrow().iter().map(|u| MoveValue::I8(*u)).collect(),
                Container::VecI16(r) => r.borrow().iter().map(|u| MoveValue::I16(*u)).collect(),
                Container::VecI32(r) => r.borrow().iter().map(|u| MoveValue::I32(*u)).collect(),
                Container::VecI64(r) => r.borrow().iter().map(|u| MoveValue::I64(*u)).collect(),
                Container::VecI128(r) => r.borrow().iter().map(|u| MoveValue::I128(*u)).collect(),
                Container::VecI256(r) => r.borrow().iter().map(|u| MoveValue::I256(*u)).collect(),
                Container::VecBool(r) => r.borrow().iter().map(|u| MoveValue::Bool(*u)).collect(),
                Container::VecAddress(r) => {
                    r.borrow().iter().map(|u| MoveValue::Address(*u)).collect()
                },
                Container::Vec(r) => r
                    .borrow()
                    .iter()
                    .map(|v| v.as_move_value(inner_layout.as_ref()))
                    .collect(),
                Container::Struct(_) => {
                    panic!("got struct container when converting vec")
                },
                Container::Locals(_) => panic!("got locals container when converting vec"),
            }),

            (L::Signer, Value::Container(Container::Struct(r))) => {
                let v = r.borrow();
                match &v[MASTER_ADDRESS_FIELD_OFFSET] {
                    Value::Address(a) => MoveValue::Signer(*a),
                    v => panic!("Unexpected non-address while converting signer: {:?}", v),
                }
            },

            (L::Function, Value::ClosureValue(closure)) => {
                use better_any::TidExt;
                use move_core_types::function::MoveClosure;

                // Downcast to MockAbstractFunction to access data directly
                if let Some(mock_fun) = closure.0.downcast_ref::<MockAbstractFunction>() {
                    let move_closure = MoveClosure {
                        module_id: mock_fun.data.module_id.clone(),
                        fun_id: mock_fun.data.fun_id.clone(),
                        ty_args: mock_fun.data.ty_args.clone(),
                        mask: mock_fun.data.mask,
                        captured: closure
                            .1
                            .iter()
                            .zip(mock_fun.data.captured_layouts.iter())
                            .map(|(captured_val, layout)| {
                                (layout.clone(), captured_val.as_move_value(layout))
                            })
                            .collect(),
                    };
                    MoveValue::closure(move_closure)
                } else {
                    // Fallback for unknown function types
                    panic!("Cannot convert unknown function type to MoveValue")
                }
            },

            (layout, val) => panic!("Cannot convert value {:?} as {:?}", val, layout),
        }
    }
}

fn try_get_variant_field_layouts<'a>(
    layout: &'a MoveStructLayout,
    values: &[Value],
) -> Option<(u16, &'a [MoveTypeLayout])> {
    if matches!(layout, MoveStructLayout::RuntimeVariants(..)) {
        if let Some(Value::U16(tag)) = values.first() {
            return Some((*tag, layout.fields(Some(*tag as usize))));
        }
    }
    None
}

#[cfg_attr(feature = "force-inline", inline(always))]
fn check_depth(depth: u64, max_depth: Option<u64>) -> PartialVMResult<()> {
    if max_depth.is_some_and(|max_depth| depth > max_depth) {
        return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
    }
    Ok(())
}
