// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable, utils::standardize_address};
use anyhow::{anyhow, Error};
use aptos_protos::transaction::v1::{
    multisig_transaction_payload, transaction::TxnData, transaction_payload, EntryFunctionId,
    EntryFunctionPayload, Transaction, TransactionPayload,
};
use once_cell::sync::OnceCell;
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
    #[serde(skip)]
    #[builder(setter(skip))]
    standardized_sender: OnceCell<Option<String>>,
}

impl UserTransactionFilter {
    fn get_standardized_sender(&self) -> &Option<String> {
        self.standardized_sender.get_or_init(|| {
            self.sender
                .clone()
                .map(|address| standardize_address(&address))
        })
    }
}

impl From<aptos_protos::indexer::v1::UserTransactionFilter> for UserTransactionFilter {
    fn from(proto_filter: aptos_protos::indexer::v1::UserTransactionFilter) -> Self {
        Self {
            standardized_sender: OnceCell::with_value(
                proto_filter
                    .sender
                    .as_ref()
                    .map(|address| standardize_address(address)),
            ),
            sender: proto_filter.sender,
            payload: proto_filter.payload_filter.map(|f| f.into()),
        }
    }
}

impl From<UserTransactionFilter> for aptos_protos::indexer::v1::UserTransactionFilter {
    fn from(user_transaction_filter: UserTransactionFilter) -> Self {
        Self {
            sender: user_transaction_filter.sender,
            payload_filter: user_transaction_filter.payload.map(Into::into),
        }
    }
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
    fn matches(&self, txn: &Transaction) -> bool {
        let user_request = if let Some(TxnData::User(u)) = txn.txn_data.as_ref() {
            if let Some(user_request) = u.request.as_ref() {
                user_request
            } else {
                return false;
            }
        } else {
            return false;
        };

        if let Some(sender_filter) = self.get_standardized_sender() {
            if &standardize_address(&user_request.sender) != sender_filter {
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
                if !payload_filter.matches(payload) {
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
    #[serde(skip)]
    #[builder(setter(skip))]
    standardized_address: OnceCell<Option<String>>,
}

impl EntryFunctionFilter {
    fn get_standardized_address(&self) -> &Option<String> {
        self.standardized_address.get_or_init(|| {
            self.address
                .clone()
                .map(|address| standardize_address(&address))
        })
    }
}

impl From<aptos_protos::indexer::v1::EntryFunctionFilter> for EntryFunctionFilter {
    fn from(proto_filter: aptos_protos::indexer::v1::EntryFunctionFilter) -> Self {
        Self {
            standardized_address: OnceCell::with_value(
                proto_filter
                    .address
                    .as_ref()
                    .map(|address| standardize_address(address)),
            ),
            address: proto_filter.address,
            module: proto_filter.module_name,
            function: proto_filter.function,
        }
    }
}

impl From<EntryFunctionFilter> for aptos_protos::indexer::v1::EntryFunctionFilter {
    fn from(entry_function_filter: EntryFunctionFilter) -> Self {
        Self {
            address: entry_function_filter.address,
            module_name: entry_function_filter.module,
            function: entry_function_filter.function,
        }
    }
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
    fn matches(&self, module_id: &EntryFunctionId) -> bool {
        if !self.function.matches(&module_id.name) {
            return false;
        }

        if self.address.is_some() || self.function.is_some() {
            if let Some(module) = &module_id.module.as_ref() {
                if !(self
                    .get_standardized_address()
                    .matches(&standardize_address(&module.address))
                    && self.module.matches(&module.name))
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

impl From<aptos_protos::indexer::v1::UserTransactionPayloadFilter>
    for UserTransactionPayloadFilter
{
    fn from(proto_filter: aptos_protos::indexer::v1::UserTransactionPayloadFilter) -> Self {
        Self {
            function: proto_filter.entry_function_filter.map(|f| f.into()),
        }
    }
}

impl From<UserTransactionPayloadFilter>
    for aptos_protos::indexer::v1::UserTransactionPayloadFilter
{
    fn from(user_transaction_payload_filter: UserTransactionPayloadFilter) -> Self {
        Self {
            entry_function_filter: user_transaction_payload_filter.function.map(Into::into),
        }
    }
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
    fn matches(&self, payload: &EntryFunctionPayload) -> bool {
        self.function.matches_opt(&payload.function)
    }
}

/// Get the entry_function_payload from both UserPayload and MultisigPayload
fn get_entry_function_payload_from_transaction_payload(
    payload: &TransactionPayload,
) -> Option<&EntryFunctionPayload> {
    if let Some(payload) = &payload.payload {
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
    }
}
