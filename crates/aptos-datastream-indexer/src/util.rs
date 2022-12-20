// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bigdecimal::FromPrimitive;

pub fn u64_to_bigdecimal(val: u64) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_u64(val).unwrap()
}
