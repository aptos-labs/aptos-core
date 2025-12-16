// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![no_main]
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
