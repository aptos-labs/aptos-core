// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use ark_ec::{PairingEngine, ProjectiveCurve};
use ark_ff::PrimeField;
use better_any::{Tid, TidAble, TidExt};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::{Reference, Struct, StructRef, Value};
use num_traits::One;
use smallvec::smallvec;
use crate::natives::util::make_native_from_func;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

#[derive(Tid)]
pub struct AlgebraContext {
    object_map: HashMap<String, Vec<Rc<dyn Any>>>,
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self {
            object_map: HashMap::new()
        }
    }

    pub fn append(&mut self, obj_type: &str, obj: Rc<dyn Any>) -> usize {
        let obj_type = obj_type.to_string();
        let obj_list = self.object_map.entry(obj_type.clone()).or_insert(vec![]);
        let ret = obj_list.len();
        obj_list.push(obj);
        ret
    }

    pub fn borrow(&self, obj_type: &String, handle: usize) -> Option<Rc<dyn Any>> {
        self.object_map.get(obj_type)
            .map_or(None, |objs| objs.get(handle))
            .map_or(None, |obj| Some(obj.clone()))
    }
}

fn get_handle(struct_ref: StructRef) -> PartialVMResult<usize> {
    struct_ref.borrow_field(0)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<u64>()
        .map(|val| val as usize)
}

fn group_generator(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();

    match type_tag.as_str() {
        "0x1::algebra::BLS12_381_G1" => {
            let new_handle = context.extensions_mut().get_mut::<AlgebraContext>().append(&type_tag, Rc::new(ark_bls12_381::G1Projective::prime_subgroup_generator()));
            Ok(NativeResult::ok(
                gas_params.base,
                smallvec![Value::struct_(Struct::pack(vec![Value::u64(new_handle as u64)]))],
            ))
        }
        "0x1::algebra::BLS12_381_G2" => {
            let new_handle = context.extensions_mut().get_mut::<AlgebraContext>().append(&type_tag, Rc::new(ark_bls12_381::G2Projective::prime_subgroup_generator()));
            Ok(NativeResult::ok(
                gas_params.base,
                smallvec![Value::struct_(Struct::pack(vec![Value::u64(new_handle as u64)]))],
            ))
        }
        _ => {
            todo!()
        }
    }
}

fn equal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_1 = get_handle(pop_arg!(args, StructRef))?;
    let handle_0 = get_handle(pop_arg!(args, StructRef))?;
    match type_tag.as_str() {
        "0x1::algebra::BLS12_381_G1" => {
            let q = context.extensions().get::<AlgebraContext>().borrow(&type_tag, handle_0).unwrap();
            let element_0 = q.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
            let p = context.extensions().get::<AlgebraContext>().borrow(&type_tag, handle_1).unwrap();
            let element_1 = p.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
            let result = element_0.eq(element_1);
            Ok(NativeResult::ok(
                gas_params.base,
                smallvec![Value::bool(result)],
            ))
        }
        _ => {
            todo!()
        }
    }
}

fn pairing(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(3, ty_args.len());
    let g1_type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let g2_type_tag = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let gt_type_tag = context
        .type_to_type_tag(ty_args.get(2).unwrap())?
        .to_string();
    let handle_1 = get_handle(pop_arg!(args, StructRef))?;
    let handle_0 = get_handle(pop_arg!(args, StructRef))?;
    match (g1_type_tag.as_str(), g2_type_tag.as_str(), gt_type_tag.as_str()) {
        ("0x1::algebra::BLS12_381_G1", "0x1::algebra::BLS12_381_G2", "0x1::algebra::BLS12_381_Gt") => {
            let x = ark_bls12_381::Fr::one().into_repr();
            let q = context.extensions().get::<AlgebraContext>().borrow(&g1_type_tag, handle_0).unwrap();
            let element_0 = q.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
            let p = context.extensions().get::<AlgebraContext>().borrow(&g2_type_tag, handle_1).unwrap();
            let element_1 = p.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
            let result = ark_bls12_381::Bls12_381::pairing(element_0.into_affine(), element_1.into_affine());
            let new_handle = context.extensions_mut().get_mut::<AlgebraContext>().append("0x1::algebra::BLS12_381_Gt", Rc::new(result));
            Ok(NativeResult::ok(
                gas_params.base,
                smallvec![Value::struct_(Struct::pack(vec![Value::u64(new_handle as u64)]))],
            ))
        }
        _ => {
            todo!()
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "group_generator",
            make_native_from_func(gas_params.clone(), group_generator),
        ),
        (
            "equal",
            make_native_from_func(gas_params.clone(), equal),
        ),
        (
            "pairing",
            make_native_from_func(gas_params.clone(), pairing),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}
