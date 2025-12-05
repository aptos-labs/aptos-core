// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod proof_test;
mod write_test;

use super::*;
use crate::test_helpers::{arb_hash_batch, test_get_frozen_subtree_hashes_impl};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_get_frozen_subtree_hashes(leaves in arb_hash_batch(1000)) {
        test_get_frozen_subtree_hashes_impl(leaves);
    }
}
