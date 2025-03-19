// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::{GasScheduleV2, OnChainConfig},
    state_store::StateView,
};

/// Returns the gas feature version stored in [GasScheduleV2]. If the gas schedule does not exist,
/// returns 0 gas feature version.
pub fn get_gas_feature_version(state_view: &impl StateView) -> u64 {
    GasScheduleV2::fetch_config(state_view)
        .map(|gas_schedule| gas_schedule.feature_version)
        .unwrap_or(0)
}
