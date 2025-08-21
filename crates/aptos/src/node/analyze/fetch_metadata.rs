// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use aptos_api_types::{BlockMetadataTransaction, TransactionData};
use aptos_rest_client::{
    aptos_api_types::{IdentifierWrapper, MoveResource, WriteSetChange},
    Client as RestClient, Transaction, VersionedNewBlockEvent,
};
use aptos_types::{
    account_address::AccountAddress, account_config::NewBlockEvent,
    contract_event::TransactionEvent,
};
use std::str::FromStr;

const MAX_FETCH_BATCH_SIZE: u16 = 1000;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct ValidatorInfo {
    pub address: AccountAddress,
    pub voting_power: u64,
    pub validator_index: u16,
}

pub struct EpochInfo {
    pub epoch: u64,
    pub blocks: Vec<VersionedNewBlockEvent>,
    pub validators: Vec<ValidatorInfo>,
    pub partial: bool,
}

pub struct FetchMetadata {}

impl FetchMetadata {
    fn get_validator_addresses(
        data: &MoveResource,
        field_name: &str,
    ) -> Result<Vec<ValidatorInfo>> {
        fn extract_validator_address(validator: &serde_json::Value) -> Result<ValidatorInfo> {
            Ok(ValidatorInfo {
                address: AccountAddress::from_hex_literal(
                    validator.get("addr").unwrap().as_str().unwrap(),
                )
                .map_err(|e| anyhow!("Cannot parse address {:?}", e))?,
                voting_power: validator
                    .get("voting_power")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse()
                    .map_err(|e| anyhow!("Cannot parse voting_power {:?}", e))?,
                validator_index: validator
                    .get("config")
                    .unwrap()
                    .get("validator_index")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse()
                    .map_err(|e| anyhow!("Cannot parse validator_index {:?}", e))?,
            })
        }

        let validators_json = data
            .data
            .0
            .get(&IdentifierWrapper::from_str(field_name).unwrap())
            .unwrap();
        if let serde_json::Value::Array(validators_array) = validators_json {
            let mut validators: Vec<ValidatorInfo> = vec![];
            for validator in validators_array {
                validators.push(extract_validator_address(validator)?);
            }
            Ok(validators)
        } else {
            Err(anyhow!("{} validators not in json", field_name))
        }
    }

    async fn get_transactions_in_range(
        client: &RestClient,
        start: u64,
        last: u64,
    ) -> Result<Vec<Transaction>> {
        let mut result = Vec::new();
        let mut cursor = start;
        while cursor < last {
            let limit = std::cmp::min(MAX_FETCH_BATCH_SIZE as u64, last - cursor) as u16;
            let mut current = client
                .get_transactions(Some(cursor), Some(limit))
                .await?
                .into_inner();
            if current.is_empty() {
                return Err(anyhow!(
                    "No transactions returned with start={} and limit={}",
                    cursor,
                    limit
                ));
            }
            cursor += current.len() as u64;
            result.append(&mut current);
        }
        Ok(result)
    }

    fn get_validators_from_transaction(transaction: &Transaction) -> Result<Vec<ValidatorInfo>> {
        if let Ok(info) = transaction.transaction_info() {
            for change in &info.changes {
                if let WriteSetChange::WriteResource(resource) = change {
                    if resource.data.typ.name.0.as_str() == "ValidatorSet" {
                        // No pending at epoch change
                        assert_eq!(
                            Vec::<ValidatorInfo>::new(),
                            FetchMetadata::get_validator_addresses(
                                &resource.data,
                                "pending_inactive"
                            )?
                        );
                        assert_eq!(
                            Vec::<ValidatorInfo>::new(),
                            FetchMetadata::get_validator_addresses(
                                &resource.data,
                                "pending_active"
                            )?
                        );
                        return FetchMetadata::get_validator_addresses(
                            &resource.data,
                            "active_validators",
                        );
                    }
                }
            }
        }
        Err(anyhow!("Couldn't find ValidatorSet in the transaction"))
    }

