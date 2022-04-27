// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::constants::CORE_ACCOUNT_MODULE_IDENTIFIER, event::EventHandle};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// A Rust representation of an Account resource.
/// This is not how the Account is represented in the VM but it's a convenient representation.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountResource {
    authentication_key: Vec<u8>,
    sequence_number: u64,
    self_address: AccountAddress,
    balance: Coin,
    transfer_events: TransferEvents,
}

impl AccountResource {
    /// Constructs an Account resource.
    pub fn new(
        sequence_number: u64,
        authentication_key: Vec<u8>,
        self_address: AccountAddress,
        balance: u64,
        sent_events: EventHandle,
        received_events: EventHandle,
    ) -> Self {
        AccountResource {
            authentication_key,
            sequence_number,
            self_address,
            balance: Coin::new(balance),
            transfer_events: TransferEvents::new(sent_events, received_events),
        }
    }

    /// Return the sequence_number field for the given AccountResource
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    /// Return the authentication_key field for the given AccountResource
    pub fn authentication_key(&self) -> &[u8] {
        &self.authentication_key
    }

    pub fn address(&self) -> AccountAddress {
        self.self_address
    }

    pub fn balance(&self) -> u64 {
        self.balance.value()
    }

    pub fn transfer_events(&self) -> &TransferEvents {
        &self.transfer_events
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn set_balance(&mut self, amount: u64) {
        self.balance.value = amount;
    }
}

impl MoveStructType for AccountResource {
    const MODULE_NAME: &'static IdentStr = CORE_ACCOUNT_MODULE_IDENTIFIER;
    const STRUCT_NAME: &'static IdentStr = CORE_ACCOUNT_MODULE_IDENTIFIER;
}

impl MoveResource for AccountResource {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct TransferEvents {
    sent_events: EventHandle,
    received_events: EventHandle,
}

impl TransferEvents {
    pub fn new(sent_events: EventHandle, received_events: EventHandle) -> Self {
        Self {
            sent_events,
            received_events,
        }
    }

    pub fn received_events(&self) -> &EventHandle {
        &self.received_events
    }

    pub fn sent_events(&self) -> &EventHandle {
        &self.sent_events
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct Coin {
    value: u64,
}

impl Coin {
    pub fn new(amount: u64) -> Self {
        Coin { value: amount }
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
