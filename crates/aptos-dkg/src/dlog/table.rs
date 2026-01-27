// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::CurveGroup;
use std::collections::HashMap;

/// Build a baby-step table of size `table_size`
///
/// Returns a HashMap: `C.to_compressed() |---> exponent`
#[allow(non_snake_case)]
pub fn build<C: CurveGroup>(G: C, table_size: u64) -> HashMap<Vec<u8>, u64> {
    let byte_size = G.compressed_size();

    let mut table = HashMap::with_capacity(table_size as usize);
    let mut current = C::zero();

    for j in 0..table_size {
        let mut buf = vec![0u8; byte_size];
        current.serialize_compressed(&mut &mut buf[..]).unwrap();
        table.insert(buf, j);
        current += G;
    }

    table
}

#[allow(non_snake_case)]
pub fn build_default<C: CurveGroup>(table_size: u64) -> HashMap<Vec<u8>, u64> {
    let G = C::generator();
    build(G, table_size)
}
