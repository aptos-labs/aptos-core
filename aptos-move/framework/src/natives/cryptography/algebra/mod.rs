// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        cryptography::algebra::gas::GasParameters,
        helpers::{make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safely_pop_arg,
};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatures};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use better_any::{Tid, TidAble};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{any::Any, collections::VecDeque, hash::Hash, ops::Add, rc::Rc, sync::Arc};

pub mod gas;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C_0000;

/// This encodes an algebraic structure defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fr,
}

impl Structure {
    pub fn from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
        match type_tag.to_string().as_str() {
            // Should match the full path to struct `Fr` in `algebra_bls12381.move`.
            "0x1::algebra_bls12381::Fr" => Some(Structure::BLS12381Fr),
            _ => None,
        }
    }
}

/// This encodes a supported serialization formats defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    /// This refers to `format_bls12381fr_lsb()` in `algebra_bls12381.move`.
    BLS12381FrLsb,
}

impl TryFrom<u64> for SerializationFormat {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            // Should match `format_bls12381fr_lsb()` in `algebra_bls12381.move`.
            0x0a00000000000000 => Ok(SerializationFormat::BLS12381FrLsb),
            _ => Err(()),
        }
    }
}

#[derive(Tid, Default)]
pub struct AlgebraContext {
    objs: Vec<Rc<dyn Any>>,
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self { objs: Vec::new() }
    }
}

macro_rules! structure_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        Structure::from_type_tag(&type_tag)
    }};
}

macro_rules! store_element {
    ($context:expr, $obj:expr) => {{
        let target_vec = &mut $context.extensions_mut().get_mut::<AlgebraContext>().objs;
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }};
}

macro_rules! abort_unless_feature_enabled {
    ($context:ident, $feature:expr) => {
        if !$context.get_feature_flags().is_enabled($feature) {
            return Err(SafeNativeError::Abort {
                abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
            });
        }
    };
}

macro_rules! get_element_pointer {
    ($context:expr, $handle:expr) => {{
        $context.extensions_mut().get_mut::<AlgebraContext>().objs[$handle].clone()
    }};
}

/// Macros that implements `serialize_internal()` using arkworks libraries.
///
/// Example expansion follows for `BLS12381Fr` structure and `bls12381fr_lsb` format.
/// ```
/// {
///     let element_ptr = get_element_pointer!(context, handle);
///     let element = element_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
///     context.charge(gas_params.placeholder)?;
///     let mut buf = Vec::new();
///     element.serialize_uncompressed(&mut buf).unwrap();
///     buf
/// }
/// ```
macro_rules! ark_serialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $structure:expr,
        $handle:expr,
        $format:expr,
        $typ:ty,
        $ser_func:ident
    ) => {{
        let element_ptr = get_element_pointer!($context, $handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.placeholder)?;
        let mut buf = Vec::new();
        element.$ser_func(&mut buf).unwrap();
        buf
    }};
}

fn serialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let handle = safely_pop_arg!(args, u64) as usize;
    let format_opt = SerializationFormat::try_from(safely_pop_arg!(args, u64));
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrLsb)) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                Structure::BLS12381Fr,
                handle,
                SerializationFormat::BLS12381FrLsb,
                ark_bls12_381::Fr,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_deserialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $bytes:expr,
        $structure:expr,
        $format:expr,
        $typ:ty,
        $deser_func:ident
    ) => {{
        $context.charge($gas_params.placeholder)?;
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let handle = store_element!($context, element);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            _ => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        }
    }};
}

fn deserialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure = structure_from_ty_arg!(context, &ty_args[0]);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
    let format_opt = SerializationFormat::try_from(safely_pop_arg!(args, u64));
    match (structure, format_opt) {
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrLsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381Fr,
                SerializationFormat::BLS12381FrLsb,
                ark_bls12_381::Fr,
                deserialize_uncompressed
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_element_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_element_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.placeholder)?;
        let new_element = element_1.add(element_2);
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_add_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_add_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "deserialize_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                deserialize_internal,
            ),
        ),
        (
            "field_add_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_add_internal,
            ),
        ),
        (
            "serialize_internal",
            make_safe_native(gas_params, timed_features, features, serialize_internal),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}
