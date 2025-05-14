// Copyright (c) 2025 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::EntryFunction;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, TypeTag, CORE_CODE_ADDRESS};
use move_core_types::value::{serialize_values, MoveValue};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

struct AutomationTransactionEntryRef {
    module_id: ModuleId,
    function: Identifier,
}

static AUTOMATION_REGISTRATION_ENTRY: Lazy<AutomationTransactionEntryRef> =
    Lazy::new(|| AutomationTransactionEntryRef {
        module_id: ModuleId::new(
            CORE_CODE_ADDRESS,
            Identifier::new("automation_registry").unwrap(),
        ),
        function: Identifier::new("register").unwrap(),
    });


/// Represents set of parameters required to register automation task.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationParams {
    V1(RegistrationParamsV1)
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
        RegistrationParams::V1(RegistrationParamsV1::new (
            automated_function,
            expiration_timestamp_secs,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
        ))
    }

    pub fn automated_function(&self) -> &EntryFunction {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.automated_function()
    }

    pub fn expiration_timestamp_secs(&self) -> u64 {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.expiration_timestamp_secs
    }

    pub fn gas_price_cap(&self) -> u64 {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.gas_price_cap
    }

    pub fn max_gas_amount(&self) -> u64 {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.max_gas_amount
    }

    pub fn into_v1(self) -> Option<RegistrationParamsV1> {
        let RegistrationParams::V1(v1_self) = self;
        Some(v1_self)
    }

    /// Module id containing registration function.
    pub fn module_id(&self) -> &ModuleId {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.module_id()
    }

    /// Registration function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.function()
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }

    pub fn serialized_args_with_sender_and_parent_hash(
        &self,
        sender: AccountAddress,
        parent_hash: Vec<u8>,
    ) -> Vec<Vec<u8>> {
        let RegistrationParams::V1(v1_self) = self;
        v1_self.serialized_args_with_sender_and_parent_hash(sender, parent_hash)
    }
}

/// Initial set of parameters required to register automation task.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    aux_data: Vec<Vec<u8>>
}

impl RegistrationParamsV1 {
    pub fn new(
        automated_function: EntryFunction,
        expiration_timestamp_secs: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        aux_data: Vec<Vec<u8>>,
    ) -> RegistrationParamsV1 {
        Self {
            automated_function,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            expiration_timestamp_secs,
            aux_data,
        }
    }

    pub fn automated_function(&self) -> &EntryFunction {
        &self.automated_function
    }

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
        &AUTOMATION_REGISTRATION_ENTRY.module_id
    }

    /// Registration function name accepting enclosed parameters.
    pub fn function(&self) -> &IdentStr {
        &AUTOMATION_REGISTRATION_ENTRY.function
    }

    /// Type arguments required by registration function.
    pub fn ty_args(&self) -> Vec<TypeTag> {
        vec![]
    }

    pub fn serialized_args_with_sender_and_parent_hash(
        &self,
        sender: AccountAddress,
        parent_hash: Vec<u8>,
    ) -> Vec<Vec<u8>> {
        let aux_move_args = self.aux_data.iter().map(|item| MoveValue::vector_u8(item.clone())).collect();
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
}

/// Rust representation of the Automation task meta information in Move.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Not used currently. Reserved for future extentions.
    pub(crate) aux_data: Vec<Vec<u8>>,
    /// Registration epoch timestamp
    pub(crate) registration_time: u64,
    /// Flag indicating whether the task is active.
    pub(crate) is_active: bool,
    /// Fee locked for the task estimated for the next epoch at the start of the current epoch.
    pub(crate) locked_fee_for_next_epoch: u64,
}

impl AutomationTaskMetaData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        is_active: bool,
    ) -> Self {
        Self::new_with_locked_fee(
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
            is_active,
            0,
        )
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
        is_active: bool,
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
            is_active,
            locked_fee_for_next_epoch,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn gas_price_cap(&self) -> u64 {
        self.gas_price_cap
    }

    pub fn payload_tx(&self) -> &[u8] {
        &self.payload_tx
    }

    pub fn expiry_time(&self) -> u64 {
        self.expiry_time
    }

    pub fn tx_hash(&self) -> &[u8] {
        &self.tx_hash
    }

    pub fn max_gas_amount(&self) -> u64 {
        self.max_gas_amount
    }

    pub fn registration_time(&self) -> u64 {
        self.registration_time
    }

    pub fn owner(&self) -> AccountAddress {
        self.owner
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn locked_fee_for_next_epoch(&self) -> u64 {
        self.locked_fee_for_next_epoch
    }
}
