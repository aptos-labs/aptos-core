// Copyright (c) 2025 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::{FeatureFlag, Features};
use crate::transaction::{EntryFunction, Transaction};
use aptos_crypto::HashValue;
use derive_getters::Getters;
use derive_more::Constructor;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, TypeTag, CORE_CODE_ADDRESS};
use move_core_types::value::{serialize_values, MoveValue};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

struct AutomationTransactionEntryRef {
    module_id: ModuleId,
    register_user_task_function: Identifier,
    register_system_task_function: Identifier,
    process_tasks_function: Identifier,
}

static AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS: Lazy<AutomationTransactionEntryRef> =
    Lazy::new(|| AutomationTransactionEntryRef {
        module_id: ModuleId::new(
            CORE_CODE_ADDRESS,
            Identifier::new("automation_registry").unwrap(),
        ),
        register_user_task_function: Identifier::new("register").unwrap(),
        register_system_task_function: Identifier::new("register_system_task").unwrap(),
        process_tasks_function: Identifier::new("process_tasks").unwrap(),
    });

/// Represents set of parameters required to register automation task.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationParams {
    V1(RegistrationParamsV1),
    V2(RegistrationParamsV2),
}

impl RegistrationParams {
    pub fn new_v1(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
    ) -> RegistrationParams {
        RegistrationParams::V1(RegistrationParamsV1::new(
            automated_function,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            expiration_timestamp_secs,
            aux_data,
        ))
    }

    pub fn new_v2(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
        task_type: AutomationTaskType,
        priority_value: Option<u64>,
    ) -> RegistrationParams {
        RegistrationParams::V2(RegistrationParamsV2::new(
            automated_function,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            expiration_timestamp_secs,
            aux_data,
            task_type,
            priority_value,
        ))
    }

    pub fn new_user_automation_task_v1(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
    ) -> RegistrationParams {
        Self::new_v1(
            automated_function,
            expiration_timestamp_secs,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
        )
    }

    pub fn new_user_automation_task_v2(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
        priority_value: Option<u64>,
    ) -> RegistrationParams {
        Self::new_v2(
            automated_function,
            expiration_timestamp_secs,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
            AutomationTaskType::User,
            priority_value,
        )
    }

    pub fn new_system_automation_task(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        aux_data: Vec<Vec<u8>>,
        priority_value: Option<u64>,
    ) -> RegistrationParams {
        Self::new_v2(
            automated_function,
            expiration_timestamp_secs,
            max_gas_amount,
            0,
            0,
            aux_data,
            AutomationTaskType::System,
            priority_value,
        )
    }

    pub fn automated_function(&self) -> &EntryFunction {
        match self {
            RegistrationParams::V1(p) => p.automated_function(),
            RegistrationParams::V2(p) => p.automated_function(),
        }
    }

    pub fn expiration_timestamp_secs(&self) -> u64 {
        match self {
            RegistrationParams::V1(p) => p.expiration_timestamp_secs,
            RegistrationParams::V2(p) => p.expiration_timestamp_secs,
        }
    }

    pub fn gas_price_cap(&self) -> u64 {
        match self {
            RegistrationParams::V1(p) => p.gas_price_cap,
            RegistrationParams::V2(p) => p.gas_price_cap,
        }
    }

    pub fn max_gas_amount(&self) -> u64 {
        match self {
            RegistrationParams::V1(p) => p.max_gas_amount,
            RegistrationParams::V2(p) => p.max_gas_amount,
        }
    }

    pub fn into_v1(self) -> Option<RegistrationParamsV1> {
        match self {
            RegistrationParams::V1(p) => Some(p),
            RegistrationParams::V2(_) => None,
        }
    }

    pub fn into_v2(self) -> Option<RegistrationParamsV2> {
        match self {
            RegistrationParams::V1(_) => None,
            RegistrationParams::V2(p) => Some(p),
        }
    }

    /// Module id containing registration function.
    pub fn module_id(&self) -> &ModuleId {
        match self {
            RegistrationParams::V1(p) => p.module_id(),
            RegistrationParams::V2(p) => p.module_id(),
        }
    }

