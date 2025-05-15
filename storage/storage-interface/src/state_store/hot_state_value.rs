// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValue;
use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum HotStateValue {
    Occupied(StateValue),
    Vacant,
}
