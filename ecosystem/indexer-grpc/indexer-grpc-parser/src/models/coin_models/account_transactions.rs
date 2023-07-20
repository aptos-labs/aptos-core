// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    models::{
        default_models::user_transactions::UserTransaction,
        token_models::v2_token_utils::ObjectWithMetadata,
    },
    schema::account_transactions,
    utils::util::standardize_address,
};
use aptos_protos::transaction::v1::{
    transaction::TxnData, write_set_change::Change, DeleteResource, Event, Transaction,
    WriteResource,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type AccountTransactionPK = (String, i64);

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(account_address, transaction_version))]
#[diesel(table_name = account_transactions)]
pub struct AccountTransaction {
    pub transaction_version: i64,
    pub account_address: String,
}

impl AccountTransaction {
    /// This table will record every transaction that touch an account which could be
    /// a user account, an object, or a resource account.
    /// We will consider all transactions that modify a resource or event associated with a particular account.
    /// We will do 1 level of redirection for now (e.g. if it's an object, we will record the owner as account address).
    /// We will also consider transactions that the account signed or is part of a multi sig / multi agent.
    /// TODO: recursively find the parent account of an object
    /// TODO: include table items in the detection path
    pub fn from_transaction(transaction: &Transaction) -> HashMap<AccountTransactionPK, Self> {
        let txn_version = transaction.version as i64;
        let txn_data = transaction
            .txn_data
            .as_ref()
            .unwrap_or_else(|| panic!("Txn Data doesn't exit for version {}", txn_version));
        let transaction_info = transaction.info.as_ref().unwrap_or_else(|| {
            panic!("Transaction info doesn't exist for version {}", txn_version)
        });
        let wscs = &transaction_info.changes;
        let (events, signatures) = match txn_data {
            TxnData::User(inner) => (
                &inner.events,
                UserTransaction::get_signatures(
                    inner.request.as_ref().unwrap_or_else(|| {
                        panic!("User request doesn't exist for version {}", txn_version)
                    }),
                    txn_version,
                    transaction.block_height as i64,
                ),
            ),
            TxnData::Genesis(inner) => (&inner.events, vec![]),
            TxnData::BlockMetadata(inner) => (&inner.events, vec![]),
            _ => {
                return HashMap::new();
            },
        };
        let mut account_transactions = HashMap::new();
        for sig in &signatures {
            account_transactions.insert((sig.signer.clone(), txn_version), Self {
                transaction_version: txn_version,
                account_address: sig.signer.clone(),
            });
        }
        for event in events {
            account_transactions.extend(Self::from_event(event, txn_version));
        }
        for wsc in wscs {
            match wsc.change.as_ref().unwrap() {
                Change::DeleteResource(res) => {
                    account_transactions
                        .extend(Self::from_delete_resource(res, txn_version).unwrap());
                },
                Change::WriteResource(res) => {
                    account_transactions
                        .extend(Self::from_write_resource(res, txn_version).unwrap());
                },
                _ => {},
            }
        }
        account_transactions
    }

    /// Base case, record event account address. We don't really have to worry about
    /// objects here because it'll be taken care of in the resource section
    fn from_event(event: &Event, txn_version: i64) -> HashMap<AccountTransactionPK, Self> {
        let account_address =
            standardize_address(event.key.as_ref().unwrap().account_address.as_str());
        HashMap::from([((account_address.clone(), txn_version), Self {
            transaction_version: txn_version,
            account_address,
        })])
    }

    /// Base case, record resource account. If the resource is an object, then we record the owner as well
    /// This handles partial deletes as well
    fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<HashMap<AccountTransactionPK, Self>> {
        let mut result = HashMap::new();
        let account_address = standardize_address(write_resource.address.as_str());
        result.insert((account_address.clone(), txn_version), Self {
            transaction_version: txn_version,
            account_address,
        });
        if let Some(inner) = &ObjectWithMetadata::from_write_resource(write_resource, txn_version)?
        {
            result.insert((inner.object_core.get_owner_address(), txn_version), Self {
                transaction_version: txn_version,
                account_address: inner.object_core.get_owner_address(),
            });
        }
        Ok(result)
    }

    /// Base case, record resource account.
    /// TODO: If the resource is an object, then we need to look for the latest owner. This isn't really possible
    /// right now given we have parallel threads so it'll be very difficult to ensure that we have the correct
    /// latest owner
    fn from_delete_resource(
        delete_resource: &DeleteResource,
        txn_version: i64,
    ) -> anyhow::Result<HashMap<AccountTransactionPK, Self>> {
        let mut result = HashMap::new();
        let account_address = standardize_address(delete_resource.address.as_str());
        result.insert((account_address.clone(), txn_version), Self {
            transaction_version: txn_version,
            account_address,
        });
        Ok(result)
    }
}
