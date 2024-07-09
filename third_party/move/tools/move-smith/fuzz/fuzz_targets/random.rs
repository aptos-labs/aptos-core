// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use move_smith::{config::Config, utils::run_transactional_test, CodeGenerator, MoveSmith};
use rand::{rngs::StdRng, Rng, SeedableRng};

const INITIAL_BUFFER_SIZE: usize = 1024 * 4;
const MAX_BUFFER_SIZE: usize = 1024 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let mut seed = [0u8; 8];
    seed.copy_from_slice(&data[0..8]);
    let mut rng = StdRng::seed_from_u64(u64::from_be_bytes(seed));

    let mut buffer_size = INITIAL_BUFFER_SIZE;
    let mut buffer = vec![];

    let code = loop {
        if buffer_size > buffer.len() {
            let diff = buffer_size - buffer.len();
            let mut new_buffer = vec![0u8; diff];
            rng.fill(&mut new_buffer[..]);
            buffer.extend(new_buffer);
        }

        let mut smith = MoveSmith::default();
        let u = &mut Unstructured::new(&buffer);
        match smith.generate(u) {
            Ok(()) => break smith.get_compile_unit().emit_code(),
            Err(_) => {
                if buffer_size >= MAX_BUFFER_SIZE {
                    panic!(
                        "Failed to generate a module with {} bytes input",
                        buffer_size
                    );
                }
            },
        };
        buffer_size *= 2;
    };

    run_transactional_test(code, &Config::default()).unwrap();
});
