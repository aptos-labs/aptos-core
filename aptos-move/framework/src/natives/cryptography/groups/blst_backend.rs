// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;
use better_any::{Tid, TidAble};
use blst::{blst_fp12, blst_fr, blst_p1, blst_p2, blst_p2_affine, blst_p2_affine_compress, blst_p2_to_affine};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use smallvec::smallvec;
use crate::natives::cryptography::groups::abort_codes::E_UNKNOWN_GROUP;

#[derive(Debug, Clone)]
pub struct Bls12381GasParameters {
    pub fp12_to_bendian: InternalGasPerArg,
    pub p2_affine_compress: InternalGasPerArg,
    pub pairing_product_base: InternalGas,
    pub pairing_product_per_pair: InternalGasPerArg,
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub bls12_381: Bls12381GasParameters,
}

#[derive(Tid)]
pub struct Bls12381Context {
    fr_store: Vec<blst_fr>,
    g1_point_store: Vec<blst_p1>,
    g2_point_store: Vec<blst_p2>,
    gt_point_store: Vec<blst_fp12>,
}

impl Bls12381Context {
    pub fn new() -> Self {
        Self {
            fr_store: vec![],
            g1_point_store: vec![],
            g2_point_store: vec![],
            gt_point_store: vec![],
        }
    }

    pub fn add_scalar(&mut self, scalar: blst_fr) -> usize {
        let ret = self.fr_store.len();
        self.fr_store.push(scalar);
        ret
    }

    pub fn get_scalar(&self, handle: usize) -> &blst_fr {
        self.fr_store.get(handle).unwrap()
    }

    pub fn add_g1_point(&mut self, p0: blst_p1) -> usize {
        let ret = self.g1_point_store.len();
        self.g1_point_store.push(p0);
        ret
    }

    pub fn get_g1_point(&self, handle: usize) -> &blst_p1 {
        self.g1_point_store.get(handle).unwrap()
    }

    pub fn add_g2_point(&mut self, p0: blst_p2) -> usize {
        let ret = self.g2_point_store.len();
        self.g2_point_store.push(p0);
        ret
    }

    pub fn get_g2_point(&self, handle: usize) -> &blst_p2 {
        self.g2_point_store.get(handle).unwrap()
    }

    pub fn add_gt_point(&mut self, point: blst_fp12) -> usize {
        let ret = self.gt_point_store.len();
        self.gt_point_store.push(point);
        ret
    }

    pub fn get_gt_point(&self, handle: usize) -> &blst_fp12 {
        self.gt_point_store.get(handle).unwrap()
    }
}
