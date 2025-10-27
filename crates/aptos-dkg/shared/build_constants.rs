// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
pub const CHUNK_SIZE: u32 = 32; // TODO: this is not used for the build script, but it IS used at compile time... not sure why that's not spotted
pub const TABLE_SIZE: u32 = 16; // Table size in bits, so the number of entries is 1 << TABLE_SIZE

/// "Nothing up my sleeve" domain-separator tag (DST) for the hash-to-curve operation used
/// to pick our PVSS public parameters (group elements) as `hash_to_curve(seed, dst, group_element_name)`.
pub const DST_PVSS_PUBLIC_PARAMS: &[u8; 32] = b"APTOS_DISTRIBUTED_RANDOMNESS_DST";
/// "Nothing up my sleeve" seed to deterministically-derive the public parameters.
pub const SEED_PVSS_PUBLIC_PARAMS: &[u8; 33] = b"APTOS_DISTRIBUTED_RANDOMNESS_SEED";
