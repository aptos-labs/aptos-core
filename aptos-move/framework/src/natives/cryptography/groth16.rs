// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module is cool.

use crate::natives::util::make_native_from_func;
use bellman::domain::Scalar;
use bellman::groth16::{prepare_verifying_key, Proof, VerifyingKey};
use better_any::{Tid, TidAble};
use bls12_381::Bls12;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::{Reference, Struct, VectorRef};
use move_vm_types::values::{StructRef, Value};
use smallvec::smallvec;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

fn new_verifying_key_from_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let x: VectorRef = pop_arg!(arguments, VectorRef);
    let byte_count = x.len(&Type::U8)?.value_as::<u64>()? as usize;
    let mut bytes = Vec::with_capacity(byte_count);
    for i in 0..byte_count {
        let y = x.borrow_elem(i, &Type::U8)?;
        let z = y.value_as::<Reference>()?;
        let w = z.read_ref()?.value_as::<u8>()?;
        bytes.push(w);
    }
    println!("{:?}", bytes.as_slice());
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::struct_(Struct::pack(vec![Value::vector_u8(bytes),]))],
    ))
}

fn new_proof_from_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let x: VectorRef = pop_arg!(arguments, VectorRef);
    let byte_count = x.len(&Type::U8)?.value_as::<u64>()? as usize;
    let mut bytes = Vec::with_capacity(byte_count);
    for i in 0..byte_count {
        let y = x.borrow_elem(i, &Type::U8)?;
        let z = y.value_as::<Reference>()?;
        let w = z.read_ref()?.value_as::<u8>()?;
        bytes.push(w);
    }
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::struct_(Struct::pack(vec![Value::vector_u8(bytes),]))],
    ))
}

fn ref_vector_u8_to_bytes(r: VectorRef) -> Vec<u8> {
    let byte_count = r.len(&Type::U8).unwrap().value_as::<u64>().unwrap() as usize;
    let mut bytes = Vec::with_capacity(byte_count);
    for i in 0..byte_count {
        let y = r.borrow_elem(i, &Type::U8).unwrap();
        let z = y.value_as::<Reference>().unwrap();
        let w = z.read_ref().unwrap().value_as::<u8>().unwrap();
        bytes.push(w);
    }
    bytes
}

#[derive(Tid)]
pub struct BellmanContext {
    proof_store: Vec<Proof<Bls12>>,
    vk_store: Vec<VerifyingKey<Bls12>>,
}

