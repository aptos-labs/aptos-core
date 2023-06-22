// Copyright © Aptos Foundation

use crate::sharded_block_executor::{
    cross_shard_state_view::CrossShardStateView, messages::CrossShardMsg,
};
use aptos_state_view::StateView;
use aptos_types::write_set::TransactionWrite;
use std::sync::{mpsc::Receiver, Arc};

pub struct CrossShardCommitReceiver {}

impl CrossShardCommitReceiver {
    pub fn start<S: StateView + Sync + Send>(
        cross_shard_state_view: Arc<CrossShardStateView<S>>,
        message_rx: &Receiver<CrossShardMsg>,
    ) {
        loop {
            let msg = message_rx.recv().unwrap();
            match msg {
                CrossShardMsg::RemoteTxnWriteMsg(txn_commit_msg) => {
                    let (_, state_key, write_op) = txn_commit_msg.take();
                    cross_shard_state_view.set_value(&state_key, write_op.as_state_value());
                },
                CrossShardMsg::StopMsg => {
                    break;
                },
            }
        }
    }
}