    /// Registration function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        match self {
            RegistrationParams::V1(p) => p.function(),
            RegistrationParams::V2(p) => p.function(),
        }
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }

    pub fn serialized_args_with_sender_and_parent_hash(
        &self,
        sender: AccountAddress,
        parent_hash: Vec<u8>,
        features: &Features,
    ) -> Vec<Vec<u8>> {
        match self {
            RegistrationParams::V1(p) => {
                p.serialized_args_with_sender_and_parent_hash(sender, parent_hash, features)
            },
            RegistrationParams::V2(p) => {
                p.serialized_args_with_sender_and_parent_hash(sender, parent_hash, features)
            },
        }
    }
    pub fn task_type(&self) -> AutomationTaskType {
        match self {
            RegistrationParams::V1(p) => p.task_type(),
            RegistrationParams::V2(p) => *p.task_type(),
        }
    }
}

/// Initial set of parameters required to register automation task.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Constructor, Getters)]
pub struct RegistrationParamsV1 {
    /// Entry function to be automated.
    automated_function: EntryFunction,
    /// Max gas amount for automated transaction.
    max_gas_amount: u64,
    /// Gas Uint price upper limit that user is willing to pay.
    gas_price_cap: u64,
    /// Maximum automation fee that user is willing to pay for epoch.
    automation_fee_cap_for_epoch: u64,
    /// Expiration time of the automated transaction in seconds since UTC Epoch start.
    expiration_timestamp_secs: u64,
    /// Reserved for future extensions of registration parameters.
    /// Will be helpful if the new registration parameters will affect only registration but not
    /// task execution layer in native layer.
    /// If a newly added parameter affects automation-task execution flow, that means
    /// the entire flow of the automation task execution is going to be affected in native layer,
    /// which will require all components upgrade( not only supra-framework/state but also node)
    /// then it is advised to add a new version of registration parameters and have the new parameter properly
    /// integrated in the automation-task/automated-transaction execution flow.
    aux_data: Vec<Vec<u8>>,
}

impl RegistrationParamsV1 {
    pub fn into_inner(self) -> (EntryFunction, u64, u64, u64, u64, Vec<Vec<u8>>) {
        (
            self.automated_function,
            self.max_gas_amount,
            self.gas_price_cap,
            self.expiration_timestamp_secs,
            self.automation_fee_cap_for_epoch,
            self.aux_data,
        )
    }
    /// Module id containing registration function.
    pub fn module_id(&self) -> &ModuleId {
        &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.module_id
    }

    /// Registration function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.register_user_task_function
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }

    /// Returns [`AutomationTaskType::User`] as task type .
    pub fn task_type(&self) -> AutomationTaskType {
        AutomationTaskType::User
    }

    pub fn serialized_args_with_sender_and_parent_hash(
        &self,
        sender: AccountAddress,
        parent_hash: Vec<u8>,
        features: &Features,
    ) -> Vec<Vec<u8>> {
        let aux_move_args = self.prepare_aux_data(features);
        serialize_values(&[
            MoveValue::Address(sender),
            MoveValue::vector_u8(bcs::to_bytes(&self.automated_function).unwrap()),
            MoveValue::U64(self.expiration_timestamp_secs),
            MoveValue::U64(self.max_gas_amount),
            MoveValue::U64(self.gas_price_cap),
            MoveValue::U64(self.automation_fee_cap_for_epoch),
            MoveValue::vector_u8(parent_hash),
            MoveValue::Vector(aux_move_args),
        ])
    }

    fn prepare_aux_data(&self, features: &Features) -> Vec<MoveValue> {
        let mut aux_data = vec![];
        if features.is_enabled(FeatureFlag::SUPRA_AUTOMATION_V2) {
            let type_value = vec![self.task_type() as u8];
            // With V1 version no priority is supported and will always be assigned by
            // registry with default value
            let priority_value = vec![];
            aux_data = vec![type_value, priority_value];
        }
        aux_data
            .iter()
            .chain(self.aux_data.iter())
            .map(|item| MoveValue::vector_u8(item.clone()))
            .collect()
    }
}

