// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod abort_codes {
    pub const E_UNKNOWN_GROUP: u64 = 2;
    pub const E_UNKNOWN_PAIRING: u64 = 3;
    pub const NUM_ELEMENTS_SHOULD_MATCH_NUM_SCALARS: u64 = 4;
}

pub mod blst_backend;
pub mod arkworks_backend;
