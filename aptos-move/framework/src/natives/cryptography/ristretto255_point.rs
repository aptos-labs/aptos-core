// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A crate which extends Move with a RistrettoPoint struct that points to a Rust-native
//! curve25519_dalek::ristretto::RistrettoPoint.

use crate::natives::cryptography::ristretto255::{pop_64_byte_slice, GasParameters};
use crate::natives::cryptography::ristretto255::{pop_scalar_from_bytes, scalar_from_struct};
use better_any::{Tid, TidAble};
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};
use move_deps::move_core_types::gas_algebra::{InternalGas, NumArgs, NumBytes};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{Reference, StructRef, Value, VectorRef},
    },
};
use sha2::Sha512;
use smallvec::smallvec;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::{cell::RefCell, collections::VecDeque, convert::TryFrom, fmt::Display};

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
#[derive(Tid)]
pub struct NativeRistrettoPointContext {
    point_data: RefCell<PointStore>,
}

//
// Private Data Structures and Constants
//

/// A structure representing mutable data of the NativeRistrettoPointContext. This is in a RefCell
/// of the overall context so we can mutate while still accessing the overall context.
#[derive(Default)]
struct PointStore {
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
    fn set_point(&mut self, handle: &RistrettoPointHandle, point: RistrettoPoint) {
        self.points[handle.0 as usize] = point
    }

    /// Gets a RistrettoPoint that was previously allocated.
    fn get_point(&self, handle: &RistrettoPointHandle) -> &RistrettoPoint {
        //&self.points[handle.0 as usize]
        self.points.get(handle.0 as usize).unwrap()
    }

    /// Gets a RistrettoPoint that was previously allocated.
    fn get_point_mut(&mut self, handle: &RistrettoPointHandle) -> &mut RistrettoPoint {
        //&mut self.points[handle.0 as usize]
        self.points.get_mut(handle.0 as usize).unwrap()
    }

    /// Returns mutable references to two different Ristretto points in the vector using split_at_mut.
    /// Note that Rust's linear types prevent us from simply returning `(&mut points[i], &mut points[j])`.
    fn get_two_muts(
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

    /// Adds the point to the store and returns its RistrettoPointHandle ID
    pub fn add_point(&mut self, point: RistrettoPoint) -> u64 {
        let id = self.points.len();
        self.points.push(point);

        id as u64
    }
}

//
// Partial implementation of GasParameters for point operations
//

impl GasParameters {
    /// If 'bytes' canonically-encode a valid RistrettoPoint, returns the point.  Otherwise, returns None.
    fn decompress_maybe_non_canonical_point_bytes(
        &self,
        cumulative_cost: &mut InternalGas,
        bytes: Vec<u8>,
    ) -> Option<RistrettoPoint> {
        let compressed = match compressed_point_from_bytes(bytes) {
            Some(point) => point,
            None => return None,
        };

        *cumulative_cost += self.point_decompress * NumArgs::one();
        compressed.decompress()
    }
}

//
// Native function implementations for point operations
//

pub(crate) fn native_point_identity(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 0);

    let cost = gas_params.point_identity * NumArgs::one();
    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();
    let result_handle = point_data.add_point(RistrettoPoint::identity());

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_point_is_canonical(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(_ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    let bytes = pop_arg!(args, Vec<u8>);
    let mut cost = InternalGas::zero();
    let opt_point = gas_params.decompress_maybe_non_canonical_point_bytes(&mut cost, bytes);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(opt_point.is_some())],
    ))
}

pub(crate) fn native_point_decompress(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(_ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();
    let bytes = pop_arg!(args, Vec<u8>);
    let mut cost = InternalGas::zero();

    let point = match gas_params.decompress_maybe_non_canonical_point_bytes(&mut cost, bytes) {
        Some(point) => point,
        None => {
            // NOTE: We return (u64::MAX, false) in this case.
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::u64(u64::MAX), Value::bool(false)],
            ));
        }
    };

    // Take the # of points produced so far, which creates a unique and deterministic global ID
    // within the temporary scope of this current transaction. Then, store the RistrettoPoint in
    // a vector using this global ID as an index.
    let id = point_data.add_point(point);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::u64(id), Value::bool(true)],
    ))
}

