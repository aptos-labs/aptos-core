// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A crate which extends Move with a RistrettoPoint struct that points to a Rust-native
//! curve25519_dalek::ristretto::RistrettoPoint.

use crate::natives::cryptography::{
    helpers::log2_floor,
    ristretto255::{
        pop_64_byte_slice, pop_scalar_from_bytes, scalar_from_struct, COMPRESSED_POINT_NUM_BYTES,
    },
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, safely_pop_type_arg, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use better_any::{Tid, TidAble};
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_TABLE,
    ristretto::{CompressedRistretto, RistrettoPoint},
    traits::{Identity, VartimeMultiscalarMul},
};
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, StructRef, Value, VectorRef},
};
use sha2::Sha512;
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::VecDeque,
    convert::TryFrom,
    fmt::Display,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

//
// Public Data Structures and Constants
//

/// The representation of a RistrettoPoint handle.
/// The handle is just an incrementing counter whenever a new point is added to the PointStore.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct RistrettoPointHandle(pub u64);

impl Display for RistrettoPointHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RistrettoPoint-{:X}", self.0)
    }
}

/// The native RistrettoPoint context extension. This needs to be attached to the NativeContextExtensions
/// value which is passed into session functions, so its accessible from natives of this extension.
#[derive(Default, Tid)]
pub struct NativeRistrettoPointContext {
    pub point_data: RefCell<PointStore>,
}

//
// Private Data Structures and Constants
//

/// This limit ensures that no more than 1.6MB will be allocated for Ristretto points (160 bytes for each) per VM session.
const NUM_POINTS_LIMIT: usize = 10000;

/// Equivalent to `std::error::resource_exhausted(4)` in Move.
const E_TOO_MANY_POINTS_CREATED: u64 = 0x09_0004;

/// A structure representing mutable data of the NativeRistrettoPointContext. This is in a RefCell
/// of the overall context so we can mutate while still accessing the overall context.
#[derive(Default)]
pub struct PointStore {
    points: Vec<RistrettoPoint>,
}

/// The field index of the `handle` field in the `RistrettoPoint` Move struct.
const HANDLE_FIELD_INDEX: usize = 0;

//
// Implementation of Native RistrettoPoint Context
//

impl NativeRistrettoPointContext {
    /// Create a new instance of a native RistrettoPoint context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new() -> Self {
        Self {
            point_data: Default::default(),
        }
    }
}

impl PointStore {
    /// Re-sets a RistrettoPoint that was previously allocated.
    pub fn set_point(&mut self, handle: &RistrettoPointHandle, point: RistrettoPoint) {
        self.points[handle.0 as usize] = point
    }

    /// Gets a RistrettoPoint that was previously allocated.
    pub fn get_point(&self, handle: &RistrettoPointHandle) -> &RistrettoPoint {
        //&self.points[handle.0 as usize]
        self.points.get(handle.0 as usize).unwrap()
    }

    /// Gets a RistrettoPoint that was previously allocated.
    pub fn get_point_mut(&mut self, handle: &RistrettoPointHandle) -> &mut RistrettoPoint {
        //&mut self.points[handle.0 as usize]
        self.points.get_mut(handle.0 as usize).unwrap()
    }

    /// Returns mutable references to two different Ristretto points in the vector using split_at_mut.
    /// Note that Rust's linear types prevent us from simply returning `(&mut points[i], &mut points[j])`.
    pub fn get_two_muts(
        &mut self,
        a: &RistrettoPointHandle,
        b: &RistrettoPointHandle,
    ) -> (&mut RistrettoPoint, &mut RistrettoPoint) {
        use std::cmp::Ordering;

        let (sw, a, b) = match Ord::cmp(&a, &b) {
            Ordering::Less => (false, a.0 as usize, b.0 as usize),
            Ordering::Greater => (true, b.0 as usize, a.0 as usize),
            Ordering::Equal => panic!("attempted to exclusive-borrow one element twice"),
        };

        let (left, right) = self.points.split_at_mut(a + 1);
        let (a_ref, b_ref) = (&mut left[a], &mut right[b - (a + 1)]);

        if sw {
            (b_ref, a_ref)
        } else {
            (a_ref, b_ref)
        }
    }

