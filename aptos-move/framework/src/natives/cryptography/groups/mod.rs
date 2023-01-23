// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0


use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg};
use std::rc::Rc;
use ark_bls12_381::{Parameters};
use ark_ec::{PairingEngine, ProjectiveCurve};
use ark_ff::{Field, Fp256, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use better_any::{Tid, TidAble, TidExt};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use num_traits::One;
use once_cell::sync::Lazy;
use sha2::Sha256;
use smallvec::smallvec;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_types::on_chain_config::FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS;
use crate::natives::util::make_native_from_func;


macro_rules! abort_if_feature_disabled {
    ($context:expr, $feature:expr) => {
        if !$context.extensions().get::<GroupContext>().features.is_enabled($feature) {
            return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
        }
    };
}

macro_rules! borrow_bls12_381_g1 {
    ($context:expr, $handle:expr) => {
        $context.extensions().get::<GroupContext>().bls12_381_g1_elements.get($handle).unwrap()
    }
}

macro_rules! borrow_bls12_381_g2 {
    ($context:expr, $handle:expr) => {
        $context.extensions().get::<GroupContext>().bls12_381_g2_elements.get($handle).unwrap()
    }
}

macro_rules! borrow_bls12_381_gt {
    ($context:expr, $handle:expr) => {
        $context.extensions().get::<GroupContext>().bls12_381_fq12_elements.get($handle).unwrap()
    }
}

macro_rules! borrow_ristretto255 {
    ($context:expr, $handle:expr) => {
        $context.extensions().get::<GroupContext>().ristretto255_elements.get($handle).unwrap()
    }
}

macro_rules! store_bls12_381_g1 {
    ($context:expr, $new_point:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_g1_elements.len();
        inner_ctxt.bls12_381_g1_elements.push($new_point);
        ret
    }}
}

macro_rules! store_bls12_381_g2 {
    ($context:expr, $new_point:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_g2_elements.len();
        inner_ctxt.bls12_381_g2_elements.push($new_point);
        ret
    }}
}

macro_rules! store_bls12_381_gt {
    ($context:expr, $new_element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_fq12_elements.len();
        inner_ctxt.bls12_381_fq12_elements.push($new_element);
        ret
    }}
}

macro_rules! store_ristretto255 {
    ($context:expr, $new_point:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.ristretto255_elements.len();
        inner_ctxt.ristretto255_elements.push($new_point);
        ret
    }}
}

macro_rules! group_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        structure_from_type_tag(&type_tag)
    }}
}

pub const NOT_IMPLEMENTED: u64 = 2; //TODO: update it to be aligned with `error.move`.

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

#[derive(Tid)]
pub struct GroupContext {
    features: Features,
    bls12_381_g1_elements: Vec<ark_bls12_381::G1Projective>,
    bls12_381_g2_elements: Vec<ark_bls12_381::G2Projective>,
    bls12_381_fq12_elements: Vec<ark_bls12_381::Fq12>,
    ristretto255_elements: Vec<curve25519_dalek::ristretto::RistrettoPoint>,
}

impl GroupContext {
    pub fn new(features: Features) -> Self {
        Self {
            features,
            bls12_381_g1_elements: vec![],
            bls12_381_g2_elements: vec![],
            bls12_381_fq12_elements: vec![],
            ristretto255_elements: vec![]
        }
    }
}

static BLS12381_GT_GENERATOR: Lazy<ark_bls12_381::Fq12> = Lazy::new(||{
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    ark_bls12_381::Fq12::deserialize(buf.as_slice()).unwrap()
});

fn generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let group_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[0]);
    match group_opt {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let point = ark_bls12_381::G1Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g1!(context, point);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(handle as u64)]))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let point = ark_bls12_381::G2Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g2!(context, point);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(handle as u64)]))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = BLS12381_GT_GENERATOR.clone();
            let handle = store_bls12_381_gt!(context, element);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(handle as u64)]))
        }
        Some(Structure::Ristretto255) => {
            abort_if_feature_disabled!(context, FeatureFlag::RISTRETTO255_GROUP);
            let point = curve25519_dalek::ristretto::RistrettoPoint::default();
            let handle = store_ristretto255!(context, point);
            context.extensions_mut().get_mut::<GroupContext>().ristretto255_elements.push(point);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(handle as u64)]))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

