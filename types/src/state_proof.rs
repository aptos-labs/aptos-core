// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// A convenience type for the collection of sub-proofs that consistitute a
/// response to a `get_state_proof` request.
///
/// From a `StateProof` response, a client should be able to ratchet their
/// [`TrustedState`] to the last epoch change LI in the [`EpochChangeProof`]
/// or the latest [`LedgerInfoWithSignatures`] if the epoch changes get them into
/// the most recent epoch.
///
/// [`TrustedState`]: crate::trusted_state::TrustedState
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StateProof {
    latest_li_w_sigs: LedgerInfoWithSignatures,
    epoch_changes: EpochChangeProof,
}

impl StateProof {
    pub fn new(
        latest_li_w_sigs: LedgerInfoWithSignatures,
        epoch_changes: EpochChangeProof,
    ) -> Self {
        Self {
            latest_li_w_sigs,
            epoch_changes,
        }
    }

    pub fn into_inner(self) -> (LedgerInfoWithSignatures, EpochChangeProof) {
        (self.latest_li_w_sigs, self.epoch_changes)
    }

    pub fn as_inner(&self) -> (&LedgerInfoWithSignatures, &EpochChangeProof) {
        (&self.latest_li_w_sigs, &self.epoch_changes)
    }

    #[inline]
    pub fn latest_ledger_info(&self) -> &LedgerInfo {
        self.latest_li_w_sigs.ledger_info()
    }

    #[inline]
    pub fn latest_ledger_info_w_sigs(&self) -> &LedgerInfoWithSignatures {
        &self.latest_li_w_sigs
    }

    #[inline]
    pub fn epoch_changes(&self) -> &EpochChangeProof {
        &self.epoch_changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcs::test_helpers::assert_canonical_encode_decode;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn test_state_proof_canonical_serialization(proof in any::<StateProof>()) {
            assert_canonical_encode_decode(proof);
        }
    }
}
