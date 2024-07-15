// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable};
use anyhow::Error;
use aptos_protos::transaction::v1::{transaction::TransactionType, Transaction};
use serde::{Deserialize, Serialize};

/// Example:
/// ```
/// use aptos_transaction_filter::TransactionRootFilterBuilder;
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

impl Filterable<Transaction> for TransactionRootFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.success.is_none() && self.txn_type.is_none() {
            return Err(Error::msg("At least one of success or txn_types must be set").into());
        };
        Ok(())
    }

    #[inline]
    fn is_allowed(&self, item: &Transaction) -> bool {
        if !self
            .success
            .is_allowed_opt(&item.info.as_ref().map(|i| i.success))
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
