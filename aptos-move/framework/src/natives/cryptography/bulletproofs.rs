// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::ristretto255_point::{
    get_point_handle, NativeRistrettoPointContext,
};
use crate::natives::util::make_native_from_func;
use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use bulletproofs::{BulletproofGens, PedersenGens};
use curve25519_dalek::ristretto::CompressedRistretto;
use merlin::Transcript;
use move_deps::move_binary_format::errors::PartialVMResult;
use move_deps::move_core_types::gas_algebra::{InternalGasPerArg, NumArgs};
use move_deps::move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_deps::move_vm_types::loaded_data::runtime_types::Type;
use move_deps::move_vm_types::natives::function::NativeResult;
use move_deps::move_vm_types::pop_arg;
use move_deps::move_vm_types::values::{StructRef, Value};
use once_cell::sync::Lazy;
use smallvec::smallvec;
use std::collections::VecDeque;

/// Abort code when deserialization fails (0x01 == INVALID_ARGUMENT)
/// NOTE: This must match the code in the Move implementation
pub mod abort_codes {
    pub const NFE_DESERIALIZE_RANGE_PROOF: u64 = 0x01_0001;
}

/// Default Pedersen commitment key compatible with the default Bulletproof verification API.
static RANGE_PROOF_PEDERSEN_GENERATORS: Lazy<PedersenGens> = Lazy::new(PedersenGens::default);

fn native_verify_range_proof_custom_ck(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 6);

    let point_context = context.extensions().get::<NativeRistrettoPointContext>();
    let point_data = point_context.point_data.borrow_mut();

    let dst = pop_arg!(args, Vec<u8>);
    let bit_length = pop_arg!(args, u64) as usize;
    let proof_bytes = pop_arg!(args, Vec<u8>);
    let rand_base_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let val_base_handle = get_point_handle(&pop_arg!(args, StructRef))?;
    let comm_bytes = pop_arg!(args, Vec<u8>);

    let comm_point = CompressedRistretto::from_slice(comm_bytes.as_slice());
    let rand_base = point_data.get_point(&rand_base_handle);
    let val_base = point_data.get_point(&val_base_handle);

    let pg = PedersenGens {
        B: *val_base,
        B_blinding: *rand_base,
    };

    gas_params.verify_range_proof(&comm_point, &pg, &proof_bytes[..], bit_length, dst)
}

fn native_verify_range_proof(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 4);

    let dst = pop_arg!(args, Vec<u8>);
    let bit_length = pop_arg!(args, u64) as usize;
    let proof_bytes = pop_arg!(args, Vec<u8>);
    let comm_bytes = pop_arg!(args, Vec<u8>);
    let comm_point = CompressedRistretto::from_slice(comm_bytes.as_slice());

    gas_params.verify_range_proof(
        &comm_point,
        &RANGE_PROOF_PEDERSEN_GENERATORS,
        &proof_bytes[..],
        bit_length,
        dst,
    )
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub per_rangeproof_deserialize: InternalGasPerArg,
    pub per_bit_rangeproof_verify: InternalGasPerArg,
}

impl GasParameters {
    /// Helper function to gas meter and verify a single Bulletproof range proof for a Pedersen
    /// commitment with `pc_gens` as its commitment key.
    fn verify_range_proof(
        &self,
        comm_point: &CompressedRistretto,
        pc_gens: &PedersenGens,
        proof_bytes: &[u8],
        bit_length: usize,
        dst: Vec<u8>,
    ) -> PartialVMResult<NativeResult> {
        static BULLETPROOF_GENERATORS: Lazy<BulletproofGens> =
            Lazy::new(|| BulletproofGens::new(MAX_RANGE_BITS, 1));

        let mut cost = self.per_rangeproof_deserialize * NumArgs::one();

        let range_proof = match bulletproofs::RangeProof::from_bytes(proof_bytes) {
            Ok(proof) => proof,
            Err(_) => {
                return Ok(NativeResult::err(
                    cost,
                    abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
                ))
            }
        };

        // The (Bullet)proof size is $\log_2(num_bits)$ and its verification time is $O(num_bits)$
        cost += self.per_bit_rangeproof_verify * NumArgs::new(bit_length as u64);

        let mut ver_trans = Transcript::new(dst.as_slice());

        let success = range_proof
            .verify_single(
                &BULLETPROOF_GENERATORS,
                pc_gens,
                &mut ver_trans,
                comm_point,
                bit_length,
            )
            .is_ok();

        Ok(NativeResult::ok(cost, smallvec![Value::bool(success)]))
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "verify_range_proof_custom_ck_internal",
            make_native_from_func(gas_params.clone(), native_verify_range_proof_custom_ck),
        ),
        (
            "verify_range_proof_internal",
            make_native_from_func(gas_params, native_verify_range_proof),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
