// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod aggregator_v1_extension;
pub mod bounded_math;
pub mod delayed_change;
pub mod delayed_field_extension;
pub mod delta_change_set;
pub mod delta_math;
pub mod resolver;
pub mod types;

#[cfg(any(test, feature = "testing"))]
pub mod tests;

#[cfg(any(test, feature = "testing"))]
pub use tests::types::{
    aggregator_v1_id_for_test, aggregator_v1_state_key_for_test, FakeAggregatorView,
};
