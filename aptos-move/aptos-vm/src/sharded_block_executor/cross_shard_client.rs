// Copyright © Aptos Foundation

use crate::{
    block_executor::BlockAptosExecutor,
    sharded_block_executor::{
        cross_shard_commit_listener::CrossShardCommitListener, messages::CrossShardMsg,
    },
};
use aptos_state_view::StateView;
use std::sync::{mpsc::Receiver, Arc};

pub struct CrossShardCommitReceiver {}

impl CrossShardCommitReceiver {
    pub fn start<S: StateView + Sync>(
        block_executor: Arc<BlockAptosExecutor<S, CrossShardCommitListener>>,
        message_rx: &Receiver<CrossShardMsg>,
    ) {
        loop {
            let msg = message_rx.recv().unwrap();
            match msg {
                CrossShardMsg::RemoteTxnCommitMsg(txn_commit_msg) => {
                    let (txn_index, txn_writes) = txn_commit_msg.take();
                    for (state_key, write_op) in txn_writes {
                        block_executor.add_txn_write(state_key, (txn_index, 0), write_op);
                    }
                    block_executor.mark_dependency_resolve(txn_index)
                },
                CrossShardMsg::StopMsg => {
                    break;
                },
            }
        }
    }
}

// pub struct CrossShardClient<S: StateView + Sync + Send> {
//     // The senders of cross-shard messages to other shards.
//     message_txs: Vec<Sender<CrossShardMsg>>,
//     // This thread is blocked on receiving messages from other shards.
//     receiver_thread: thread::JoinHandle<()>,
//
//     remote_commit_handler: Arc<RemoteTxnCommitHandler<S>>,
// }
//
// impl<S: StateView + Sync + Send> CrossShardClient<S> {
//     fn new(
//         message_rx: Receiver<CrossShardMsg>,
//         message_txs: Vec<Sender<CrossShardMsg>>,
//         remote_commit_handler: Arc<RemoteTxnCommitHandler<S>>,
//     ) -> Self {
//         let remote_commit_handler_clone = remote_commit_handler.clone();
//         let receiver_thread = thread::spawn(move || loop {
//             let msg = message_rx.recv().unwrap();
//             match msg {
//                 CrossShardMsg::RemoteTxnCommitMsg(msg) => {
//                     remote_commit_handler_clone.handle_remote_txn_commit(msg);
//                 },
//                 CrossShardMsg::StopMsg => {
//                     break;
//                 },
//             }
//         });
//
//         Self {
//             message_txs,
//             receiver_thread,
//             remote_commit_handler,
//         }
//     }
//
//     fn send_message(&self, msg: CrossShardMsg, shard_id: usize) {
//         self.message_txs[shard_id].send(msg).unwrap();
//     }
// }
//
// struct RemoteTxnCommitHandler<S: StateView + Sync + Send> {
//     block_executor: Weak<Mutex<Option<BlockAptosExecutor<'_, S>>>>,
// }
//
// impl<S: StateView + Sync + Send> RemoteTxnCommitHandler<S> {
//     fn new() -> Self {
//         Self {
//             block_executor: Arc::downgrade(&Arc::new(Mutex::new(None))),
//         }
//     }
// }
//
// impl<S: StateView + Sync + Send> RemoteTxnCommitHandler<S> {
//     fn handle_remote_txn_commit(&self, _msg: RemoteTxnCommit) {
//         //self.block_executor.commit_remote_txn(msg.txn_index(), msg.shard_id(), msg.txn_writes());
//     }
// }
