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
    natives::function::{PartialVMError, StatusCode},
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

impl TryFrom<TypeTag> for Structure {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            // Should match the full path to struct `Fr` in `algebra_bls12381.move`.
            "0x1::algebra_bls12381::Fr" => Ok(Structure::BLS12381Fr),
            _ => Err(()),
        }
    }
}

/// This encodes a supported serialization formats defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    /// This refers to `format_bls12381fr_lsb()` in `algebra_bls12381.move`.
    BLS12381FrLsb,
}

impl TryFrom<TypeTag> for SerializationFormat {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            // Should match `format_bls12381fr_lsb()` in `algebra_bls12381.move`.
            "0x1::algebra_bls12381::FrFormatLsb" => Ok(SerializationFormat::BLS12381FrLsb),
            _ => Err(()),
        }
    }
}

fn feature_flag_of_single_type_basic_op(structure_opt: Option<Structure>) -> Option<FeatureFlag> {
    match structure_opt {
        Some(Structure::BLS12381Fr) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

fn feature_flag_of_serialization_format(
    structure_opt: Option<Structure>,
    format_opt: Option<SerializationFormat>,
) -> Option<FeatureFlag> {
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
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
        let type_tag = $context.type_to_type_tag($typ)?;
        Structure::try_from(type_tag).ok()
    }};
}

macro_rules! format_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ)?;
        SerializationFormat::try_from(type_tag).ok()
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

macro_rules! abort_unless_feature_flag_enabled {
    ($context:ident, $flag_opt:expr) => {
        match $flag_opt {
            Some(flag) if $context.get_feature_flags().is_enabled(flag) => {
                // Continue.
            },
            _ => {
                return Err(SafeNativeError::Abort {
                    abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
                });
            },
        }
    };
}

macro_rules! abort_unless_single_type_basic_op_enabled {
    ($context:ident, $structure_opt:expr) => {
        let flag_opt = feature_flag_of_single_type_basic_op($structure_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_serialization_format_enabled {
    ($context:ident, $structure_opt:expr, $format_opt:expr) => {
        let flag_opt = feature_flag_of_serialization_format($structure_opt, $format_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

fn abort_invariant_violated() -> PartialVMError {
    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
}

/// Try getting a pointer to the `handle`-th elements in `context` and assign it to a local variable `ptr_out`.
/// Then try casting it to a reference of `typ` and assign it in a local variable `ref_out`.
/// Abort the VM execution with invariant violation if anything above fails.
macro_rules! safe_borrow_element {
    ($context:expr, $handle:expr, $typ:ty, $ptr_out:ident, $ref_out:ident) => {
        let $ptr_out = $context
            .extensions()
            .get::<AlgebraContext>()
            .objs
            .get($handle)
            .ok_or_else(abort_invariant_violated)?
            .clone();
        let $ref_out = $ptr_out
            .downcast_ref::<$typ>()
            .ok_or_else(abort_invariant_violated)?;
    };
}

/// Macros that implements `serialize_internal()` using arkworks libraries.
macro_rules! ark_serialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $args:ident,
        $structure:expr,
        $format:expr,
        $ark_type:ty,
        $ark_ser_func:ident
    ) => {{
        $context.charge($gas_params.placeholder)?;
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_type, element_ptr, element);
        let mut buf = vec![];
        match element.$ark_ser_func(&mut buf) {
            Ok(_) => {},
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
        buf
    }};
}

fn serialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, structure_opt, format_opt);
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
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

/// Macros that implements `deserialize_internal()` using arkworks libraries.
macro_rules! ark_deserialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $structure:expr,
        $bytes:expr,
        $format:expr,
        $ark_typ:ty,
        $ark_deser_func:ident
    ) => {{
        $context.charge($gas_params.placeholder)?;
        match <$ark_typ>::$ark_deser_func($bytes) {
            Ok(element) => {
                let handle = store_element!($context, element);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            Err(ark_serialize::SerializationError::InvalidData)
            | Err(ark_serialize::SerializationError::UnexpectedFlags) => {
                Ok(smallvec![Value::bool(false), Value::u64(0)])
            },
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
    }};
}

fn deserialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, structure_opt, format_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                gas_params,
                context,
                Structure::BLS12381Fr,
                bytes,
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

macro_rules! ark_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $ark_type:ty) => {{
        $context.charge($gas_params.placeholder)?;
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $ark_type, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $ark_type, element_2_ptr, element_2);
        let new_element = element_1.add(element_2);
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn add_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => {
            ark_add_internal!(
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
            "add_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                add_internal,
            ),
        ),
        (
            "serialize_internal",
            make_safe_native(gas_params, timed_features, features, serialize_internal),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}