fn eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    let group_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[0]);
    match group_opt {
        Some(Structure::BLS12_381_G1) => {
            let point_1 = borrow_bls12_381_g1!(context, handle_1);
            let point_2 = borrow_bls12_381_g1!(context, handle_2);
            let result = point_1.eq(point_2);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::bool(result)]))
        }
        Some(Structure::BLS12_381_G2) => {
            let point_1 = borrow_bls12_381_g2!(context, handle_1);
            let point_2 = borrow_bls12_381_g2!(context, handle_2);
            let result = point_1.eq(point_2);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::bool(result)]))
        }
        Some(Structure::BLS12_381_Gt) => {
            let element_1 = borrow_bls12_381_gt!(context, handle_1);
            let element_2 = borrow_bls12_381_gt!(context, handle_2);
            let result = element_1.eq(element_2);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::bool(result)]))
        }
        Some(Structure::Ristretto255) => {
            let point_1 = borrow_ristretto255!(context, handle_1);
            let point_2 = borrow_ristretto255!(context, handle_2);
            let result = point_1.eq(point_2);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::bool(result)]))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

fn add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    let group_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[0]);
    match group_opt {
        Some(Structure::BLS12_381_G1) => {
            let point_1 = borrow_bls12_381_g1!(context, handle_1);
            let point_2 = borrow_bls12_381_g1!(context, handle_2);
            let new_point = point_1.add(point_2);
            let new_handle = store_bls12_381_g1!(context, new_point);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(new_handle as u64)]))
        }
        Some(Structure::BLS12_381_G2) => {
            let point_1 = borrow_bls12_381_g2!(context, handle_1);
            let point_2 = borrow_bls12_381_g2!(context, handle_2);
            let new_point = point_1.add(point_2);
            let new_handle = store_bls12_381_g2!(context, new_point);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(new_handle as u64)]))
        }
        Some(Structure::BLS12_381_Gt) => {
            let element_1 = borrow_bls12_381_gt!(context, handle_1);
            let element_2 = borrow_bls12_381_gt!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(new_handle as u64)]))
        }
        Some(Structure::Ristretto255) => {
            let element_1 = borrow_ristretto255!(context, handle_1);
            let element_2 = borrow_ristretto255!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_ristretto255!(context, new_element);
            Ok(NativeResult::ok(gas_params.base, smallvec![Value::u64(new_handle as u64)]))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn pairing_product_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(3, ty_args.len());
    let g1_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[0]);
    let g2_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[1]);
    let gt_opt: Option<Structure> = group_from_ty_arg!(context, &ty_args[2]);
    let g2_handles = pop_arg!(args, Vec<u64>);
    let g1_handles = pop_arg!(args, Vec<u64>);
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Gt)) => {
            let g1_prepared: Vec<ark_ec::models::bls12::g1::G1Prepared<Parameters>> = g1_handles
                .iter()
                .map(|&handle| {
                    let element = borrow_bls12_381_g1!(context, handle as usize);
                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();
            let g2_prepared: Vec<ark_ec::models::bls12::g2::G2Prepared<Parameters>> = g2_handles
                .iter()
                .map(|&handle| {
                    let element = borrow_bls12_381_g2!(context, handle as usize);
                    ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();

            let z: Vec<(
                ark_ec::models::bls12::g1::G1Prepared<Parameters>,
                ark_ec::models::bls12::g2::G2Prepared<Parameters>,
            )> = g1_prepared
                .into_iter()
                .zip(g2_prepared.into_iter())
                .collect();
            let pairing_result = ark_bls12_381::Bls12_381::product_of_pairings(z.as_slice());
            let result_handle = store_bls12_381_gt!(context, pairing_result);
            Ok(NativeResult::ok(
                gas_params.base,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "generator_internal",
            make_native_from_func(gas_params.clone(), generator_internal),
        ),
        (
            "eq_internal",
            make_native_from_func(gas_params.clone(), eq_internal),
        ),
        (
            "pairing_product_internal",
            make_native_from_func(gas_params.clone(), pairing_product_internal),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12_381_G1,
    BLS12_381_G2,
    BLS12_381_Gt,
    Ristretto255,
}

fn structure_from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
    match type_tag.to_string().as_str() {
        "0x1::groups::BLS12_381_G1" => Some(Structure::BLS12_381_G1),
        "0x1::groups::BLS12_381_G2" => Some(Structure::BLS12_381_G2),
        "0x1::groups::BLS12_381_Gt" => Some(Structure::BLS12_381_Gt),
        "0x1::groups::Ristretto255" => Some(Structure::Ristretto255),
        _ => None
    }
}