/// Initial set of parameters required to register automation task.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Constructor, Getters)]
pub struct RegistrationParamsV2 {
    /// Entry function to be automated.
    automated_function: EntryFunction,
    /// Max gas amount for automated transaction.
    max_gas_amount: u64,
    /// Gas Uint price upper limit that user is willing to pay.
    gas_price_cap: u64,
    /// Maximum automation fee that user is willing to pay for epoch.
    automation_fee_cap_for_epoch: u64,
    /// Expiration time of the automated transaction in seconds since UTC Epoch start.
    expiration_timestamp_secs: u64,
    /// Reserved for future extensions of registration parameters.
    /// Will be helpful if the new registration parameters will affect only registration but not
    /// task execution layer in native layer.
    /// If a newly added parameter affects automation-task execution flow, that means
    /// the entire flow of the automation task execution is going to be affected in native layer,
    /// which will require all components upgrade( not only supra-framework/state but also node)
    /// then it is advised to add a new version of registration parameters and have the new parameter properly
    /// integrated in the automation-task/automated-transaction execution flow.
    aux_data: Vec<Vec<u8>>,
    /// The type of the task being registered.
    task_type: AutomationTaskType,
    /// The priority of the task assigned at registration time.
    /// None priority means that it will be assigned by registry at registration time.
    priority_value: Option<u64>,
}

impl RegistrationParamsV2 {
    pub fn into_inner(
        self,
    ) -> (
        EntryFunction,
        u64,
        u64,
        u64,
        u64,
        Vec<Vec<u8>>,
        AutomationTaskType,
        Option<u64>,
    ) {
        (
            self.automated_function,
            self.max_gas_amount,
            self.gas_price_cap,
            self.expiration_timestamp_secs,
            self.automation_fee_cap_for_epoch,
            self.aux_data,
            self.task_type,
            self.priority_value,
        )
    }
    /// Module id containing registration function.
    pub fn module_id(&self) -> &ModuleId {
        &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.module_id
    }

    /// Registration function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        match self.task_type() {
            AutomationTaskType::System => {
                &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.register_system_task_function
            },
            AutomationTaskType::User => {
                &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.register_user_task_function
            },
        }
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }

    pub fn serialized_args_with_sender_and_parent_hash(
        &self,
        sender: AccountAddress,
        parent_hash: Vec<u8>,
        features: &Features,
    ) -> Vec<Vec<u8>> {
        let aux_move_args = self.prepare_aux_data(features);
        match self.task_type() {
            AutomationTaskType::System => serialize_values(&[
                MoveValue::Address(sender),
                MoveValue::vector_u8(bcs::to_bytes(&self.automated_function).unwrap()),
                MoveValue::U64(self.expiration_timestamp_secs),
                MoveValue::U64(self.max_gas_amount),
                MoveValue::vector_u8(parent_hash),
                MoveValue::Vector(aux_move_args),
            ]),
            AutomationTaskType::User => serialize_values(&[
                MoveValue::Address(sender),
                MoveValue::vector_u8(bcs::to_bytes(&self.automated_function).unwrap()),
                MoveValue::U64(self.expiration_timestamp_secs),
                MoveValue::U64(self.max_gas_amount),
                MoveValue::U64(self.gas_price_cap),
                MoveValue::U64(self.automation_fee_cap_for_epoch),
                MoveValue::vector_u8(parent_hash),
                MoveValue::Vector(aux_move_args),
            ]),
        }
    }

    fn prepare_aux_data(&self, features: &Features) -> Vec<MoveValue> {
        let mut aux_data = vec![];
        // If SUPRA_AUTOMATION_V2 feature is not enabled then no type and priority should be prepended to aux-data
        if features.is_enabled(FeatureFlag::SUPRA_AUTOMATION_V2) {
            let type_value = vec![*self.task_type() as u8];
            // If no priority is specified by user, it will be assigned by registry at registration time.
            let priority_value = self
                .priority_value()
                .as_ref()
                .map(|v| bcs::to_bytes(v).expect("u64 value should always serialize"))
                .unwrap_or_default();
            aux_data = vec![type_value, priority_value]
        }
        aux_data
            .iter()
            .chain(self.aux_data.iter())
            .map(|item| MoveValue::vector_u8(item.clone()))
            .collect()
    }
}

impl From<RegistrationParamsV1> for RegistrationParamsV2 {
    fn from(value: RegistrationParamsV1) -> Self {
        let RegistrationParamsV1 {
            automated_function,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            expiration_timestamp_secs,
            aux_data,
        } = value;
        Self {
            automated_function,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            expiration_timestamp_secs,
            aux_data,
            // Only user automation tasks are supported via V1 parameters
            task_type: AutomationTaskType::User,
            priority_value: None,
        }
    }
}

