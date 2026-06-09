// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// On-chain configuration for the epoch force-end watchdog.
///
/// When `force_end_grace_period_secs` is `Some(n)`, an in-progress reconfiguration
/// is force-finalized once `now >= last_reconfiguration_time + epoch_interval + n_secs`,
/// regardless of DKG state. `None` disables the watchdog.
///
/// The wire shape matches the Move struct:
/// `0x1::epoch_timeout_config::EpochTimeoutConfig { force_end_grace_period_secs: Option<u64> }`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EpochTimeoutConfig {
    pub force_end_grace_period_secs: Option<u64>,
}

impl EpochTimeoutConfig {
    pub fn disabled() -> Self {
        Self {
            force_end_grace_period_secs: None,
        }
    }

    pub fn with_grace_period(secs: u64) -> Self {
        Self {
            force_end_grace_period_secs: Some(secs),
        }
    }

    pub fn default_if_missing() -> Self {
        Self::disabled()
    }
}

impl OnChainConfig for EpochTimeoutConfig {
    const MODULE_IDENTIFIER: &'static str = "epoch_timeout_config";
    const TYPE_IDENTIFIER: &'static str = "EpochTimeoutConfig";
}
