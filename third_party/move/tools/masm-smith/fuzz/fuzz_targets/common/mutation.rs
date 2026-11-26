// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Input decoding, mutation, and crossover for fuzz targets whose corpus holds encoded tests.
//!
//! The mutator mostly applies mutations that keep tests verifiable, and sometimes generates a
//! fresh test or applies libFuzzer's byte mutations instead. Crossover splices two tests at the
//! transaction level.

use arbitrary::Unstructured;
use masm_smith::{
    generate,
    mutate::{decode, encode, mutate},
    splice::splice,
    Config, MasmTest,
};

/// Decodes `data` as a stored test, or generates one from it when it is not an encoded test.
/// While `config` disables `type_name::get`, a stored test's calls to it are replaced, and a test
/// whose calls cannot be replaced is generated from `data` instead.
pub fn test_from_input(data: &[u8], config: &Config) -> Option<MasmTest> {
    decode(data)
        .and_then(|mut test| {
            (config.enable_type_name_get || test.replace_type_name_get()).then_some(test)
        })
        .or_else(|| generate(&mut Unstructured::new(data), config).ok())
}

/// Body of `fuzz_mutator!`: mutates the test in `data[..size]` in place and returns its new size.
pub fn mutate_input(
    data: &mut [u8],
    size: usize,
    max_size: usize,
    seed: u32,
    config: &Config,
) -> usize {
    // Choices are drawn from a buffer derived from `seed` so a mutation is reproducible.
    let choices = choice_bytes(seed);
    let mut unstructured = Unstructured::new(&choices);
    let mutated = match choices[0] % 16 {
        // Byte-level havoc on the encoded form; usually fails to decode and falls back to the
        // generator, which keeps fresh shapes entering the corpus.
        0 => return libfuzzer_sys::fuzzer_mutate(data, size, max_size),
        1 => generate(&mut unstructured, config).ok(),
        _ => test_from_input(&data[..size], config).and_then(|mut test| {
            mutate(&mut test, config, &mut unstructured)
                .ok()?
                .then_some(test)
        }),
    };
    match mutated {
        Some(test) => {
            let bytes = encode(&test);
            if bytes.len() > max_size {
                return libfuzzer_sys::fuzzer_mutate(data, size, max_size);
            }
            data[..bytes.len()].copy_from_slice(&bytes);
            bytes.len()
        },
        None => libfuzzer_sys::fuzzer_mutate(data, size, max_size),
    }
}

/// Body of `fuzz_crossover!`: writes a splice of the two tests to `out` and returns its size, or
/// 0 when none applies.
pub fn crossover_inputs(
    data1: &[u8],
    data2: &[u8],
    out: &mut [u8],
    seed: u32,
    config: &Config,
) -> usize {
    let choices = choice_bytes(seed);
    let spliced = match (
        test_from_input(data1, config),
        test_from_input(data2, config),
    ) {
        (Some(recipient), Some(donor)) => {
            splice(&recipient, &donor, &mut Unstructured::new(&choices))
                .ok()
                .flatten()
        },
        (None, _) | (_, None) => None,
    };
    let Some(test) = spliced else {
        return 0;
    };
    let bytes = encode(&test);
    if bytes.len() > out.len() {
        return 0;
    }
    out[..bytes.len()].copy_from_slice(&bytes);
    bytes.len()
}

/// Bytes for mutation choices, expanded from `seed` with SplitMix64.
fn choice_bytes(seed: u32) -> Vec<u8> {
    masm_smith::seeded_bytes(u64::from(seed) ^ 0x9E37_79B9_7F4A_7C15, CHOICE_BYTES)
}

/// Enough choice bytes for one mutation, including regenerating a function body or a script,
/// so the generator rarely runs out and falls back to its first option at every choice.
const CHOICE_BYTES: usize = 16 * 1024;