    pub async fn fetch_new_block_events(
        client: &RestClient,
        start_epoch: Option<i64>,
        end_epoch: Option<i64>,
    ) -> Result<Vec<EpochInfo>> {
        let state = client.get_ledger_information().await?.into_inner();

        let oldest_block = client
            .get_block_by_height(state.oldest_block_height, false)
            .await?
            .into_inner();

        let oldest_block_metadata_txn = client
            .get_transaction_by_version(oldest_block.first_version.0)
            .await?
            .into_inner();

        let newest_block = client
            .get_block_by_height(state.block_height, false)
            .await?
            .into_inner();

        let newest_block_metadata_txn = client
            .get_transaction_by_version(newest_block.first_version.0)
            .await?
            .into_inner();

        fn as_block_metadata_txn(txn: Transaction) -> Option<BlockMetadataTransaction> {
            match txn {
                Transaction::BlockMetadataTransaction(txn) => Some(txn),
                _ => return None,
            }
        }

        let latest_epoch = as_block_metadata_txn(newest_block_metadata_txn)
            .map(|txn| txn.epoch.0)
            .unwrap_or(0);
        let oldest_epoch = as_block_metadata_txn(oldest_block_metadata_txn)
            .map(|txn| txn.epoch.0)
            .unwrap_or(0);

        let wanted_start_epoch = {
            let mut wanted_start_epoch = start_epoch.unwrap_or(2);
            if wanted_start_epoch < 0 {
                wanted_start_epoch = latest_epoch as i64 + wanted_start_epoch + 1;
            }

            let oldest_fetchable_epoch = std::cmp::max(oldest_epoch, 2);
            if oldest_fetchable_epoch > wanted_start_epoch as u64 {
                println!(
                    "Oldest full epoch that can be retrieved is {} ",
                    oldest_fetchable_epoch
                );
                oldest_fetchable_epoch
            } else {
                wanted_start_epoch as u64
            }
        };
        let wanted_end_epoch = {
            let mut wanted_end_epoch = end_epoch.unwrap_or(i64::MAX);
            if wanted_end_epoch < 0 {
                wanted_end_epoch = latest_epoch as i64 + wanted_end_epoch + 1;
            }
            std::cmp::min(latest_epoch + 1, std::cmp::max(2, wanted_end_epoch) as u64)
        };

        let mut start_block_height = state.oldest_block_height;
        let end_block_height = state.block_height;
        if wanted_start_epoch > 2 {
            let mut search_end = end_block_height;

            // Stop when search is close enough, and we can then linearly
            // proceed from there.
            // Since we are ignoring results we are fetching during binary search
            // we want to stop when we are close.
            while start_block_height + 20 < search_end {
                let mid = (start_block_height + search_end) / 2;

                let block = client.get_block_by_height(mid, false).await?.into_inner();
                let txn = client
                    .get_transaction_by_version(block.first_version.0)
                    .await?
                    .into_inner();

                let mid_epoch = as_block_metadata_txn(txn)
                    .map(|txn| txn.epoch.0)
                    .unwrap_or(0);

                if mid_epoch < wanted_start_epoch {
                    start_block_height = mid;
                } else {
                    search_end = mid;
                }
            }
        }

        let mut batch_index = 0;

        println!(
            "Fetching {} to {} versions, wanting epochs [{}, {}), last version: {} and epoch: {}",
            start_block_height,
            end_block_height,
            wanted_start_epoch,
            wanted_end_epoch,
            state.version,
            state.epoch,
        );
        let mut result: Vec<EpochInfo> = vec![];
        if wanted_start_epoch >= wanted_end_epoch {
            return Ok(result);
        }

        let mut validators: Vec<ValidatorInfo> = vec![];
        let mut current: Vec<VersionedNewBlockEvent> = vec![];
        let mut epoch = 0;

        let mut cursor = start_block_height;
        loop {
            let response = client.get_block_by_height(cursor, false).await;

            if response.is_err() {
                println!(
                    "Failed to read block beyond {}, stopping. {:?}",
                    cursor,
                    response.unwrap_err()
                );
                assert!(!validators.is_empty());
                result.push(EpochInfo {
                    epoch,
                    blocks: current,
                    validators: validators.clone(),
                    partial: true,
                });
                return Ok(result);
            }
            let block = response.unwrap().into_inner();

            let response = client
                .get_transaction_by_version_bcs(block.first_version.0)
                .await;
            if response.is_err() {
                println!(
                    "Failed to read block metadata transaction beyond {}, stopping. {:?}",
                    cursor,
                    response.unwrap_err()
                );
                assert!(!validators.is_empty());
                result.push(EpochInfo {
                    epoch,
                    blocks: current,
                    validators: validators.clone(),
                    partial: true,
                });
                return Ok(result);
            }

            let TransactionData::OnChain(txn) = response.unwrap().into_inner() else {
                bail!("Expected TransactionData::OnChain");
            };
            let events = txn.events;

            if events.is_empty() {
                return Err(anyhow!(
                    "No transactions returned with start={} and limit={}",
                    cursor,
                    MAX_FETCH_BATCH_SIZE
                ));
            }

            assert_eq!(events.len(), 1);
            cursor += events.len() as u64;
            batch_index += 1;

            for raw_event in events {
                let new_block_event =
                    bcs::from_bytes::<NewBlockEvent>(raw_event.get_event_data()).unwrap();
                println!("processing event: {:?}", new_block_event);
                let event_epoch = new_block_event.epoch;
                let versioned_new_block_event = VersionedNewBlockEvent {
                    event: new_block_event,
                    version: block.first_version.0,
                    sequence_number: raw_event.v1()?.sequence_number(),
                };
                if event_epoch > epoch {
                    if epoch == 0 {
                        epoch = event_epoch;
                        current = vec![];
                    } else {
                        let last = current.last().cloned();
                        if let Some(last) = last {
                            let transactions = FetchMetadata::get_transactions_in_range(
                                client,
                                last.version,
                                versioned_new_block_event.version,
                            )
                            .await?;
                            assert_eq!(
                                transactions.first().unwrap().version().unwrap(),
                                last.version
                            );
                            for transaction in transactions {
                                if let Ok(new_validators) =
                                    FetchMetadata::get_validators_from_transaction(&transaction)
                                {
                                    if epoch >= wanted_start_epoch {
                                        assert!(!validators.is_empty());
                                        result.push(EpochInfo {
                                            epoch,
                                            blocks: current,
                                            validators: validators.clone(),
                                            partial: false,
                                        });
                                    }
                                    current = vec![];

                                    validators = new_validators;
                                    validators.sort_by_key(|v| v.validator_index);
                                    assert_eq!(epoch + 1, event_epoch);
                                    epoch = event_epoch;
                                    if epoch >= wanted_end_epoch {
                                        return Ok(result);
                                    }
                                    break;
                                }
                            }
                            assert!(
                                current.is_empty(),
                                "Couldn't find ValidatorSet change for transactions start={}, limit={} for epoch {}",
                                last.version,
                                block.first_version.0 - last.version,
                                event_epoch,
                            );
                        }
                    }
                }

                current.push(versioned_new_block_event);
            }

            if batch_index % 100 == 0 {
                println!(
                    "Fetched {} epochs (in epoch {} with {} blocks) from {} NewBlockEvents",
                    result.len(),
                    epoch,
                    current.len(),
                    cursor
                );
            }

            if cursor > end_block_height {
                if !validators.is_empty() {
                    result.push(EpochInfo {
                        epoch,
                        blocks: current,
                        validators: validators.clone(),
                        partial: true,
                    });
                }
                return Ok(result);
            }
        }
    }
}
