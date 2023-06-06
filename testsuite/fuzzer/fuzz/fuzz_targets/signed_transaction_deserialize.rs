// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use aptos_types::transaction::SignedTransaction;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    data: Vec<u8>,
}

fuzz_target!(|fuzz_data: FuzzData| {
    let _ = bcs::from_bytes::<SignedTransaction>(&fuzz_data.data);
});