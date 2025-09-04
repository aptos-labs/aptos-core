// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use velor_rest_client::{
    velor_api_types::{IdentifierWrapper, MoveResource, WriteSetChange},
    Client as RestClient, Transaction, VersionedNewBlockEvent,
};
use velor_types::account_address::AccountAddress;
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
        let (last_events, state) = client
            .get_new_block_events_bcs(None, Some(1))
            .await?
            .into_parts();
        let mut start_seq_num = state.oldest_block_height;
        assert_eq!(last_events.len(), 1, "{:?}", last_events);
        let last_event = last_events.first().unwrap();
        let last_seq_num = last_event.sequence_number;

        let wanted_start_epoch = {
            let mut wanted_start_epoch = start_epoch.unwrap_or(2);
            if wanted_start_epoch < 0 {
                wanted_start_epoch = last_event.event.epoch() as i64 + wanted_start_epoch + 1;
            }

            let oldest_event = client
                .get_new_block_events_bcs(Some(start_seq_num), Some(1))
                .await?
                .into_inner()
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("No blocks at oldest_block_height {}", start_seq_num))?;
            let oldest_fetchable_epoch = std::cmp::max(oldest_event.event.epoch() + 1, 2);
            if oldest_fetchable_epoch > wanted_start_epoch as u64 {
                println!(
                    "Oldest full epoch that can be retreived is {} ",
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
                wanted_end_epoch = last_event.event.epoch() as i64 + wanted_end_epoch + 1;
            }
            std::cmp::min(
                last_event.event.epoch() + 1,
                std::cmp::max(2, wanted_end_epoch) as u64,
            )
        };

        if wanted_start_epoch > 2 {
            let mut search_end = last_seq_num;

            // Stop when search is close enough, and we can then linearly
            // proceed from there.
            // Since we are ignoring results we are fetching during binary search
            // we want to stop when we are close.
            while start_seq_num + 20 < search_end {
                let mid = (start_seq_num + search_end) / 2;

                let mid_epoch = client
                    .get_new_block_events_bcs(Some(mid), Some(1))
                    .await?
                    .into_inner()
                    .first()
                    .unwrap()
                    .event
                    .epoch();

                if mid_epoch < wanted_start_epoch {
                    start_seq_num = mid;
                } else {
                    search_end = mid;
                }
            }
        }

        let mut batch_index = 0;

        println!(
            "Fetching {} to {} sequence number, wanting epochs [{}, {}), last version: {} and epoch: {}",
            start_seq_num, last_seq_num, wanted_start_epoch, wanted_end_epoch, state.version, state.epoch,
        );
        let mut result: Vec<EpochInfo> = vec![];
        if wanted_start_epoch >= wanted_end_epoch {
            return Ok(result);
        }

        let mut validators: Vec<ValidatorInfo> = vec![];
        let mut current: Vec<VersionedNewBlockEvent> = vec![];
        let mut epoch = 0;

        let mut cursor = start_seq_num;
        loop {
            let response = client
                .get_new_block_events_bcs(Some(cursor), Some(MAX_FETCH_BATCH_SIZE))
                .await;

            if response.is_err() {
                println!(
                    "Failed to read new_block_events beyond {}, stopping. {:?}",
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
            let events = response.unwrap().into_inner();

            if events.is_empty() {
                return Err(anyhow!(
                    "No transactions returned with start={} and limit={}",
                    cursor,
                    MAX_FETCH_BATCH_SIZE
                ));
            }

            cursor += events.len() as u64;
            batch_index += 1;

            for event in events {
                if event.event.epoch() > epoch {
                    if epoch == 0 {
                        epoch = event.event.epoch();
                        current = vec![];
                    } else {
                        let last = current.last().cloned();
                        if let Some(last) = last {
                            let transactions = FetchMetadata::get_transactions_in_range(
                                client,
                                last.version,
                                event.version,
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
                                    assert_eq!(epoch + 1, event.event.epoch());
                                    epoch = event.event.epoch();
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
                                event.version - last.version,
                                event.event.epoch(),
                            );
                        }
                    }
                }
                current.push(event);
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

            if cursor > last_seq_num {
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
