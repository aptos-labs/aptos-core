// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use velor_sdk::{
    move_types::language_storage::TypeTag,
    types::{account_address::AccountAddress, contract_event::ContractEvent},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositMoveStruct {
    account: AccountAddress,
    amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorSnapshotu64MoveStruct {
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MintMoveStruct {
    collection: AccountAddress,
    index: AggregatorSnapshotu64MoveStruct,
    token: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurnMoveStruct {
    collection: AccountAddress,
    index: u64,
    token: AccountAddress,
    previous_owner: AccountAddress,
}

pub fn get_mint_token_addr(events: &[ContractEvent]) -> Result<AccountAddress> {
    let mint_event: MintMoveStruct =
        search_single_event_data(events, &TypeTag::from_str("0x4::collection::Mint")?)?;
    Ok(mint_event.token)
}

pub fn get_burn_token_addr(events: &[ContractEvent]) -> Result<AccountAddress> {
    let burn_event: BurnMoveStruct =
        search_single_event_data(events, &TypeTag::from_str("0x4::collection::Burn")?)?;
    Ok(burn_event.token)
}

fn search_event(events: &[ContractEvent], type_tag: &TypeTag) -> Vec<ContractEvent> {
    events
        .iter()
        .filter(|event| event.type_tag() == type_tag)
        .cloned()
        .collect::<Vec<_>>()
}

fn search_single_event_data<T>(events: &[ContractEvent], type_tag: &TypeTag) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let matching_events = search_event(events, type_tag);
    if matching_events.len() != 1 {
        bail!(
            "Expected 1 event, found: {}, events: {:?}",
            matching_events.len(),
            events
                .iter()
                .map(|event| event.type_tag().to_canonical_string())
                .collect::<Vec<_>>()
        );
    }
    let event = matching_events
        .first()
        .ok_or_else(|| anyhow::anyhow!("No deposit event found"))?;
    Ok(bcs::from_bytes::<T>(event.event_data())?)
}

pub fn get_deposit_dst(events: &[ContractEvent]) -> Result<AccountAddress> {
    let deposit_event: DepositMoveStruct = search_single_event_data(
        events,
        &TypeTag::from_str("0x1::coin::Deposit<0x1::velor_coin::VelorCoin>")?,
    )?;
    Ok(deposit_event.account)
}