/// Type of the automation task.
// The order of the entries is important, a new one should be appended at the end.
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AutomationTaskType {
    // User submitted automation task
    User = 1,
    // System authorized automation task
    System = 2,
}

impl PartialOrd<Self> for AutomationTaskType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AutomationTaskType {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

/// Automation task priority
pub type Priority = u64;

impl From<AutomationTaskType> for Vec<u8> {
    fn from(value: AutomationTaskType) -> Self {
        bcs::to_bytes(&value).unwrap()
    }
}

impl TryFrom<&[u8]> for AutomationTaskType {
    type Error = String;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() == 1 {
            match value[0] {
                1 => Ok(AutomationTaskType::User),
                2 => Ok(AutomationTaskType::System),
                e => Err(format!("Invalid AutomationTaskType discriminant: {e}",)),
            }
        } else {
            Err(format!(
                "Invalid automation task type with discriminant as vector: {:?}",
                value
            ))
        }
    }
}


/// Describes the state of the automation task
// The order of the entries is important, a new one should be appended at the end.
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AutomationTaskState {
    Pending = 0,
    Active,
    Cancelled,
}

impl AutomationTaskState {
    /// Checks whether the task is active of execution.
    /// The asks in cancelled state are considered active as well till the end of the automation cycle.
    pub fn is_active(&self) -> bool {
        match self {
            AutomationTaskState::Pending => false,
            AutomationTaskState::Active |
            AutomationTaskState::Cancelled => true
        }
    }
}


/// Rust representation of the Automation task meta information in Move.
#[derive(Clone, Debug, Serialize, Deserialize, Getters, Constructor)]
pub struct AutomationTaskMetaData {
    /// Automation task index in registry
    pub(crate) id: u64,
    /// The address of the task owner.
    pub(crate) owner: AccountAddress,
    /// The function signature associated with the registry entry.
    pub(crate) payload_tx: Vec<u8>,
    /// Expiry of the task, represented in a timestamp in second.
    pub(crate) expiry_time: u64,
    /// The transaction hash of the request transaction.
    pub(crate) tx_hash: Vec<u8>,
    /// Max gas amount of automation task
    pub(crate) max_gas_amount: u64,
    /// Maximum gas price for the task to be paid ever.
    pub(crate) gas_price_cap: u64,
    /// Maximum automation fee for epoch to be paid ever.
    pub(crate) automation_fee_cap_for_epoch: u64,
    /// Auxiliary data specified for the task to aid registration.
    /// Not used currently. Reserved for future extensions.
    pub(crate) aux_data: Vec<Vec<u8>>,
    /// Registration epoch timestamp
    pub(crate) registration_time: u64,
    /// State of the task
    pub(crate) state: AutomationTaskState,
    /// Fee locked for the task estimated for the next epoch at the start of the current epoch.
    pub(crate) locked_fee_for_next_epoch: u64,
    #[serde(skip)]
    /// Extracted task type.
    pub(crate) task_type: OnceCell<Option<AutomationTaskType>>,
    #[serde(skip)]
    /// Priority of the task to be executed.
    pub(crate) priority: OnceCell<Option<Priority>>,
}
impl AutomationTaskMetaData {
    const TASK_TYPE_AUX_INDEX: usize = 0;
    const TASK_PRIORITY_AUX_INDEX: usize = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: u64,
        owner: AccountAddress,
        payload_tx: Vec<u8>,
        expiry_time: u64,
        tx_hash: Vec<u8>,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
        registration_time: u64,
        state: AutomationTaskState,
    ) -> Self {
        Self {
            id,
            owner,
            payload_tx,
            expiry_time,
            tx_hash,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
            registration_time,
            state,
            locked_fee_for_next_epoch: 0,
            task_type: Default::default(),
            priority: Default::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_locked_fee(
        id: u64,
        owner: AccountAddress,
        payload_tx: Vec<u8>,
        expiry_time: u64,
        tx_hash: Vec<u8>,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
        registration_time: u64,
        state: AutomationTaskState,
        locked_fee_for_next_epoch: u64,
    ) -> Self {
        Self {
            id,
            owner,
            payload_tx,
            expiry_time,
            tx_hash,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
            registration_time,
            state,
            locked_fee_for_next_epoch,
            task_type: Default::default(),
            priority: Default::default(),
        }
    }

    pub fn get_task_type(&self) -> &Option<AutomationTaskType> {
        self.task_type.get_or_init(|| {
            if self.aux_data.is_empty() {
                // For the old tasks registered in scope of the automation v1 feature which are not
                // yet migrated to V2 version
                Some(AutomationTaskType::User)
            } else {
                match AutomationTaskType::try_from(
                    self.aux_data[Self::TASK_TYPE_AUX_INDEX].as_slice(),
                ) {
                    Ok(t) => Some(t),
                    Err(_) => None,
                }
            }
        })
    }

    pub fn get_task_priority(&self) -> &Option<Priority> {
        self.priority.get_or_init(|| {
            if self.aux_data.len() <= Self::TASK_PRIORITY_AUX_INDEX {
                // For tasks which miss priority value use the task index, this is required for Backward Compatibility
                // until Automation V2 is fully enabled/release.
                Some(self.id)
            } else {
                match bcs::from_bytes::<u64>(&self.aux_data[Self::TASK_PRIORITY_AUX_INDEX]) {
                    Ok(value) => Some(value),
                    // If deserialization fails then none is considered specified
                    Err(_) => None,
                }
            }
        })
    }

    /// Consumes the input and returns properties flattened.
    /// No property is modified.
    /// To get exact values of the task type and priority corresponding special getter functions should be used.
    pub fn flatten(self) ->
        (u64,
         AccountAddress,
         Vec<u8>,
         u64,
         Vec<u8>,
         u64,
         u64,
         u64,
         Vec<Vec<u8>>,
         u64,
         AutomationTaskState,
         u64,
         ) {
            (
                self.id,
                self.owner,
                self.payload_tx,
                self.expiry_time,
                self.tx_hash,
                self.max_gas_amount,
                self.gas_price_cap,
                self.automation_fee_cap_for_epoch,
                self.aux_data,
                self.registration_time,
                self.state,
                self.locked_fee_for_next_epoch,
                )
        }
}

/// Action to be performed on automation registry.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum AutomationRegistryAction {
    Process { task_indexes: Vec<u64> },
}

