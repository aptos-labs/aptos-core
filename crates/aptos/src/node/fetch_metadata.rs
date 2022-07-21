// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::analyze_validators::NewBlockInfo;
use anyhow::Result;
use aptos_rest_client::{
    aptos_api_types::{BlockMetadataTransaction, IdentifierWrapper, MoveResource, WriteSetChange},
    Client as RestClient, Transaction,
};
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::NewBlockEvent;
use std::convert::TryFrom;
use std::str::FromStr;

pub struct EpochInfo {
    pub epoch: u64,
    pub blocks: Vec<NewBlockInfo>,
    pub validators: Vec<AccountAddress>,
}

pub struct FetchMetadata {}

impl FetchMetadata {
    fn metadata_to_new_block_event(metadata: &BlockMetadataTransaction) -> NewBlockInfo {
        NewBlockInfo {
            event: NewBlockEvent::new(
                *metadata.epoch.inner(),
                *metadata.round.inner(),
                0,
                metadata.previous_block_votes.clone(),
                *metadata.proposer.inner(),
                metadata
                    .failed_proposer_indices
                    .iter()
                    .map(|i| *i as u64)
                    .collect(),
                *metadata.timestamp.inner(),
            ),
            version: metadata.info.version.into(),
        }
    }

    async fn get_epoch_at_version(
        mut cursor: u64,
        batch: u64,
        last_version: u64,
        client: &RestClient,
    ) -> Result<u64> {
        loop {
            let wanted = client
                .get_transactions(Some(cursor), Some(u16::try_from(batch).unwrap()))
                .await?
                .into_inner()
                .into_iter()
                .filter(|t| matches!(t, Transaction::BlockMetadataTransaction(_)))
                .next();
            if let Some(t) = wanted {
                if let Transaction::BlockMetadataTransaction(m) = t {
                    return Ok(*m.epoch.inner());
                }
            }
            cursor += batch;
            if cursor > last_version {
                return Ok(u64::MAX);
            }
        }
    }

    fn get_validator_addresses(
        data: &MoveResource,
        field_name: &str,
    ) -> Option<Vec<AccountAddress>> {
        let active_validators_json = data
            .data
            .0
            .get(&IdentifierWrapper::from_str(field_name).unwrap())
            .unwrap();
        if let serde_json::Value::Array(active_validators) = active_validators_json {
            let mut validators: Vec<AccountAddress> = vec![];
            for validator in active_validators {
                if let serde_json::Value::Object(value) = validator {
                    if let serde_json::Value::String(address) = &value["addr"] {
                        validators.push(AccountAddress::from_hex_literal(&address).unwrap());
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            Some(validators)
        } else {
            None
        }
    }

    pub async fn fetch_new_block_events(
        client: &RestClient,
        start_epoch: Option<u64>,
        end_epoch: Option<u64>,
    ) -> Result<Vec<EpochInfo>> {
        let mut start_version = 0;
        let last_version = client
            .get_transactions(None, Some(1))
            .await?
            .into_inner()
            .first()
            .unwrap()
            .version()
            .unwrap();

        let batch: u64 = 1000;
        if let Some(start_epoch) = start_epoch {
            if start_epoch > 1 {
                let mut search_end = last_version;

                while start_version + 5 * batch < search_end {
                    let mid = (start_version + search_end) / 2;

                    let mid_epoch =
                        FetchMetadata::get_epoch_at_version(mid, batch, last_version, &client)
                            .await?;
                    if mid_epoch < start_epoch {
                        start_version = mid;
                    } else {
                        search_end = mid;
                    }
                }
            }
        }
        println!("Fetching {} to {}", start_version, last_version);

        let mut validators: Vec<AccountAddress> = vec![];
        let mut epoch = 0;

        let mut current: Vec<NewBlockInfo> = vec![];
        let mut result: Vec<EpochInfo> = vec![];

        let mut cursor = start_version;
        let start_epoch = start_epoch.unwrap_or(2);
        loop {
            let transactions = client
                .get_transactions(Some(cursor), Some(u16::try_from(batch).unwrap()))
                .await;

            if transactions.is_err() {
                println!(
                    "Failed to read transactions beyond {}, stopping. {:?}",
                    cursor,
                    transactions.unwrap_err()
                );
                assert!(!validators.is_empty());
                result.push(EpochInfo {
                    epoch,
                    blocks: current,
                    validators: validators.clone(),
                });
                return Ok(result);
            }
            for t in transactions.unwrap().into_inner() {
                if let Transaction::BlockMetadataTransaction(metadata) = t {
                    current.push(FetchMetadata::metadata_to_new_block_event(&metadata));
                    for change in &metadata.info.changes {
                        if let WriteSetChange::WriteResource(resource) = change {
                            if resource.data.typ.name.0.clone().into_string() == "ValidatorSet" {
                                if epoch >= start_epoch {
                                    assert!(!validators.is_empty());
                                    result.push(EpochInfo {
                                        epoch,
                                        blocks: current,
                                        validators: validators.clone(),
                                    });
                                }
                                current = vec![];

                                if let Some(v) = FetchMetadata::get_validator_addresses(
                                    &resource.data,
                                    "active_validators",
                                ) {
                                    validators = v;
                                    validators.sort();
                                    epoch = metadata.epoch.0 + 1;
                                    if end_epoch.is_some() && epoch >= end_epoch.unwrap() {
                                        return Ok(result);
                                    }
                                } else {
                                    validators = vec![];
                                }
                                // No pending at epoch change
                                assert_eq!(
                                    Some(vec![]),
                                    FetchMetadata::get_validator_addresses(
                                        &resource.data,
                                        "pending_inactive"
                                    )
                                );
                                assert_eq!(
                                    Some(vec![]),
                                    FetchMetadata::get_validator_addresses(
                                        &resource.data,
                                        "pending_active"
                                    )
                                );
                            }
                        }
                    }
                }
            }

            cursor += batch;
            if cursor % 100000 == 0 {
                println!(
                    "Fetched {} epochs (in epoch {} with {} blocks) from {} transactions",
                    result.len(),
                    epoch,
                    current.len(),
                    cursor
                );
            }

            if cursor > last_version {
                return Ok(result);
            }
        }
    }
}
