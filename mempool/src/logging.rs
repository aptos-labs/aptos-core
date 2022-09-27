// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::shared_mempool::types::{BatchId, QuorumStoreRequest};
use anyhow::Error;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_logger::Schema;
use aptos_types::{account_address::AccountAddress, on_chain_config::OnChainConfigPayload};
use mempool_notifications::MempoolCommitNotification;
use serde::Serialize;
use std::{fmt, fmt::Write, time::SystemTime};

pub struct TxnsLog {
    txns: Vec<(AccountAddress, u64, Option<String>, Option<SystemTime>)>,
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

    pub fn new_txn(account: AccountAddress, seq_num: u64) -> Self {
        Self {
            txns: vec![(account, seq_num, None, None)],
            len: 0,
            max_displayed: usize::MAX,
        }
    }

    pub fn add(&mut self, account: AccountAddress, seq_num: u64) {
        if self.txns.len() < self.max_displayed {
            self.txns.push((account, seq_num, None, None));
        }
        self.len += 1;
    }

    pub fn add_with_status(&mut self, account: AccountAddress, seq_num: u64, status: &str) {
        if self.txns.len() < self.max_displayed {
            self.txns
                .push((account, seq_num, Some(status.to_string()), None));
        }
        self.len += 1;
    }

    pub fn add_full_metadata(
        &mut self,
        account: AccountAddress,
        seq_num: u64,
        status: &str,
        timestamp: SystemTime,
    ) {
        if self.txns.len() < self.max_displayed {
            self.txns
                .push((account, seq_num, Some(status.to_string()), Some(timestamp)));
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

        for (account, seq_num, status, timestamp) in self.txns.iter() {
            let mut txn = format!("{}:{}", account, seq_num);
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
    reconfig_update: Option<OnChainConfigPayload>,
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
    batch_id: Option<&'a BatchId>,
    backpressure: Option<bool>,
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
            reconfig_update: None,
            account: None,
            txns: None,
            quorum_store_msg: None,
            state_sync_msg: None,
            network_level: None,
            upstream_network: None,
            batch_id: None,
            backpressure: None,
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
    InvariantViolated,
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