    /// Adds the point to the store and returns its RistrettoPointHandle ID.
    /// Aborts if the number of points has exceeded a limit.
    fn safe_add_point(&mut self, point: RistrettoPoint) -> SafeNativeResult<u64> {
        let id = self.points.len();
        if id >= NUM_POINTS_LIMIT {
            Err(SafeNativeError::Abort {
                abort_code: E_TOO_MANY_POINTS_CREATED,
            })
        } else {
            self.points.push(point);
            Ok(id as u64)
        }
    }
}

//
// Partial implementation of GasParameters for point operations
//

/// If 'bytes' canonically-encode a valid RistrettoPoint, returns the point.  Otherwise, returns None.
fn decompress_maybe_non_canonical_point_bytes(
    context: &mut SafeNativeContext,
    bytes: Vec<u8>,
) -> SafeNativeResult<Option<RistrettoPoint>> {
    context.charge(RISTRETTO255_POINT_DECOMPRESS * NumArgs::one())?;

    let compressed = match compressed_point_from_bytes(bytes) {
        Some(point) => point,
        None => return Ok(None),
    };

    Ok(compressed.decompress())
}

//
// Native function implementations for point operations
//

pub(crate) fn native_point_identity(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 0);

    context.charge(RISTRETTO255_POINT_IDENTITY * NumArgs::one())?;
    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();
    let point = RistrettoPoint::identity();
    let result_handle = point_data.safe_add_point(point)?;

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_point_is_canonical(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let opt_point = decompress_maybe_non_canonical_point_bytes(context, bytes)?;

    Ok(smallvec![Value::bool(opt_point.is_some())])
}

pub(crate) fn native_point_decompress(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    let point = match decompress_maybe_non_canonical_point_bytes(context, bytes)? {
        Some(point) => point,
        None => {
            // NOTE: We return (u64::MAX, false) in this case.
            return Ok(smallvec![Value::u64(u64::MAX), Value::bool(false)]);
        },
    };

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    // Take the # of points produced so far, which creates a unique and deterministic global ID
    // within the temporary scope of this current transaction. Then, store the RistrettoPoint in
    // a vector using this global ID as an index.
    let id = point_data.safe_add_point(point)?;

    Ok(smallvec![Value::u64(id), Value::bool(true)])
}