impl AutomationRegistryAction {
    pub fn process(task_indexes: Vec<u64>) -> Self {
        AutomationRegistryAction::Process { task_indexes }
    }

    pub fn process_task(task_index: u64) -> Self {
        AutomationRegistryAction::Process {
            task_indexes: vec![task_index],
        }
    }

    pub fn as_move_value(&self) -> MoveValue {
        let AutomationRegistryAction::Process { task_indexes } = self;
        let value_indexes = task_indexes
            .iter()
            .map(|v| MoveValue::U64(*v))
            .collect::<Vec<_>>();
        MoveValue::Vector(value_indexes)
    }

    /// Returns a tuple of min and max task indexes included in the action.
    pub fn task_range(&self) -> (u64, u64) {
        let AutomationRegistryAction::Process { task_indexes } = self;
        (
            task_indexes.iter().min().copied().unwrap_or(u64::MAX),
            task_indexes.iter().max().copied().unwrap_or(u64::MAX),
        )
    }

    /// Module id containing automation registry target function.
    pub fn module_id(&self) -> &ModuleId {
        &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.module_id
    }

    /// Action function name accepting enclosed tasks.
    pub fn function(&self) -> &IdentStr {
        &AUTOMATION_REGISTRY_PRIVATE_ENTRY_REFS.process_tasks_function
    }

    /// Type arguments required by action function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }
}

/// Automation Registry transaction payload to be executed on cycle transition.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AutomationRegistryRecord {
    /// Index of the record. Should be unique in set of the records scheduled in scope of the same block.
    index: u64,
    /// Index of the new cycle to be moved to.
    cycle_id: u64,
    /// Height of the block in scope of which registry action is requested/scheduled.
    block_height: u64,
    /// Action to perform in scope of the request.
    action: AutomationRegistryAction,
}

impl PartialOrd<Self> for AutomationRegistryRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let this_range = self.action.task_range();
        let other_range = other.action.task_range();
        this_range.partial_cmp(&other_range)
    }
}

impl Ord for AutomationRegistryRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        let this_range = self.action.task_range();
        let other_range = other.action.task_range();
        this_range.cmp(&other_range)
    }
}

