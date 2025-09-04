// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_protos::indexer::v1::TransactionsInStorage;
use prost::Message;
use std::io::Read;

pub fn decompress_fixture(bytes: &[u8]) -> TransactionsInStorage {
    let mut decompressor = lz4::Decoder::new(bytes).expect("Lz4 decompression failed.");
    let mut decompressed = Vec::new();
    decompressor
        .read_to_end(&mut decompressed)
        .expect("Lz4 decompression failed.");
    TransactionsInStorage::decode(decompressed.as_slice()).expect("Failed to parse transaction")
}

#[allow(dead_code)]
pub fn load_tvelor_fixture() -> TransactionsInStorage {
    let data = include_bytes!(
        "../../fixtures/compressed_files_lz4_00008bc1d5adcf862d3967c1410001fb_705101000.pb.lz4"
    );
    decompress_fixture(data)
}

#[allow(dead_code)]
pub fn load_random_april_3mb_fixture() -> TransactionsInStorage {
    let data = include_bytes!(
        "../../fixtures/compressed_files_lz4_0013c194ec4fdbfb8db7306170aac083_445907000.pb.lz4"
    );
    decompress_fixture(data)
}

pub fn load_graffio_fixture() -> TransactionsInStorage {
    let data = include_bytes!(
        "../../fixtures/compressed_files_lz4_f3d880d9700c70d71fefe71aa9218aa9_301616000.pb.lz4"
    );
    decompress_fixture(data)
}
