// Copyright (c) 2024 Supra.

use std::collections::VecDeque;
use std::sync::Arc;
use keccak_hash::{keccak, H256};
use move_vm_types::values::Value;
use move_vm_types::loaded_data::runtime_types::Type;
use eth_trie::{EthTrie, Trie, DB};
use eth_trie::MemoryDB;
use smallvec::{smallvec, SmallVec};
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use aptos_native_interface::{safely_pop_arg, safely_pop_vec_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult};
use aptos_gas_schedule::gas_params::natives::aptos_framework::{ETH_TRIE_PROOF_BASE, ETH_TRIE_PROOF_DECODE_BASE, ETH_TRIE_PROOF_DECODE_PER_BYTE, ETH_TRIE_PROOF_HASH_BASE, ETH_TRIE_PROOF_HASH_PER_BYTE};
#[cfg(feature = "testing")]
use rand::Rng;

/// The minimum length (in bytes) for an encoded node to be stored by hash.
const HASHED_LENGTH: usize = 32;

/// Native function for verifying an Ethereum Merkle Patricia Trie proof.
///
/// # Arguments
///
///   1. `proof`: A vector of RLP–encoded trie nodes (`Vec<Vec<u8>>`)
///   2. `key`: The key to be looked up (`Vec<u8>`)
///   3. `root`: The trie root hash (a 32-byte vector, i.e. `Vec<u8>`)
///
/// # Returns
///
/// A tuple of `(bool, vector<u8>)` where:
///   - If the proof is valid and the key exists, returns `(true, value)` (with `value` being the found value) (i.e. inclusion proof.)
///   - Otherwise, returns `(true, empty vector)` to show the proof is valid and key does not exist (i.e. exclusion proof.)
///   - Returns `(false, empty vector)` to show that proof is invalid
pub fn native_verify_proof_eth_trie(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(ETH_TRIE_PROOF_BASE)?;

    let proof: Vec<Vec<u8>> = safely_pop_vec_arg!(arguments, Vec<u8>);
    let key: Vec<u8> = safely_pop_arg!(arguments, Vec<u8>);
    let root: Vec<u8> = safely_pop_arg!(arguments, Vec<u8>);

    let total_proof_bytes = proof.iter().map(|node| node.len() as u64).sum::<u64>();
    context.charge(
            (ETH_TRIE_PROOF_HASH_BASE + ETH_TRIE_PROOF_DECODE_BASE) * NumArgs::new(proof.len() as u64) +
            (ETH_TRIE_PROOF_HASH_PER_BYTE + ETH_TRIE_PROOF_DECODE_PER_BYTE) * NumBytes::new(total_proof_bytes))?;

    // Convert the root (a Vec<u8>) into a H256 hash.
    let root_hash = H256::from_slice(&root);

    // Build a temporary in–memory DB from the proof nodes.
    let memdb = MemoryDB::new(true);
    let db = Arc::new(memdb);
    for node_encoded in proof.iter() {
        let hash: H256 = keccak(&node_encoded).as_fixed_bytes().into();
        // Insert the node if it is the root or if its encoded length is at least HASHED_LENGTH.
        if root_hash.eq(&hash) || node_encoded.len() >= HASHED_LENGTH {
            db.insert(hash.as_bytes(), node_encoded.clone()).unwrap();
        }
    }

    // Create an EthTrie instance using the temporary DB and the given root.
    let trie = EthTrie::new(db).at_root(root_hash);

    // Call the trie’s get method.
    let value_opt = match trie.get(key.as_slice()) {
        Ok(value_opt) => value_opt,
        Err(_) => {
            return Ok(smallvec![Value::bool(false), Value::vector_u8(vec![])]);
        },
    };

    // Convert the Option<Vec<u8>> result into a tuple (bool, vector).
    // If Some(val) is returned, we output (true, val); if None, we output (true, empty vector).
    let result = match value_opt {
        Some(val) => smallvec![Value::bool(true), Value::vector_u8(val)],
        None => smallvec![Value::bool(true), Value::vector_u8(vec![])],
    };

    Ok(result)
}

#[cfg(feature = "testing")]
pub fn native_generate_random_trie(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // 1. Pop argument: number of random keys to insert
    let num_keys: u64 = safely_pop_arg!(arguments, u64);

    // 3. Build a random trie
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = EthTrie::new(Arc::clone(&memdb));
    let mut rng = rand::thread_rng();

    // For storing the final data
    let mut all_key_proofs: Vec<Value> = Vec::new();
    let mut all_keys: Vec<Vec<u8>> = Vec::new();

    for _ in 0..num_keys {
        // Generate a random key (and use the same bytes as the value)
        let len: u8 = rng.gen_range(2, 30);
        let random_key: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
        // Insert into trie
        trie.insert(&random_key, &random_key).unwrap();
        all_keys.push(random_key.clone());
    }

    // Grab the final root
    let root = trie.root_hash().unwrap();

    for k in &all_keys {
        let proof = trie.get_proof(k).unwrap();

        // Build a "Value::Vector" representing `[ key, proof_node1, proof_node2, ... ]`
        // 1) Key as a Value::vector_u8
        let mut subvec_items: Vec<Value> = Vec::with_capacity(1 + proof.len());
        subvec_items.push(Value::vector_u8(k.clone()));
        // 2) Each proof node as a Value::vector_u8
        for node in &proof {
            subvec_items.push(Value::vector_u8(node.clone()));
        }
        let subvec_val = Value::vector_for_testing_only(subvec_items);

        all_key_proofs.push(subvec_val);
    }

    // Build the top-level vector
    let big_vector_val = Value::vector_for_testing_only(all_key_proofs);

    // Return `(root_as_vector_u8, big_vector_of_vectors)`
    // so Move sees a pair:  (vector<u8>, vector<vector<vector<u8>>>)
    Ok(smallvec![
        Value::vector_u8(root.as_bytes().to_vec()),
        big_vector_val
    ])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    #[cfg(feature = "testing")]
    natives.extend([(
        "generate_random_trie",
        native_generate_random_trie as RawSafeNative,
    )]);

    natives.extend([
        (
            "native_verify_proof_eth_trie",
            native_verify_proof_eth_trie as RawSafeNative,
        ),
    ]);

    builder.make_named_natives(natives)
}
