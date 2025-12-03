#![no_main]
// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use aptos_types::transaction::SignedTransaction;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    data: Vec<u8>,
}

fuzz_target!(|fuzz_data: FuzzData| {
    let _ = bcs::from_bytes::<SignedTransaction>(&fuzz_data.data);
});
