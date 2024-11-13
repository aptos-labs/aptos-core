// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::account::{Account, AccountData};
use proptest::prelude::*;

impl Arbitrary for Account {
    type Parameters = ();
    type Strategy = fn() -> Account;

    fn arbitrary_with(_params: ()) -> Self::Strategy {
        // Provide Account::new as the canonical strategy. This means that no shrinking will happen,
        // but that's fine as accounts have nothing to shrink inside them anyway.
        Account::new as Self::Strategy
    }
}

impl AccountData {
    /// Returns a [`Strategy`] that creates `AccountData` instances.
    pub fn strategy(balance_strategy: impl Strategy<Value = u64>) -> impl Strategy<Value = Self> {
        // Pick sequence numbers and event counts in a smaller range so that valid transactions can
        // be generated.
        // XXX should we also test edge cases around large sequence numbers?
        let sequence_strategy = 0u64..(1 << 32);

        (any::<Account>(), balance_strategy, sequence_strategy).prop_map(
            |(account, balance, sequence_number)| {
                AccountData::with_account_and_fungible_store(
                    account,
                    balance,
                    sequence_number,
                    false,
                )
            },
        )
    }
}
