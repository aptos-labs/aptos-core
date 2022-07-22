// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Types in move that are converted to Rust for BCS deserialization
//!
//! These should be implemented in language specific formats so that it can be deserialized
//! accordingly with BCS.
//!
//! TODO: These types should be generated from move code
//! TODO: This should be in the SDK and not the client

use aptos_api_types::U64;
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AptosCoin {
    pub value: U64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pub coin: AptosCoin,
}

impl Balance {
    pub fn get(&self) -> u64 {
        *self.coin.value.inner()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AptosVersion {
    pub major: U64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    authentication_key: Vec<u8>,
    sequence_number: u64,
    self_address: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockMetadata {
    /// Height of the current block
    height: u64,
    /// Time period between epochs.
    epoch_internal: u64,
    /// Handle where events with the time of new blocks are emitted
    new_block_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventHandle {
    /// Total number of events emitted to this event stream.
    counter: u64,
    /// A globally unique ID for this event stream.
    guid: Guid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Guid {
    id: Id,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Id {
    /// If creation_num is `i`, this is the `i+1`th GUID created by `addr`
    creation_num: u64,
    /// Address that created the GUID
    addr: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewBlockEvent {
    epoch: u64,
    round: u64,
    height: u64,
    previous_block_votes: Vec<bool>,
    proposer: AccountAddress,
    failed_proposer_indices: Vec<u64>,
    /// On-chain time during the block at the given height
    time_microseconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinInfo {
    name: String,
    symbol: String,
    decimals: u64,
    supply: Option<u128>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CoinStore {
    coin: Coin,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Coin {
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositEvent {
    amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawEvent {
    amount: u64,
}
