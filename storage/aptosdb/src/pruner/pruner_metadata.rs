// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum PrunerMetadata {
    LatestVersion(Version),
}

#[derive(Clone, Debug, Deserialize, FromPrimitive, PartialEq, Eq, ToPrimitive, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub enum PrunerTag {
    LedgerPruner = 0,
    StateMerklePruner = 1,
    EpochEndingStateMerklePruner = 2,
}