pub(crate) fn native_point_compress(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 1);
    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let point_data = point_context.point_data.borrow();
    let handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_compress * NumArgs::one();
    let point = point_data.get_point(&handle);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(point.compress().to_bytes())],
    ))
}

pub(crate) fn native_point_mul(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 3);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = pop_arg!(args, bool);
    let scalar = pop_scalar_from_bytes(&mut args)?;
    let point_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_mul * NumArgs::one();

    // Compute result = a * point (or a = a * point) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let point = point_data.get_point(&point_handle).mul(scalar);
            point_data.add_point(point)
        }
        true => {
            point_data.get_point_mut(&point_handle).mul_assign(scalar);
            point_handle.0
        }
    };

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_point_equals(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 2);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let point_data = point_context.point_data.borrow_mut();

    let b_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_equals * NumArgs::one();

    let a = point_data.get_point(&a_handle);
    let b = point_data.get_point(&b_handle);

    // Checks if a == b
    Ok(NativeResult::ok(cost, smallvec![Value::bool(a.eq(b))]))
}

pub(crate) fn native_point_neg(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 2);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = pop_arg!(args, bool);
    let point_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_neg * NumArgs::one();

    // Compute result = - point (or point = -point) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let point = point_data.get_point(&point_handle).neg();
            point_data.add_point(point)
        }
        true => {
            let neg = point_data.get_point_mut(&point_handle).neg();
            point_data.set_point(&point_handle, neg);
            point_handle.0
        }
    };

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_point_add(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 3);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = pop_arg!(args, bool);
    let b_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_add * NumArgs::one();

    // Compute result = a + b (or a = a + b) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let a = point_data.get_point(&a_handle);
            let b = point_data.get_point(&b_handle);

            let point = a.add(b);
            point_data.add_point(point)
        }
        true => {
            // NOTE: When calling Move's add_assign, Move's linear types ensure that we will never
            // get references to the same a and b RistrettoPoint, while our own invariants ensure
            // we never have two different Move RistrettoPoint constructed with the same handles.
            debug_assert!(a_handle != b_handle);
            let (a, b) = point_data.get_two_muts(&a_handle, &b_handle);

            a.add_assign(&*b);
            a_handle.0
        }
    };

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_point_sub(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 3);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let in_place = pop_arg!(args, bool);
    let b_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let a_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let cost = gas_params.point_sub * NumArgs::one();

    // Compute result = a - b (or a = a - b) and return a RistrettoPointHandle
    let result_handle = match in_place {
        false => {
            let a = point_data.get_point(&a_handle);
            let b = point_data.get_point(&b_handle);

            let point = a.sub(b);
            point_data.add_point(point)
        }
        true => {
            // NOTE: When calling Move's sub_assign, Move's linear types ensure that we will never
            // get references to the same a and b RistrettoPoint, while our own invariants ensure
            // we never have two different Move RistrettoPoint constructed with the same handles.
            debug_assert!(a_handle != b_handle);
            let (a, b) = point_data.get_two_muts(&a_handle, &b_handle);

            a.sub_assign(&*b);
            a_handle.0
        }
    };

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_basepoint_mul(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let a = pop_scalar_from_bytes(&mut args)?;

    let cost = gas_params.basepoint_mul * NumArgs::one();
    let basepoint = RISTRETTO_BASEPOINT_TABLE;
    let result = basepoint.mul(&a);
    let result_handle = point_data.add_point(result);

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

#[allow(non_snake_case)]
pub(crate) fn native_basepoint_double_mul(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 3);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let b = pop_scalar_from_bytes(&mut args)?;
    let A_handle = pop_ristretto_handle(&mut args)?;
    let a = pop_scalar_from_bytes(&mut args)?;

    // Compute result = a * A + b * BASEPOINT and return a RistrettoPointHandle
    let cost = gas_params.basepoint_double_mul * NumArgs::one();
    let A_ref = point_data.get_point(&A_handle);
    let result = RistrettoPoint::vartime_double_scalar_mul_basepoint(&a, A_ref, &b);
    let result_handle = point_data.add_point(result);

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_new_point_from_sha512(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.point_from_64_uniform_bytes * NumArgs::one()
        + gas_params.sha512_per_hash * NumArgs::one()
        + gas_params.sha512_per_byte * NumBytes::new(bytes.len() as u64);

    let result_handle = point_data.add_point(RistrettoPoint::hash_from_bytes::<Sha512>(&bytes));

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_new_point_from_64_uniform_bytes(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 0);
    assert_eq!(args.len(), 1);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let slice = pop_64_byte_slice(&mut args)?;
    let cost = gas_params.point_from_64_uniform_bytes * NumArgs::one();
    let result_handle = point_data.add_point(RistrettoPoint::from_uniform_bytes(&slice));

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

pub(crate) fn native_multi_scalar_mul(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(ty_args.len(), 2);
    assert_eq!(args.len(), 2);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let mut point_data = point_context.point_data.borrow_mut();

    let scalar_type = ty_args.pop().unwrap();
    let point_type = ty_args.pop().unwrap();

    let scalars_ref = pop_arg!(args, VectorRef);
    let points_ref = pop_arg!(args, VectorRef);

    let num = scalars_ref.len(&scalar_type)?.value_as::<u64>()? as usize;

    // parse scalars
    let mut scalars = Vec::with_capacity(num);
    for i in 0..num {
        let move_scalar = scalars_ref.borrow_elem(i, &scalar_type)?;
        let scalar = scalar_from_struct(move_scalar)?;

        scalars.push(scalar);
    }

    // parse points
    let mut points = Vec::with_capacity(num);
    for i in 0..num {
        let move_point = points_ref.borrow_elem(i, &point_type)?;
        let point_handle = get_point_handle_from_struct(move_point)?;

        points.push(point_data.get_point(&point_handle));
    }

    let result = RistrettoPoint::vartime_multiscalar_mul(scalars.iter(), points.into_iter());

    // NOTE: The variable-time multiscalar multiplication (MSM) algorithm for a size-n MSM employed in curve25519 is:
    //  1. Strauss, when n <= 190, see https://www.jstor.org/stable/2310929
    //  2. Pippinger, when n > 190, which roughly requires O(n / log_2 n) scalar multiplications
    // For simplicity, we estimate the complexity as O(n / log_2 n)
    let cost = gas_params.scalar_parse_arg * NumArgs::new(num as u64)
        + gas_params.point_parse_arg * NumArgs::new(num as u64)
        + gas_params.point_mul * NumArgs::new((num as f64 / f64::log2(num as f64)).ceil() as u64);

    let result_handle = point_data.add_point(result);

    Ok(NativeResult::ok(cost, smallvec![Value::u64(result_handle)]))
}

// =========================================================================================
// Helpers

fn get_point_handle(move_point: &StructRef) -> PartialVMResult<RistrettoPointHandle> {
    let field_ref = move_point
        .borrow_field(HANDLE_FIELD_INDEX)?
        .value_as::<Reference>()?;

    field_ref
        .read_ref()?
        .value_as::<u64>()
        .map(RistrettoPointHandle)
}

/// Get a RistrettoPointHandle struct from a Move RistrettoPoint struct.
pub fn get_point_handle_from_struct(move_point: Value) -> PartialVMResult<RistrettoPointHandle> {
    let move_struct = move_point.value_as::<StructRef>()?;

    get_point_handle(&move_struct)
}

/// Pops a RistrettoPointHandle off the argument stack
fn pop_ristretto_handle(args: &mut VecDeque<Value>) -> PartialVMResult<RistrettoPointHandle> {
    get_point_handle(&pop_arg!(args, StructRef))
}

/// Checks if 32 bytes were given as input and, if so, returns Some(CompressedRistretto).
fn compressed_point_from_bytes(bytes: Vec<u8>) -> Option<CompressedRistretto> {
    match <[u8; 32]>::try_from(bytes) {
        Ok(slice) => Some(CompressedRistretto(slice)),
        Err(_) => None,
    }
}