impl AutomationRegistryRecord {
    pub fn new(
        record_index: u64,
        cycle_id: u64,
        block_height: u64,
        action: AutomationRegistryAction,
    ) -> AutomationRegistryRecord {
        Self {
            index: record_index,
            cycle_id,
            block_height,
            action,
        }
    }

    pub fn serialize_args_with_sender(&self, sender: AccountAddress) -> Vec<Vec<u8>> {
        let action_as_value = self.action.as_move_value();
        serialize_values(&[
            MoveValue::Address(sender),
            MoveValue::U64(self.cycle_id),
            action_as_value,
        ])
    }

    pub fn hash(&self) -> HashValue {
        HashValue::keccak_256_of(
            &bcs::to_bytes(self).expect("AutomationRegistryRecord serialization should never fail"),
        )
    }

    /// Module id containing automation registry target function.
    pub fn module_id(&self) -> &ModuleId {
        self.action.module_id()
    }

    /// Action  function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        self.action.function()
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        self.action.ty_args()
    }

    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn cycle_id(&self) -> u64 {
        self.cycle_id
    }

    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    pub fn action(&self) -> &AutomationRegistryAction {
        &self.action
    }
}

impl From<AutomationRegistryRecord> for Transaction {
    fn from(value: AutomationRegistryRecord) -> Self {
        Transaction::AutomationRegistryTransaction(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutomationRegistryRecordBuilder {
    record_index: Option<u64>,
    action: Option<AutomationRegistryAction>,
    cycle_id: Option<u64>,
    block_height: Option<u64>,
}

impl AutomationRegistryRecordBuilder {
    pub fn new(cycle_id: u64) -> Self {
        Self {
            action: None,
            cycle_id: Some(cycle_id),
            record_index: None,
            block_height: None,
        }
    }

    pub fn task_range(&self) -> (u64, u64) {
        if self.action.is_none() {
            return (u64::MAX, u64::MAX);
        }
        self.action.as_ref().unwrap().task_range()
    }

    pub fn with_record_index(mut self, record_index: u64) -> Self {
        self.record_index = Some(record_index);
        self
    }

    pub fn with_action(mut self, action: AutomationRegistryAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_cycle_id(mut self, cycle_id: u64) -> Self {
        self.cycle_id = Some(cycle_id);
        self
    }

    pub fn with_block_height(mut self, block_height: u64) -> Self {
        self.block_height = Some(block_height);
        self
    }

    /// Splits the existing record builder into single task based actions if possible.
    /// If no action is specified the same instance is returned.
    pub fn split(mut self) -> Vec<Self> {
        match self.action.take() {
            None => vec![self],
            Some(AutomationRegistryAction::Process { task_indexes }) => task_indexes
                .into_iter()
                .map(AutomationRegistryAction::process_task)
                .map(|action| Self {
                    record_index: None,
                    action: Some(action),
                    cycle_id: self.cycle_id,
                    block_height: self.block_height,
                })
                .collect::<Vec<_>>(),
        }
    }

    /// Consumes the builder and returns task indexes enclosed in the action if any specified
    pub fn into_task_indexes(self) -> Vec<u64> {
        let Some(action_data) = self.action else {
            return vec![];
        };
        let AutomationRegistryAction::Process { task_indexes } = action_data;
        task_indexes
    }

    /// Returns potential number of the tasks to be processed in scope of the record.
    pub fn task_count(&self) -> usize {
        let Some(action_data) = &self.action else {
            return 0;
        };
        let AutomationRegistryAction::Process { task_indexes } = action_data;
        task_indexes.len()
    }

    /// Constructs [`AutomationRegistryRecord`]
    /// Fails if any of the properties is not specified
    pub fn build(self) -> Result<AutomationRegistryRecord, String> {
        let Some(action) = self.action else {
            return Err("AutomationRegistryRecord must have an action".to_string());
        };
        let Some(cycle_id) = self.cycle_id else {
            return Err("AutomationRegistryRecord must have a cycle id".to_string());
        };
        let Some(block_height) = self.block_height else {
            return Err("AutomationRegistryRecord must have a block height".to_string());
        };
        let Some(record_index) = self.record_index else {
            return Err("AutomationRegistryRecord must have an index ".to_string());
        };
        Ok(AutomationRegistryRecord::new(
            record_index,
            cycle_id,
            block_height,
            action,
        ))
    }
}
