// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::event_lookup::get_deposit_dst;
use anyhow::{anyhow, Result};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    rest_client::aptos_api_types::TransactionOnChainData,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        serde_helper::bcs_utils::bcs_size_of_byte_array,
        transaction::{SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use rand::{distributions::Alphanumeric, Rng};
use std::{fs::read_to_string, path::Path};

pub trait SignedTransactionBuilder<T> {
    fn build(
        &self,
        data: &T,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction;

    fn success_output(&self, _data: &T, txn_out: &Option<TransactionOnChainData>) -> String {
        match txn_out {
            Some(_txn_out) => "success",
            None => "failure",
        }
        .to_string()
    }
}

pub fn create_account_addresses_work(
    destinations_file: &str,
    only_success: bool,
) -> Result<Vec<AccountAddress>> {
    read_to_string(Path::new(destinations_file))?
        .lines()
        .filter(|s| !only_success || s.ends_with("\tsuccess"))
        .filter_map(|s| s.split('\t').next())
        .filter(|s| !s.is_empty())
        .map(|text| {
            AccountAddress::from_str_strict(text)
                .map_err(|e| anyhow!("failed to parse {}, {:?}", text, e))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn parse_line_vec(line: &str) -> Result<(AccountAddress, AccountAddress)> {
    let mut parts = line.split('\t');
    let first = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("No first part"))?;
    let second = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("No second part"))?;
    Ok((
        AccountAddress::from_str_strict(first)
            .map_err(|e| anyhow!("failed to parse {}, {:?}", first, e))?,
        AccountAddress::from_str_strict(second)
            .map_err(|e| anyhow!("failed to parse {}, {:?}", second, e))?,
    ))
}

pub async fn create_account_address_pairs_work(
    destinations_file: &str,
    only_success: bool,
) -> Result<Vec<(AccountAddress, AccountAddress)>> {
    read_to_string(Path::new(destinations_file))?
        .lines()
        .filter(|s| !only_success || s.ends_with("\tsuccess"))
        .map(parse_line_vec)
        .collect::<Result<Vec<_>, _>>()
}

pub fn rand_string(len: usize) -> String {
    let res = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    assert_eq!(
        bcs::serialized_size(&res).unwrap(),
        bcs_size_of_byte_array(len)
    );
    res
}

// Example transaction builders:

pub struct PayloadSignedTransactionBuilder;

impl SignedTransactionBuilder<TransactionPayload> for PayloadSignedTransactionBuilder {
    fn build(
        &self,
        data: &TransactionPayload,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(txn_factory.payload(data.clone()))
    }
}

pub struct FixedPayloadSignedTransactionBuilder {
    pub payload: TransactionPayload,
}

impl FixedPayloadSignedTransactionBuilder {
    pub fn new(payload: TransactionPayload) -> Self {
        Self { payload }
    }
}

impl SignedTransactionBuilder<()> for FixedPayloadSignedTransactionBuilder {
    fn build(
        &self,
        _data: &(),
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(txn_factory.payload(self.payload.clone()))
    }
}

pub struct TransferAptSignedTransactionBuilder {
    pub amount_to_send: u64,
}

impl SignedTransactionBuilder<AccountAddress> for TransferAptSignedTransactionBuilder {
    fn build(
        &self,
        data: &AccountAddress,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(txn_factory.payload(
            aptos_stdlib::aptos_coin_transfer(*data, self.amount_to_send),
        ))
    }

    fn success_output(
        &self,
        data: &AccountAddress,
        txn_out: &Option<TransactionOnChainData>,
    ) -> String {
        let (status, dst) = match txn_out {
            Some(txn_out) => match get_deposit_dst(&txn_out.events) {
                Ok(dst) => {
                    assert_eq!(&dst, data);
                    ("success".to_string(), dst.to_standard_string())
                },
                Err(e) => (e.to_string(), data.to_standard_string()),
            },
            None => ("missing".to_string(), data.to_standard_string()),
        };
        format!("{}\t{}", dst, status)
    }
}

pub struct CreateAndTransferAptSignedTransactionBuilder {
    pub amount_to_send: u64,
}

impl SignedTransactionBuilder<AccountAddress> for CreateAndTransferAptSignedTransactionBuilder {
    fn build(
        &self,
        data: &AccountAddress,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(txn_factory.payload(
            aptos_stdlib::aptos_account_transfer(*data, self.amount_to_send),
        ))
    }

    fn success_output(
        &self,
        data: &AccountAddress,
        txn_out: &Option<TransactionOnChainData>,
    ) -> String {
        let (status, dst) = match txn_out {
            Some(txn_out) => match get_deposit_dst(&txn_out.events) {
                Ok(dst) => {
                    assert_eq!(&dst, data);
                    ("success", dst.to_standard_string())
                },
                Err(_e) => ("error", data.to_standard_string()),
            },
            None => ("missing", data.to_standard_string()),
        };
        format!("{}\t{}", dst, status)
    }
}
