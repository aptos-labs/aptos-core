// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use velor_types::ledger_info::LedgerInfoWithSignatures;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        epoch in any::<u64>(),
        ledger_info_with_sigs in any::<LedgerInfoWithSignatures>()
    ) {
        assert_encode_decode::<LedgerInfoSchema>(&epoch, &ledger_info_with_sigs);
    }
}

test_no_panic_decoding!(LedgerInfoSchema);
