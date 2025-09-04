// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable};
use anyhow::Error;
use velor_protos::transaction::v1::{transaction::TransactionType, Transaction};
use serde::{Deserialize, Serialize};

/// Example:
/// ```
/// use velor_transaction_filter::TransactionRootFilterBuilder;
///
/// let filter = TransactionRootFilterBuilder::default()
///   .success(true)
///   .build()
///   .unwrap();
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(strip_option), default)]
pub struct TransactionRootFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txn_type: Option<TransactionType>,
}

impl From<velor_protos::indexer::v1::TransactionRootFilter> for TransactionRootFilter {
    fn from(proto_filter: velor_protos::indexer::v1::TransactionRootFilter) -> Self {
        Self {
            success: proto_filter.success,
            txn_type: proto_filter
                .transaction_type
                .map(|_| proto_filter.transaction_type()),
        }
    }
}

impl From<TransactionRootFilter> for velor_protos::indexer::v1::TransactionRootFilter {
    fn from(transaction_root_filter: TransactionRootFilter) -> Self {
        Self {
            success: transaction_root_filter.success,
            transaction_type: transaction_root_filter.txn_type.map(Into::into),
        }
    }
}

impl Filterable<Transaction> for TransactionRootFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.success.is_none() && self.txn_type.is_none() {
            return Err(Error::msg("At least one of success or txn_types must be set").into());
        };
        Ok(())
    }

    #[inline]
    fn matches(&self, item: &Transaction) -> bool {
        if !self
            .success
            .matches_opt(&item.info.as_ref().map(|i| i.success))
        {
            return false;
        }

        if let Some(txn_type) = &self.txn_type {
            if txn_type
                != &TransactionType::try_from(item.r#type).expect("Invalid transaction type")
            {
                return false;
            }
        }

        true
    }
}
