// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

fn main() {
    // Test for ripemd160, output_length < 256
    let ripemd = diem_crypto::hkdf::Hkdf::<ripemd160::Ripemd160>::extract(None, &[]);
    assert!(ripemd.is_ok());
}
