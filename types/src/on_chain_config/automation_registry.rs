// Copyright (c) 2025 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use derive_getters::Getters;
use derive_more::Constructor;
use move_core_types::account_address::AccountAddress;
use move_core_types::value::{serialize_values, MoveValue};
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};

const ONE_MONTH_IN_SECS: u64 = 2_626_560;
const DEFAULT_REGISTRY_MAX_GAS_CAP: u64 = 100_000_000;
const DEFAULT_AUTOMATION_BASE_FEE_IN_QUANTS_PER_SEC: u64 = 1000;
const ONE_SUPRA_IN_QUANTS: u64 = 100_000_000;
const DEFAULT_CONGESTION_THRESHOLD_PERCENTAGE: u8 = 80;
const DEFAULT_CONGESTION_BASE_FEE_IN_QUANTS_PER_SEC: u64 = 100;
const DEFAULT_CONGESTION_EXPONENT: u8 = 6;
const DEFAULT_TASK_CAPACITY: u16 = 500;
const DEFAULT_CYCLE_DURATION_SECS: u64 = 1200;
const DEFAULT_SYSTEM_TASK_CAPACITY: u16 = 100;
const DEFAULT_SYSTEM_TASKS_MAX_GAS_CAP: u64 = 200_000;

/// Initial version of configuration parameters for Supra native automation feature
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq, Getters, Constructor)]
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

/// Extended version of configuration parameters for Supra native automation feature supporting cycle-duration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq, Getters, Constructor)]
pub struct AutomationRegistryConfigV2 {
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
    /// Cycle duration in seconds.
    cycle_duration_secs: u64,
    /// Maximum allowable duration (in seconds) from the registration time that a system automation task can run.
    /// If the expiration time exceeds this duration, the task registration will fail.
    system_task_duration_cap_in_secs: u64,
    /// Maximum gas allocation for system automation tasks per epoch
    /// Exceeding this limit during task registration will cause failure and is used in fee calculation.
    system_tasks_max_gas_cap: u64,
    /// Maximum number of system tasks that registry can hold.
    system_task_capacity: u16,
}

impl Default for AutomationRegistryConfigV2 {
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
            cycle_duration_secs: DEFAULT_CYCLE_DURATION_SECS,
            system_task_duration_cap_in_secs: ONE_MONTH_IN_SECS * 2,
            system_tasks_max_gas_cap: DEFAULT_SYSTEM_TASKS_MAX_GAS_CAP,
            system_task_capacity: DEFAULT_SYSTEM_TASK_CAPACITY,
        }
    }
}

impl From<AutomationRegistryConfigV1> for AutomationRegistryConfigV2 {
    fn from(v1: AutomationRegistryConfigV1) -> Self {
        let AutomationRegistryConfigV1 {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            automation_base_fee_in_quants_per_sec,
            flat_registration_fee_in_quants,
            congestion_threshold_percentage,
            congestion_base_fee_in_quants_per_sec,
            congestion_exponent,
            task_capacity,
        } = v1;
        Self {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            automation_base_fee_in_quants_per_sec,
            flat_registration_fee_in_quants,
            congestion_threshold_percentage,
            congestion_base_fee_in_quants_per_sec,
            congestion_exponent,
            task_capacity,
            cycle_duration_secs: DEFAULT_CYCLE_DURATION_SECS,
            system_task_duration_cap_in_secs: ONE_MONTH_IN_SECS * 2,
            system_tasks_max_gas_cap: DEFAULT_SYSTEM_TASKS_MAX_GAS_CAP,
            system_task_capacity: DEFAULT_SYSTEM_TASK_CAPACITY,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq)]
pub enum AutomationRegistryConfig {
    V1(AutomationRegistryConfigV1),
    V2(AutomationRegistryConfigV2),
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
        let arguments = match self {
            AutomationRegistryConfig::V1(config) => {
                vec![
                    MoveValue::Signer(signer_address),
                    MoveValue::U64(*config.task_duration_cap_in_secs()),
                    MoveValue::U64(*config.registry_max_gas_cap()),
                    MoveValue::U64(*config.automation_base_fee_in_quants_per_sec()),
                    MoveValue::U64(*config.flat_registration_fee_in_quants()),
                    MoveValue::U8(*config.congestion_threshold_percentage()),
                    MoveValue::U64(*config.congestion_base_fee_in_quants_per_sec()),
                    MoveValue::U8(*config.congestion_exponent()),
                    MoveValue::U16(*config.task_capacity()),
                    MoveValue::U64(DEFAULT_CYCLE_DURATION_SECS),
                    MoveValue::U64(ONE_MONTH_IN_SECS * 2),
                    MoveValue::U64(DEFAULT_SYSTEM_TASKS_MAX_GAS_CAP),
                    MoveValue::U16(DEFAULT_SYSTEM_TASK_CAPACITY),
                ]
            },
            AutomationRegistryConfig::V2(config) => {
                vec![
                    MoveValue::Signer(signer_address),
                    MoveValue::U64(*config.task_duration_cap_in_secs()),
                    MoveValue::U64(*config.registry_max_gas_cap()),
                    MoveValue::U64(*config.automation_base_fee_in_quants_per_sec()),
                    MoveValue::U64(*config.flat_registration_fee_in_quants()),
                    MoveValue::U8(*config.congestion_threshold_percentage()),
                    MoveValue::U64(*config.congestion_base_fee_in_quants_per_sec()),
                    MoveValue::U8(*config.congestion_exponent()),
                    MoveValue::U16(*config.task_capacity()),
                    MoveValue::U64(*config.cycle_duration_secs()),
                    MoveValue::U64(*config.system_task_duration_cap_in_secs()),
                    MoveValue::U64(*config.system_tasks_max_gas_cap()),
                    MoveValue::U16(*config.system_task_capacity()),
                ]
            },
        };
        serialize_values(&arguments)
    }
}