pub(crate) fn native_point_clone(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    context.charge(RISTRETTO255_POINT_CLONE * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();
    let handle = pop_as_ristretto_handle(&mut args)?;
    let point = point_data.get_point(&handle);
    let clone = *point;
    let result_handle = point_data.safe_add_point(clone)?;

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_point_compress(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    context.charge(RISTRETTO255_POINT_COMPRESS * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let point_data = point_context.point_data.borrow();
    let handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    let point = point_data.get_point(&handle);

    Ok(smallvec![Value::vector_u8(point.compress().to_bytes())])
}

pub(crate) fn native_point_mul(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    context.charge(RISTRETTO255_POINT_MUL * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = safely_pop_arg!(args, bool);
    let scalar = pop_scalar_from_bytes(&mut args)?;
    let point_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    // Compute result = a * point (or a = a * point) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let point = point_data.get_point(&point_handle).mul(scalar);
            point_data.safe_add_point(point)?
        },
        true => {
            point_data.get_point_mut(&point_handle).mul_assign(scalar);
            point_handle.0
        },
    };

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_point_equals(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 2);

    context.charge(RISTRETTO255_POINT_EQUALS * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let point_data = point_context.point_data.borrow_mut();

    let b_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    let a = point_data.get_point(&a_handle);
    let b = point_data.get_point(&b_handle);

    // Checks if a == b
    Ok(smallvec![Value::bool(a.eq(b))])
}

pub(crate) fn native_point_neg(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 2);

    context.charge(RISTRETTO255_POINT_NEG * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = safely_pop_arg!(args, bool);
    let point_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    // Compute result = - point (or point = -point) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let point = point_data.get_point(&point_handle).neg();
            point_data.safe_add_point(point)?
        },
        true => {
            let neg = point_data.get_point_mut(&point_handle).neg();
            point_data.set_point(&point_handle, neg);
            point_handle.0
        },
    };

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_point_add(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    context.charge(RISTRETTO255_POINT_ADD * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = safely_pop_arg!(args, bool);
    let b_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    // Compute result = a + b (or a = a + b) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let a = point_data.get_point(&a_handle);
            let b = point_data.get_point(&b_handle);

            let point = a.add(b);
            point_data.safe_add_point(point)?
        },
        true => {
            // NOTE: When calling Move's add_assign, Move's linear types ensure that we will never
            // get references `&mut a` and `&a = &b`, while our own invariants ensure
            // we never have two different Move `RistrettoPoint` structs constructed with the same
            // handles.
            debug_assert!(a_handle != b_handle);
            let (a, b) = point_data.get_two_muts(&a_handle, &b_handle);

            a.add_assign(&*b);
            a_handle.0
        },
    };

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_point_sub(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    context.charge(RISTRETTO255_POINT_SUB * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = safely_pop_arg!(args, bool);
    let b_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;

    // Compute result = a - b (or a = a - b) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let a = point_data.get_point(&a_handle);
            let b = point_data.get_point(&b_handle);

            let point = a.sub(b);
            point_data.safe_add_point(point)?
        },
        true => {
            // NOTE: When calling Move's sub_assign, Move's linear types ensure that we will never
            // get references to the same a and b RistrettoPoint, while our own invariants ensure
            // we never have two different Move RistrettoPoint constructed with the same handles.
            debug_assert!(a_handle != b_handle);
            let (a, b) = point_data.get_two_muts(&a_handle, &b_handle);

            a.sub_assign(&*b);
            a_handle.0
        },
    };

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_basepoint_mul(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    context.charge(RISTRETTO255_BASEPOINT_MUL * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let a = pop_scalar_from_bytes(&mut args)?;

    let basepoint = RISTRETTO_BASEPOINT_TABLE;
    let result = basepoint.mul(&a);
    let result_handle = point_data.safe_add_point(result)?;

    Ok(smallvec![Value::u64(result_handle)])
}

#[allow(non_snake_case)]
pub(crate) fn native_basepoint_double_mul(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    context.charge(RISTRETTO255_BASEPOINT_DOUBLE_MUL * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let b = pop_scalar_from_bytes(&mut args)?;
    let A_handle = pop_ristretto_handle(&mut args)?;
    let a = pop_scalar_from_bytes(&mut args)?;

    // Compute result = a * A + b * BASEPOINT and return a RistrettoPointHandle
    let A_ref = point_data.get_point(&A_handle);
    let result = RistrettoPoint::vartime_double_scalar_mul_basepoint(&a, A_ref, &b);
    let result_handle = point_data.safe_add_point(result)?;

    Ok(smallvec![Value::u64(result_handle)])
}

// NOTE: This was supposed to be more clearly named with *_sha2_512_*
pub(crate) fn native_new_point_from_sha512(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let bytes = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RISTRETTO255_POINT_FROM_64_UNIFORM_BYTES * NumArgs::one()
            + RISTRETTO255_SHA512_PER_HASH * NumArgs::one()
            + RISTRETTO255_SHA512_PER_BYTE * NumBytes::new(bytes.len() as u64),
    )?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();
    let point = RistrettoPoint::hash_from_bytes::<Sha512>(&bytes);
    let result_handle = point_data.safe_add_point(point)?;

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_new_point_from_64_uniform_bytes(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    context.charge(RISTRETTO255_POINT_FROM_64_UNIFORM_BYTES * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let slice = pop_64_byte_slice(&mut args)?;

    let point = RistrettoPoint::from_uniform_bytes(&slice);
    let result_handle = point_data.safe_add_point(point)?;

    Ok(smallvec![Value::u64(result_handle)])
}

pub(crate) fn native_double_scalar_mul(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(args.len(), 4);

    context.charge(RISTRETTO255_POINT_DOUBLE_MUL * NumArgs::one())?;

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let scalar2 = pop_scalar_from_bytes(&mut args)?;
    let scalar1 = pop_scalar_from_bytes(&mut args)?;
    let handle2 = pop_as_ristretto_handle(&mut args)?;
    let handle1 = pop_as_ristretto_handle(&mut args)?;

    let points = vec![
        point_data.get_point(&handle1),
        point_data.get_point(&handle2),
    ];

    let scalars = [scalar1, scalar2];

    let result = RistrettoPoint::vartime_multiscalar_mul(scalars.iter(), points);

    let result_handle = point_data.safe_add_point(result)?;

    Ok(smallvec![Value::u64(result_handle)])
}

/// This upgrades 'native_multi_scalar_mul' in two ways:
/// 1. It is a "safe" native that uses `SafeNativeContext::charge` to prevent DoS attacks.
/// 2. It no longer uses floating-point arithmetic to compute the gas costs.
///
/// Pre-conditions: The # of scalars & points are both > 0. This is ensured by the Move calling
/// function.
pub(crate) fn safe_native_multi_scalar_mul_no_floating_point(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 2);
    safely_assert_eq!(args.len(), 2);

    let _scalar_type = safely_pop_type_arg!(ty_args);
    let _point_type = safely_pop_type_arg!(ty_args);

    let scalars_ref = safely_pop_arg!(args, VectorRef);
    let points_ref = safely_pop_arg!(args, VectorRef);

    // Invariant (enforced by caller): num > 0 and # of scalars = # of points
    let num = scalars_ref.len()?.value_as::<u64>()? as usize;

    // Invariant: log2_floor(num + 1) > 0. This is because num >= 1, thanks to the invariant we enforce on
    // the caller of this native. Therefore, num + 1 >= 2, which implies log2_floor(num + 1) >= 1.
    // So we never divide by zero.
    context.charge(
        RISTRETTO255_POINT_PARSE_ARG * NumArgs::new(num as u64)
            + RISTRETTO255_SCALAR_PARSE_ARG * NumArgs::new(num as u64)
            + RISTRETTO255_POINT_MUL * NumArgs::new((num / log2_floor(num + 1).unwrap()) as u64),
    )?;

    // parse scalars
    let mut scalars = Vec::with_capacity(num);
    for i in 0..num {
        let move_scalar = scalars_ref.borrow_elem(i)?;
        let scalar = scalar_from_struct(move_scalar)?;

        scalars.push(scalar);
    }

    let result = {
        let point_data = context
            .extensions()
            .get::<NativeRistrettoPointContext>()
            .point_data
            .borrow();

        // parse points
        let mut points = Vec::with_capacity(num);
        for i in 0..num {
            let move_point = points_ref.borrow_elem(i)?;
            let point_handle = get_point_handle_from_struct(move_point)?;

            points.push(point_data.get_point(&point_handle));
        }

        // NOTE: The variable-time multiscalar multiplication (MSM) algorithm for a size-n MSM employed in curve25519 is:
        //  1. Strauss, when n <= 190, see https://www.jstor.org/stable/2310929
        //  2. Pippinger, when n > 190, which roughly requires O(n / log_2 n) scalar multiplications
        // For simplicity, we estimate the complexity as O(n / log_2 n)
        RistrettoPoint::vartime_multiscalar_mul(scalars.iter(), points)
    };

    let mut point_data_mut = context
        .extensions()
        .get::<NativeRistrettoPointContext>()
        .point_data
        .borrow_mut();

    let result_handle = point_data_mut.safe_add_point(result)?;

    Ok(smallvec![Value::u64(result_handle)])
}

// =========================================================================================
// Helpers

pub fn get_point_handle(move_point: &StructRef) -> SafeNativeResult<RistrettoPointHandle> {
    let field_ref = move_point
        .borrow_field(HANDLE_FIELD_INDEX)?
        .value_as::<Reference>()?;

    let handle = field_ref.read_ref()?.value_as::<u64>()?;

    Ok(RistrettoPointHandle(handle))
}

/// Get a RistrettoPointHandle struct from a Move RistrettoPoint struct.
pub fn get_point_handle_from_struct(move_point: Value) -> SafeNativeResult<RistrettoPointHandle> {
    let move_struct = move_point.value_as::<StructRef>()?;

    get_point_handle(&move_struct)
}

/// Pops a RistrettoPointHandle off the argument stack (when the argument is a &RistrettoPoint struct
/// that wraps the u64 handle)
fn pop_ristretto_handle(args: &mut VecDeque<Value>) -> SafeNativeResult<RistrettoPointHandle> {
    get_point_handle(&safely_pop_arg!(args, StructRef))
}

/// Pops a RistrettoPointHandle off the argument stack (when the argument is the u64 handle itself)
/// TODO: rename this and the above function to be more clear
fn pop_as_ristretto_handle(args: &mut VecDeque<Value>) -> SafeNativeResult<RistrettoPointHandle> {
    let handle = safely_pop_arg!(args, u64);

    Ok(RistrettoPointHandle(handle))
}

/// Checks if `COMPRESSED_POINT_NUM_BYTES` bytes were given as input and, if so, returns Some(CompressedRistretto).
fn compressed_point_from_bytes(bytes: Vec<u8>) -> Option<CompressedRistretto> {
    match <[u8; COMPRESSED_POINT_NUM_BYTES]>::try_from(bytes) {
        Ok(slice) => Some(CompressedRistretto(slice)),
        Err(_) => None,
    }
}
