// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable};
use anyhow::{anyhow, Error};
use aptos_protos::transaction::v1::{
    multisig_transaction_payload, transaction::TxnData, transaction_payload, EntryFunctionId,
    EntryFunctionPayload, Transaction, TransactionPayload,
};
use serde::{Deserialize, Serialize};

/// We use this for UserTransactions.
/// We support UserPayload and MultisigPayload
///
/// Example:
/// ```
/// use aptos_transaction_filter::UserTransactionFilterBuilder;
///
/// let address = "0x806b27f3d7824a1d78c4291b6d0371aa693437f9eb3393c6440519c0ffaa627f";
/// let filter = UserTransactionFilterBuilder::default().sender(address).build().unwrap();
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(strip_option), default)]
pub struct UserTransactionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(into, strip_option), default)]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<UserTransactionPayloadFilter>,
}

impl Filterable<Transaction> for UserTransactionFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.sender.is_none() && self.payload.is_none() {
            return Err(Error::msg("At least one of sender or payload must be set").into());
        };
        self.payload.is_valid()?;
        Ok(())
    }

    #[inline]
    fn is_allowed(&self, txn: &Transaction) -> bool {
        let user_request = if let Some(TxnData::User(u)) = txn.txn_data.as_ref() {
            if let Some(user_request) = u.request.as_ref() {
                user_request
            } else {
                return false;
            }
        } else {
            return false;
        };

        if let Some(sender_filter) = &self.sender {
            if &user_request.sender != sender_filter {
                return false;
            }
        }

        if let Some(payload_filter) = &self.payload {
            // Get the entry_function_payload from both UserPayload and MultisigPayload
            let entry_function_payload = user_request
                .payload
                .as_ref()
                .and_then(get_entry_function_payload_from_transaction_payload);
            if let Some(payload) = entry_function_payload {
                // Here we have an actual EntryFunctionPayload
                if !payload_filter.is_allowed(payload) {
                    return false;
                }
            }
        }

        true
    }
}

/// Example:
/// ```
/// use aptos_transaction_filter::EntryFunctionFilterBuilder;
///
/// let filter = EntryFunctionFilterBuilder::default()
///   .address("0x0000000000000000000000000000000000000000000000000000000000000001")
///   .module("coin")
///   .function("transfer")
///   .build()
///   .unwrap();
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(into, strip_option), default)]
pub struct EntryFunctionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

impl Filterable<EntryFunctionId> for EntryFunctionFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.address.is_none() && self.module.is_none() && self.function.is_none() {
            return Err(anyhow!("At least one of address, name or function must be set").into());
        };
        Ok(())
    }

    #[inline]
    fn is_allowed(&self, module_id: &EntryFunctionId) -> bool {
        if !self.function.is_allowed(&module_id.name) {
            return false;
        }

        if self.address.is_some() || self.function.is_some() {
            if let Some(module) = &module_id.module.as_ref() {
                if !(self.address.is_allowed(&module.address)
                    && self.module.is_allowed(&module.name))
                {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Example:
/// ```
/// use aptos_transaction_filter::{EntryFunctionFilterBuilder, UserTransactionPayloadFilterBuilder};
///
/// let entry_function_filter = EntryFunctionFilterBuilder::default()
///   .address("0x0000000000000000000000000000000000000000000000000000000000000001")
///   .module("coin")
///   .function("transfer")
///   .build()
///   .unwrap();
/// let filter = UserTransactionPayloadFilterBuilder::default()
///   .function(entry_function_filter)
///   .build()
///   .unwrap();
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(strip_option), default)]
pub struct UserTransactionPayloadFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<EntryFunctionFilter>,
}

impl Filterable<EntryFunctionPayload> for UserTransactionPayloadFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.function.is_none() {
            return Err(Error::msg("At least function must be set").into());
        };
        self.function.is_valid()?;
        Ok(())
    }

    #[inline]
    fn is_allowed(&self, payload: &EntryFunctionPayload) -> bool {
        self.function.is_allowed_opt(&payload.function)
    }
}

/// Get the entry_function_payload from both UserPayload and MultisigPayload
fn get_entry_function_payload_from_transaction_payload(
    payload: &TransactionPayload,
) -> Option<&EntryFunctionPayload> {
    let z = if let Some(payload) = &payload.payload {
        match payload {
            transaction_payload::Payload::EntryFunctionPayload(ef_payload) => Some(ef_payload),
            transaction_payload::Payload::MultisigPayload(ms_payload) => ms_payload
                .transaction_payload
                .as_ref()
                .and_then(|tp| tp.payload.as_ref())
                .map(|payload| match payload {
                    multisig_transaction_payload::Payload::EntryFunctionPayload(ef_payload) => {
                        ef_payload
                    },
                }),
            _ => None,
        }
    } else {
        None
    };
    z
}
