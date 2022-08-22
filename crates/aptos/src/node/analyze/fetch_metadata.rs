// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_rest_client::{
    aptos_api_types::{IdentifierWrapper, MoveResource, WriteSetChange},
    Client as RestClient, Transaction, VersionedNewBlockEvent,
};
use aptos_types::account_address::AccountAddress;
use std::convert::TryFrom;
use std::str::FromStr;

pub struct EpochInfo {
    pub epoch: u64,
    pub blocks: Vec<VersionedNewBlockEvent>,
    pub validators: Vec<AccountAddress>,
}

pub struct FetchMetadata {}

impl FetchMetadata {
    fn get_validator_addresses(
        data: &MoveResource,
        field_name: &str,
    ) -> Result<Vec<AccountAddress>> {
        fn extract_validator_address(validator: &serde_json::Value) -> Result<AccountAddress> {
            if let serde_json::Value::Object(value) = validator {
                if let Some(serde_json::Value::String(address)) = &value.get("addr") {
                    AccountAddress::from_hex_literal(address)
                        .map_err(|e| anyhow!("Cannot parse address {:?}", e))
                } else {
                    Err(anyhow!("Addr not present or of correct type"))
                }
            } else {
                Err(anyhow!("Validator config not a json object"))
            }
        }

        let validators_json = data
            .data
            .0
            .get(&IdentifierWrapper::from_str(field_name).unwrap())
            .unwrap();
        if let serde_json::Value::Array(validators_array) = validators_json {
            let mut validators: Vec<AccountAddress> = vec![];
            for validator in validators_array {
                validators.push(extract_validator_address(validator)?);
            }
            Ok(validators)
        } else {
            Err(anyhow!("{} validators not in json", field_name))
        }
    }

    fn get_validators_from_transaction(transaction: &Transaction) -> Result<Vec<AccountAddress>> {
        if let Ok(info) = transaction.transaction_info() {
            for change in &info.changes {
                if let WriteSetChange::WriteResource(resource) = change {
                    if resource.data.typ.name.0.as_str() == "ValidatorSet" {
                        // No pending at epoch change
                        assert_eq!(
                            Vec::<AccountAddress>::new(),
                            FetchMetadata::get_validator_addresses(
                                &resource.data,
                                "pending_inactive"
                            )?
                        );
                        assert_eq!(
                            Vec::<AccountAddress>::new(),
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
        start_epoch: Option<u64>,
        end_epoch: Option<u64>,
    ) -> Result<Vec<EpochInfo>> {
        let mut start_seq_num = 0;
        let last_seq_num = client
            .get_new_block_events(None, Some(1))
            .await?
            .into_inner()
            .first()
            .unwrap()
            .sequence_number;

        if let Some(start_epoch) = start_epoch {
            if start_epoch > 1 {
                let mut search_end = last_seq_num;

                // Stop when search is close enough, and we can then linearly
                // proceed from there.
                // Since we are ignoring results we are fetching during binary search
                // we want to stop when we are close.
                while start_seq_num + 20 < search_end {
                    let mid = (start_seq_num + search_end) / 2;

                    let mid_epoch = client
                        .get_new_block_events(Some(mid), Some(1))
                        .await?
                        .into_inner()
                        .first()
                        .unwrap()
                        .event
                        .epoch();

                    if mid_epoch < start_epoch {
                        start_seq_num = mid;
                    } else {
                        search_end = mid;
                    }
                }
            }
        }

        let batch: u16 = 1000;
        let mut batch_index = 0;

        println!(
            "Fetching {} to {} sequence number",
            start_seq_num, last_seq_num
        );

        let mut validators: Vec<AccountAddress> = vec![];
        let mut epoch = 0;

        let mut current: Vec<VersionedNewBlockEvent> = vec![];
        let mut result: Vec<EpochInfo> = vec![];

        let mut cursor = start_seq_num;
        let start_epoch = start_epoch.unwrap_or(2);
        loop {
            let events = client.get_new_block_events(Some(cursor), Some(batch)).await;

            if events.is_err() {
                println!(
                    "Failed to read new_block_events beyond {}, stopping. {:?}",
                    cursor,
                    events.unwrap_err()
                );
                assert!(!validators.is_empty());
                result.push(EpochInfo {
                    epoch,
                    blocks: current,
                    validators: validators.clone(),
                });
                return Ok(result);
            }

            for event in events.unwrap().into_inner() {
                if event.event.epoch() > epoch {
                    if epoch == 0 {
                        epoch = event.event.epoch();
                        current = vec![];
                    } else {
                        let last = current.last().cloned();
                        if let Some(last) = last {
                            let transactions = client
                                .get_transactions(
                                    Some(last.version),
                                    Some(u16::try_from(event.version - last.version).unwrap()),
                                )
                                .await?
                                .into_inner();
                            assert_eq!(
                                transactions.first().unwrap().version().unwrap(),
                                last.version
                            );
                            for transaction in transactions {
                                if let Ok(new_validators) =
                                    FetchMetadata::get_validators_from_transaction(&transaction)
                                {
                                    if epoch >= start_epoch {
                                        assert!(!validators.is_empty());
                                        result.push(EpochInfo {
                                            epoch,
                                            blocks: current,
                                            validators: validators.clone(),
                                        });
                                    }
                                    current = vec![];

                                    validators = new_validators;
                                    validators.sort();
                                    assert_eq!(epoch + 1, event.event.epoch());
                                    epoch = event.event.epoch();
                                    if end_epoch.is_some() && epoch >= end_epoch.unwrap() {
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

            cursor += u64::from(batch);
            batch_index += 1;
            if batch_index % 100 == 0 {
                println!(
                    "Fetched {} epochs (in epoch {} with {} blocks) from {} transactions",
                    result.len(),
                    epoch,
                    current.len(),
                    cursor
                );
            }

            if cursor > last_seq_num {
                return Ok(result);
            }
        }
    }
}