impl BellmanContext {
    pub fn new() -> Self {
        let proof_bytes = hex::decode("8ae17f6100bc0ce469b6c25b533ba8bce5a05ddc223344b390d0d135afaad09fcc9596b0204842228177a2a3a32a9466b2dba57ad279de822b97bea69e637288ab116f772f2338079b03a7ee09c329ad9f628f4c1c34f4b5f78dd419429a891e0509f34ce25b0b9a7524f2c7716e1605490d86f9b1478208238c2a7e208726d74b9039dfadabeb06e6bb8f3d53aa2cdfaab40bc03eb218367ab6fe74e5c767a4ed217464523898939f830733ee8f063b3dee922c8a6e7f8bb252327d86e2e5d8").unwrap();
        let proof = bellman::groth16::Proof::read(proof_bytes.as_slice()).unwrap();
        let vk_bytes = hex::decode("09b9da0a00f798fceb5f2e31138c49182665c077dbfaead1b4cc71957b72a83511dbdd8b3a4631c1698877508bb0bbf30cab949482993c098f76225a622a015f490cc84ee9b4a46c258398dbdcd1d51ed944b85ae4336d7faae1c4fb9d17efc21348fcef658887712a39743b6e8b6060ddd34474696d9ad345e1c5c4822c826c590c296b1cd8223c57beb8da433c53760300747169e376dd8d1e493e3482a9d41f88c9734157b633fc24358a2b62b76fcf7162a7469ccef8842f70af5d5b3ab917d65b72fb3136d477ea1ad98d37244e59d488fca4a4d3539bf55eb26443502107e75114cf8c68687d08f2268941fe14098f1de415969acd4594d57b04db76397dcb42ff2eaa6bbb34e53eaf84b538924e0b9958a46210d5f8fdcacb1385ffc7008273101376bea162aef59dec1dd41a6a565032f3978492db85c7673b387505de467c3fe1d7e32321159151f14ef58713371db273a23ac6cca4782ebc6a1b5eb16e33f9b8cd7d3bd72c64f1d4ae270c9febc36830081d927d15bea53177418102c8928d29b2e6f3f48e95c81c6f2763f6c22b3edbb269b5bcbaceb85b2411550fd13eb215c8e844257ab29079d98bc504d34d13be087d2eed586e3b69a8ac45547558b37cb456b00db7aee29a6f574be8c9539b7e66b5afbe18c2b0bfa3c5c813c5d36d7a66c46642e9254d690f5a69b07ccb8d5794b8e9c510f69a86ccab81a6c620d91e49a7ef164a6dd0dcde29fd17a27cb0c05fba57699a5b0685f0ef077fa7cb3872ea5c2f87fd1db68dfca7fdccd8480d64e922a184fa4c047780cba81424c1737ae80142df892a6d5c2d62e1105d39fd9e50c2c20b11480629ed484b4b35e7c918ee2da5a8bc42ccb7c30427154c695e24cd07a0ff1dd0efabc39d11685ea21f5518cd4487f377bb27cd847544cd4f5ae33f4ede54338190f8f8932811259d7fd5a0186d97ce93a666d22b9471c4ba8f1145b9fad882b7e8a60fe2d27d03da796aec5303912a1d18d84f188f010790dea53dcfa92322d127c2e83b2d690354db790db33619ad493f53126b188bd551860852af662210e26ce26a96720afce7463641760a4406f85a01f5af388a0971f5c344f9517d41a076b98d22d738764bd2213f3c52897c736cdae3da2a15c4e0a94e0fbdaf38e75331b828b5a23785df6467e67d2e106ada4cdf6e8c39e3bc342eaf2ae44337040e64c747da8e0000000212181cd223ddfca240e6d882836ef3ba8daccd7f69a82a9af633b106e6ccdf54032f46cad59172e8e0069061215637170707c2f059e3c9bedbfff87bc7055463736a91a5fcce53759a126ded510b5a414528ef054592bd9ff98bf705ec53ff680686f5ad27540f72b50947dc625fdf48912163a9e7270d69c6fd2aa9c64efddeb0155c37c80c1ee84688742609f9fd82013486bfe130f43c0d0d6afa5921796fad499bc0723d2c29af56c43bea3899ff10b700d34d7c40321260de33e630eefa").unwrap();
        let vk = bellman::groth16::VerifyingKey::read(vk_bytes.as_slice()).unwrap();
        Self {
            proof_store: vec![proof],
            vk_store: vec![vk],
        }
    }
}

fn get_handle(r: StructRef) -> usize {
    0
}

fn verify_proof(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut _arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bellman_ctxt = _context.extensions().get::<BellmanContext>();
    let proof_handle: usize = get_handle(pop_arg!(_arguments, StructRef));
    let proof = bellman_ctxt.proof_store.get(proof_handle).unwrap();

    let s1_bytes =
        hex::decode("555330d89b0762b4bef49da37cc7289e731b1b34fd652f6aeefa1b1eadb23434").unwrap();
    let s1_bytes = <[u8; 32]>::try_from(s1_bytes.as_slice()).unwrap();
    let s1 = bls12_381::Scalar::from_bytes(&s1_bytes).unwrap();
    let public_inputs = vec![s1];

    let vk_handle: usize = get_handle(pop_arg!(_arguments, StructRef));
    let vk = bellman_ctxt.vk_store.get(vk_handle).unwrap();
    let pvk = prepare_verifying_key(vk);

    let accepted = match bellman::groth16::verify_proof::<bls12_381::Bls12>(
        &pvk,
        proof,
        public_inputs.as_slice(),
    ) {
        Ok(()) => true,
        _ => false,
    };

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(accepted)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "new_verifying_key_from_bytes",
            make_native_from_func(gas_params.clone(), new_verifying_key_from_bytes),
        ),
        (
            "new_proof_from_bytes",
            make_native_from_func(gas_params.clone(), new_proof_from_bytes),
        ),
        (
            "verify_proof",
            make_native_from_func(gas_params.clone(), verify_proof),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
