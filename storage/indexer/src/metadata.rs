// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum Metadata {
    LatestVersion(Version),
}

#[derive(Clone, Debug, Deserialize, FromPrimitive, PartialEq, ToPrimitive, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub(crate) enum MetadataTag {
    LatestVersion = 0,
}
