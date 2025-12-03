// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::on_chain_config::ValidatorSet;
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_validator_set_canonical_serialization(set in any::<ValidatorSet>()) {
        assert_canonical_encode_decode(set);
    }
}
