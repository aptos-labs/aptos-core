// Copyright © Aptos Foundation

use num_bigint::BigUint;
use once_cell::sync::Lazy;

//
// Batch sizes for tests and benchmarks
//

// Best case
pub const BEST_CASE_THRESHOLD: usize = 333;
pub const BEST_CASE_N: usize = 1_000;

// Worst case
pub const WORST_CASE_THRESHOLD: usize = 3_333;
pub const WORST_CASE_N: usize = 10_000;

pub const OUR_THRESHOLD: usize = WORST_CASE_THRESHOLD;
pub const OUR_N: usize = WORST_CASE_N;

/// Small batch sizes used during benchmarking FFT roots-of-unity computations, hashing & polynomial multiplications
pub const SMALL_SIZES: [usize; 13] = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Large batch sizes used during benchmarking multiexps & FFTs
pub const LARGE_SIZES: [usize; 3] = [8192, 16_384, 32_768];

//
// DSTs and seeds
//

/// TODO(rand_core_hell): Domain-separator for our `rand_core_hell` randomness generation.
pub const DST_RAND_CORE_HELL: &[u8; 24] = b"APTOS_RAND_CORE_HELL_DST";

/// "Nothing up my sleeve" message & domain-separator tag (DST) for the hash-to-curve operation used
/// to pick our PVSS public parameters (group elements) as `hash_to_curve(seed, dst, group_element_name)`.
pub const DST_PVSS_PUBLIC_PARAMS: &[u8; 32] = b"APTOS_DISTRIBUTED_RANDOMNESS_DST";
pub const SEED_PVSS_PUBLIC_PARAMS: &[u8; 33] = b"APTOS_DISTRIBUTED_RANDOMNESS_SEED";

//
// Sizes
//

/// The size in bytes of a compressed G1 point (efficiently deserializable into projective coordinates)
pub const G1_PROJ_NUM_BYTES: usize = 48;

/// The size in bytes of a compressed G2 point (efficiently deserializable into projective coordinates)
pub const G2_PROJ_NUM_BYTES: usize = 96;

/// The size in bytes of a scalar.
pub const SCALAR_NUM_BYTES: usize = 32;

// TODO(rand_core_hell): Remove this once rand_core_hell is fixed.
pub(crate) const SCALAR_FIELD_ORDER: Lazy<BigUint> =
    Lazy::new(crate::utils::biguint::get_scalar_field_order_as_biguint);
