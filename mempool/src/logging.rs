// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::shared_mempool::types::{MempoolMessageId, QuorumStoreRequest};
use anyhow::Error;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_logger::Schema;
use aptos_mempool_notifications::MempoolCommitNotification;
use aptos_types::{account_address::AccountAddress, transaction::ReplayProtector};
use serde::Serialize;
use std::{fmt, fmt::Write, time::SystemTime};

#[derive(Default)]
pub struct TxnsLog {
    txns: Vec<(
        AccountAddress,
        ReplayProtector,
        Option<String>,
        Option<SystemTime>,
    )>,
    len: usize,
    max_displayed: usize,
}

impl TxnsLog {
    pub fn new() -> Self {
        Self::new_with_max(usize::MAX)
    }

    pub fn new_with_max(max_displayed: usize) -> Self {
        Self {
            txns: vec![],
            len: 0,
            max_displayed,
        }
    }

    pub fn new_txn(account: AccountAddress, replay_protector: ReplayProtector) -> Self {
        Self {
            txns: vec![(account, replay_protector, None, None)],
            len: 0,
            max_displayed: usize::MAX,
        }
    }

    pub fn add(&mut self, account: AccountAddress, replay_protector: ReplayProtector) {
        if self.txns.len() < self.max_displayed {
            self.txns.push((account, replay_protector, None, None));
        }
        self.len += 1;
    }

    pub fn add_with_status(
        &mut self,
        account: AccountAddress,
        replay_protector: ReplayProtector,
        status: &str,
    ) {
        if self.txns.len() < self.max_displayed {
            self.txns
                .push((account, replay_protector, Some(status.to_string()), None));
        }
        self.len += 1;
    }

    pub fn add_full_metadata(
        &mut self,
        account: AccountAddress,
        replay_protector: ReplayProtector,
        status: &str,
        timestamp: SystemTime,
    ) {
        if self.txns.len() < self.max_displayed {
            self.txns.push((
                account,
                replay_protector,
                Some(status.to_string()),
                Some(timestamp),
            ));
        }
        self.len += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl fmt::Display for TxnsLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut txns = "".to_string();

        for (account, replay_protector, status, timestamp) in self.txns.iter() {
            let mut txn = format!("{}:{}", account, replay_protector);
            if let Some(status) = status {
                write!(txn, ":{}", status)?;
            }
            if let Some(timestamp) = timestamp {
                write!(txn, ":{:?}", timestamp)?;
            }

            write!(txns, "{} ", txn)?;
        }

        write!(f, "{}/{} txns: {}", self.txns.len(), self.len, txns)
    }
}

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    event: Option<LogEvent>,
    #[schema(debug)]
    error: Option<&'a Error>,
    #[schema(display)]
    peer: Option<&'a PeerNetworkId>,
    is_upstream_peer: Option<bool>,
    #[schema(display)]
    txns: Option<TxnsLog>,
    account: Option<AccountAddress>,
    #[schema(display)]
    quorum_store_msg: Option<&'a QuorumStoreRequest>,
    #[schema(display)]
    state_sync_msg: Option<&'a MempoolCommitNotification>,
    network_level: Option<usize>,
    upstream_network: Option<&'a NetworkId>,
    #[schema(debug)]
    message_id: Option<&'a MempoolMessageId>,
    backpressure: Option<bool>,
    num_txns: Option<usize>,
}

impl<'a> LogSchema<'a> {
    pub fn new(name: LogEntry) -> Self {
        Self::new_event(name, None)
    }

    pub fn event_log(name: LogEntry, event: LogEvent) -> Self {
        Self::new_event(name, Some(event))
    }

    pub fn new_event(name: LogEntry, event: Option<LogEvent>) -> Self {
        Self {
            name,
            event,
            error: None,
            peer: None,
            is_upstream_peer: None,
            account: None,
            txns: None,
            quorum_store_msg: None,
            state_sync_msg: None,
            network_level: None,
            upstream_network: None,
            message_id: None,
            backpressure: None,
            num_txns: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    NewPeer,
    LostPeer,
    CoordinatorRuntime,
    GCRuntime,
    ReconfigUpdate,
    JsonRpc,
    GetTransaction,
    GetBlock,
    QuorumStore,
    StateSyncCommit,
    BroadcastTransaction,
    BroadcastACK,
    ReceiveACK,
    AddTxn,
    RemoveTxn,
    MempoolFullEvictedTxn,
    GCRemoveTxns,
    CleanCommittedTxn,
    CleanRejectedTxn,
    ProcessReadyTxns,
    DBError,
    UnexpectedNetworkMsg,
    MempoolSnapshot,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    // Runtime events
    Start,
    Live,
    Terminated,

    // VM reconfig events
    Received,
    Process,
    VMUpdateFail,

    CallbackFail,
    NetworkSendFail,

    // garbage-collect txns events
    SystemTTLExpiration,
    ClientExpiration,

    Success,
}
