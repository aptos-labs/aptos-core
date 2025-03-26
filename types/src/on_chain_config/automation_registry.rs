// Copyright (c) 2025 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use move_core_types::account_address::AccountAddress;
use move_core_types::value::{serialize_values, MoveValue};
use serde::{Deserialize, Serialize};

const ONE_MONTH_IN_SECS: u64 = 2_626_560;
const DEFAULT_REGISTRY_MAX_GAS_CAP: u64 = 100_000_000;
const DEFAULT_AUTOMATION_BASE_FEE_IN_QUANTS_PER_SEC: u64 = 1000;
const ONE_SUPRA_IN_QUANTS: u64 = 100_000_000;
const DEFAULT_CONGESTION_THRESHOLD_PERCENTAGE: u8 = 80;
const DEFAULT_CONGESTION_BASE_FEE_IN_QUANTS_PER_SEC: u64 = 100;
const DEFAULT_CONGESTION_EXPONENT: u8 = 6;
const DEFAULT_TASK_CAPACITY: u16 = 500;

/// Initial version of configuration parameters for Supra native automation feature
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq)]
pub struct AutomationRegistryConfigV1 {
    /// Maximum allowable duration (in seconds) from the registration time that an automation task can run.
    /// If the expiration time exceeds this duration, the task registration will fail.
    task_duration_cap_in_secs: u64,
    /// Maximum gas allocation for automation tasks per epoch
    /// Exceeding this limit during task registration will cause failure and is used in fee calculation.
    registry_max_gas_cap: u64,
    /// Base fee per second for the full capacity of the automation registry, measured in quants/sec.
    /// The capacity is considered full if the total committed gas of all registered tasks equals registry_max_gas_cap.
    automation_base_fee_in_quants_per_sec: u64,
    /// Flat registration fee charged by default for each task.
    flat_registration_fee_in_quants: u64,
    /// Ratio (in the range [0;100]) representing the acceptable upper limit of committed gas amount
    /// relative to registry_max_gas_cap. Beyond this threshold, congestion fees apply.
    congestion_threshold_percentage: u8,
    /// Base fee per second for the full capacity of the automation registry when the congestion threshold is exceeded.
    congestion_base_fee_in_quants_per_sec: u64,
    /// The congestion fee increases exponentially based on this value, ensuring higher fees as the registry approaches full capacity.
    congestion_exponent: u8,
    /// Maximum number of tasks that registry can hold.
    task_capacity: u16,
}

impl Default for AutomationRegistryConfigV1 {
    fn default() -> Self {
        Self {
            task_duration_cap_in_secs: ONE_MONTH_IN_SECS,
            registry_max_gas_cap: DEFAULT_REGISTRY_MAX_GAS_CAP,
            automation_base_fee_in_quants_per_sec: DEFAULT_AUTOMATION_BASE_FEE_IN_QUANTS_PER_SEC,
            flat_registration_fee_in_quants: ONE_SUPRA_IN_QUANTS,
            congestion_threshold_percentage: DEFAULT_CONGESTION_THRESHOLD_PERCENTAGE,
            congestion_base_fee_in_quants_per_sec: DEFAULT_CONGESTION_BASE_FEE_IN_QUANTS_PER_SEC,
            congestion_exponent: DEFAULT_CONGESTION_EXPONENT,
            task_capacity: DEFAULT_TASK_CAPACITY,
        }
    }
}

impl AutomationRegistryConfigV1 {
    pub fn task_duration_cap_in_secs(&self) -> u64 {
        self.task_duration_cap_in_secs
    }

    pub fn registry_max_gas_cap(&self) -> u64 {
        self.registry_max_gas_cap
    }

    pub fn automation_base_fee_in_quants_per_sec(&self) -> u64 {
        self.automation_base_fee_in_quants_per_sec
    }
    pub fn flat_registration_fee_in_quants(&self) -> u64 {
        self.flat_registration_fee_in_quants
    }

    pub fn congestion_threshold_percentage(&self) -> u8 {
        self.congestion_threshold_percentage
    }
    pub fn congestion_base_fee_in_quants_per_sec(&self) -> u64 {
        self.congestion_base_fee_in_quants_per_sec
    }

    pub fn congestion_exponent(&self) -> u8 {
        self.congestion_exponent
    }

    pub fn task_capacity(&self) -> u16 {
        self.task_capacity
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq)]
pub enum AutomationRegistryConfig {
    V1(AutomationRegistryConfigV1),
}

impl Default for AutomationRegistryConfig {
    fn default() -> Self {
        Self::V1(AutomationRegistryConfigV1::default())
    }
}

impl AutomationRegistryConfig {
    pub fn serialize_into_move_values_with_signer(
        &self,
        signer_address: AccountAddress,
    ) -> Vec<Vec<u8>> {
        let AutomationRegistryConfig::V1(config) = self;
        let arguments = vec![
            MoveValue::Signer(signer_address),
            MoveValue::U64(config.task_duration_cap_in_secs()),
            MoveValue::U64(config.registry_max_gas_cap()),
            MoveValue::U64(config.automation_base_fee_in_quants_per_sec()),
            MoveValue::U64(config.flat_registration_fee_in_quants()),
            MoveValue::U8(config.congestion_threshold_percentage()),
            MoveValue::U64(config.congestion_base_fee_in_quants_per_sec()),
            MoveValue::U8(config.congestion_exponent()),
            MoveValue::U16(config.task_capacity()),
        ];
        serialize_values(&arguments)
    }
}

impl From<AutomationRegistryConfigV1> for AutomationRegistryConfig {
    fn from(config: AutomationRegistryConfigV1) -> Self {
        Self::V1(config)
    }
}

impl OnChainConfig for AutomationRegistryConfig {
    const MODULE_IDENTIFIER: &'static str = "automation_registry";
    const TYPE_IDENTIFIER: &'static str = "AutomationRegistryConfig";
}