impl From<AutomationRegistryConfigV1> for AutomationRegistryConfig {
    fn from(config: AutomationRegistryConfigV1) -> Self {
        Self::V1(config)
    }
}

impl From<AutomationRegistryConfigV2> for AutomationRegistryConfig {
    fn from(config: AutomationRegistryConfigV2) -> Self {
        Self::V2(config)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum AutomationCycleState {
    #[default]
    READY = 0,
    STARTED,
    FINISHED,
    SUSPENDED,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AutomationCycleInfo {
    /// Current cycle id. Incremented when a start of a new cycle is given.
    pub index: u64,
    /// State of the current cycle.
    pub state: AutomationCycleState,
    /// Current cycle start time which is updated with the current chain time when a cycle is incremented.
    pub start_time: u64,
    /// Automation cycle duration in seconds.
    pub duration_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationCycleEvent {
    /// Updated cycle state information.
    pub cycle_state_info: AutomationCycleInfo,
    /// The state transitioned from
    pub old_state: AutomationCycleState,
}

impl MoveStructType for AutomationCycleEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("automation_registry");
    const STRUCT_NAME: &'static IdentStr = ident_str!("AutomationCycleEvent");
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationCycleTransitionState {
    /// Refund duration of automation fees when automation feature/cycle is suspended.
    pub refund_duration: u64,
    /// Duration of the new cycle to charge fees for.
    pub new_cycle_duration: u64,
    /// Calculated automation fee per second for a new cycle or for refund period.
    pub automation_fee_per_sec: u64,
    /// Gas committed for the new cycle being transitioned.
    pub gas_committed_for_new_cycle: u64,
    /// Gas committed for the next cycle by user submitted tasks.
    pub gas_committed_for_next_cycle: u64,
    /// Gas committed for the next cycle by system tasks.
    pub system_gas_committed_for_next_cycle: u64,
    /// Total fee charged from users for the new cycle, which is not withdrawable.
    pub locked_fees: u64,
    /// List of the tasks still to be processed during transition.
    /// This list should be sorted in ascending order.
    /// The requirement is that all tasks are processed in the order of their registration. Which should be true
    /// especially for cycle fee charges before new cycle start.
    pub expected_tasks_to_be_processed: Vec<u64>,
    /// Position of the task index in the expected_tasks_to_be_processed to be processed next.
    /// It is incremented when an expected task is successfully processed.
    pub next_task_index_position: u64,
}

/// On-chain Automation Cycle Details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationCycleDetails {
    /// Cycle index corresponding to the current state. Incremented when a transition to the new cycle is finalized.
    pub index: u64,
    /// State of the current cycle.
    pub state: AutomationCycleState,
    /// Current cycle start time which is updated with the current chain time when a cycle is incremented.
    pub start_time: u64,
    /// Automation cycle duration in seconds for the current cycle.
    pub duration_secs: u64,
    /// Intermediate state of cycle transition to next one or suspended state.
    pub transition_state: Option<AutomationCycleTransitionState>,
}

impl From<AutomationCycleDetails> for AutomationCycleInfo {
    fn from(details: AutomationCycleDetails) -> Self {
        Self {
            index: details.index,
            state: details.state,
            start_time: details.start_time,
            duration_secs: details.duration_secs,
        }
    }
}

impl OnChainConfig for AutomationCycleDetails {
    const MODULE_IDENTIFIER: &'static str = "automation_registry";
    const TYPE_IDENTIFIER: &'static str = "AutomationCycleDetails";
}
