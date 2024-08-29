// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    metrics::REMOTE_EXECUTOR_TIMER, remote_state_view::RemoteStateViewClient, ExecuteBlockCommand,
    RemoteExecutionRequest, RemoteExecutionResult,
};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use aptos_types::{
    block_executor::partitioner::ShardId, state_store::state_key::StateKey,
    transaction::TransactionOutput, vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{coordinator_client::CoordinatorClient, ExecuteV3PartitionStreamedInitCommand, ExecutorShardCommand};
use crossbeam_channel::{Receiver, Sender, unbounded};
use rayon::prelude::*;
use std::{net::SocketAddr, sync::Arc, thread};
use std::ops::AddAssign;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::sync::Mutex;
use aptos_block_executor::txn_provider::sharded::{BlockingTransaction, CrossShardClientForV3, ShardedTransaction, ShardedTxnProvider};
use aptos_logger::info;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::{get_delta_time, REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
use aptos_vm::sharded_block_executor::sharded_executor_service::{CmdsAndMetaData, OutputStreamHookImpl, TransactionIdxAndOutput, V3CmdsOrMetaData, V3CmdsOrMetaDataRef, V3MetaData, V3MetaDataRef};
use aptos_vm::sharded_block_executor::streamed_transactions_provider::BlockingTransactionsProvider;
use crate::remote_cross_shard_client::{RemoteCrossShardClient, RemoteCrossShardClientV3};

pub struct RemoteCoordinatorClient {
    state_view_client: Arc<RemoteStateViewClient>,
    command_rx: Arc<Receiver<Message>>,
    //result_tx: Sender<Message>,
    result_tx: OutboundRpcHelper,
    shard_id: ShardId,
    num_shards: usize,
    cmd_rx_msg_duration_since_epoch: Arc<AtomicU64>,
    is_block_init_done: Arc<AtomicBool>,//Mutex<bool>,
    cmd_rx_thread_pool: Arc<rayon::ThreadPool>,
    remote_cross_shard_client: Arc<RemoteCrossShardClientV3>,
}

impl RemoteCoordinatorClient {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        controller: &mut NetworkController,
        coordinator_address: SocketAddr,
        remote_cross_shard_client: Arc<RemoteCrossShardClientV3>,
    ) -> Self {
        let execute_command_type = format!("execute_command_{}", shard_id);
        let execute_result_type = format!("execute_result_{}", shard_id);
        let command_rx = controller.create_inbound_channel(execute_command_type);
        let result_tx = OutboundRpcHelper::new(controller.get_self_addr(), coordinator_address, controller.get_outbound_rpc_runtime());
            //controller.create_outbound_channel(coordinator_address, execute_result_type);
        let cmd_rx_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index| format!("remote-state-view-shard-send-request-{}-{}", shard_id, index))
                .num_threads(8)
                .build()
                .unwrap(),
        );

        let state_view_client =
            RemoteStateViewClient::new(shard_id, controller, coordinator_address);

        Self {
            state_view_client: Arc::new(state_view_client),
            command_rx: Arc::new(command_rx),
            result_tx,
            shard_id,
            num_shards,
            cmd_rx_msg_duration_since_epoch: Arc::new(AtomicU64::new(0)),
            is_block_init_done: Arc::new(AtomicBool::new(false)),
            cmd_rx_thread_pool,
            remote_cross_shard_client,
        }
    }

    // Extract all the state keys from the execute block command. It is possible that there are duplicate state keys.
    // We are not de-duplicating them here to avoid the overhead of deduplication. The state view server will deduplicate
    // the state keys.
    fn extract_state_keys(command: &ExecuteBlockCommand) -> Vec<StateKey> {
        command
            .sub_blocks
            .sub_block_iter()
            .flat_map(|sub_block| {
                sub_block
                    .transactions
                    .iter()
                    .map(|txn| {
                        let mut state_keys = vec![];
                        for storage_location in txn
                            .txn()
                            .read_hints()
                            .iter()
                            .chain(txn.txn().write_hints().iter())
                        {
                            state_keys.push(storage_location.state_key().clone());
                        }
                        state_keys
                    })
                    .flatten()
                    .collect::<Vec<StateKey>>()
            })
            .collect::<Vec<StateKey>>()
    }

    fn extract_state_keys_from_txns(txns: &Vec<AnalyzedTransaction>) -> Vec<StateKey> {
        let mut state_keys = vec![];
        for txn in txns {
            for storage_location in txn
                .read_hints()
                .iter()
                .chain(txn.write_hints().iter())
            {
                state_keys.push(storage_location.state_key().clone());
            }
        }
        state_keys
    }

    fn receive_execute_command_stream_follow_up(state_view_client: Arc<RemoteStateViewClient>,
                                                command_rx: Arc<Receiver<Message>>,
                                                blocking_transactions: Arc<Vec<ShardedTransaction<SignatureVerifiedTransaction>>>,
                                                num_txns_in_the_block: usize,
                                                num_shards: usize,
                                                shard_id: ShardId,
                                                cross_shard_client: Arc<RemoteCrossShardClientV3>,
                                                cmd_rx_msg_duration_since_epoch: Arc<AtomicU64>,
                                                is_block_init_done: Arc<AtomicBool>,
                                                cmd_rx_thread_pool: Arc<rayon::ThreadPool>,
                                                init_sender: Option<Sender<ExecutorShardCommand<RemoteStateViewClient>>>,) {
        let mut num_txns_processed = 0;
        let mut all_cmds_recvd = false;
        let mut stream_init_done = init_sender.is_none();
        let blocking_transactions_clone = blocking_transactions.clone();

        loop {
            if stream_init_done && all_cmds_recvd {
                //info!("Breaking out of the loop.............................");
                break;
            }
            match command_rx.recv() {
                Ok(message) => {
                    let state_view_client_clone = state_view_client.clone();
                   // let blocking_transactions_provider_clone = blocking_transactions_provider.clone();
                    let cmd_rx_msg_duration_since_epoch_clone = cmd_rx_msg_duration_since_epoch.clone();
                    let is_block_init_done_clone = is_block_init_done.clone();
                    let cmd_rx_thread_pool_clone = cmd_rx_thread_pool.clone();

                    let delta = get_delta_time(message.start_ms_since_epoch.unwrap());
                    REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
                        .with_label_values(&["5_cmd_tx_msg_shard_recv"]).observe(delta as f64);
                    cmd_rx_msg_duration_since_epoch_clone.store(message.start_ms_since_epoch.unwrap(), std::sync::atomic::Ordering::Relaxed);
                    let _rx_timer = REMOTE_EXECUTOR_TIMER
                        .with_label_values(&[&shard_id.to_string(), "cmd_rx"])
                        .start_timer();
                    let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
                        .with_label_values(&[&shard_id.to_string(), "cmd_rx_bcs_deser"])
                        .start_timer();
                    let cmds_or_metadata: V3CmdsOrMetaData = bcs::from_bytes(&message.data).unwrap();
                    drop(bcs_deser_timer);

                    match cmds_or_metadata {
                        V3CmdsOrMetaData::MetaData(meta_data) => {
                            match init_sender.as_ref() {
                                None => {
                                    panic!("We got more than one V3CmdsOrMetaData::MetaData message!!!!");
                                }
                                Some(sender) => {
                                    let dummy: [u8; 32] = [0; 32];
                                    let (stream_results_tx, stream_results_rx) = unbounded();
                                    let output_stream_hook = OutputStreamHookImpl {
                                        stream_results_tx,
                                    };
                                    let txn_provider = ShardedTxnProvider::new(
                                        dummy,
                                        num_shards,
                                        shard_id,
                                        cross_shard_client.clone(),
                                        blocking_transactions_clone.clone(),
                                        meta_data.global_idxs,
                                        meta_data.local_idx_by_global,
                                        meta_data.key_sets_by_dep,
                                        meta_data.follower_shard_sets,
                                        Some(output_stream_hook),
                                    );
                                    let _ = sender.send(ExecutorShardCommand::ExecuteV3PartitionStreamedInit(
                                        ExecuteV3PartitionStreamedInitCommand {
                                            state_view: state_view_client_clone,
                                            blocking_transactions_provider: txn_provider,
                                            stream_results_receiver: stream_results_rx,
                                            num_txns: meta_data.num_txns,
                                            onchain_config: meta_data.onchain_config,
                                        }
                                    ));
                                    stream_init_done = true;
                                }
                            }
                        }
                        V3CmdsOrMetaData::Cmds(cmds) => {
                            let transactions = cmds.cmds;
                            num_txns_processed += transactions.len();
                            info!("txns considered is ********* {}; num txns in block {}", num_txns_processed, num_txns_in_the_block);
                            if num_txns_processed == num_txns_in_the_block {
                                is_block_init_done_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                                all_cmds_recvd = true;
                            }

                            let init_prefetch_timer = REMOTE_EXECUTOR_TIMER
                                .with_label_values(&[&shard_id.to_string(), "init_prefetch"])
                                .start_timer();
                            let blocking_transactions_clone2 = blocking_transactions.clone();
                            cmd_rx_thread_pool_clone.spawn(move || {


                                let batch_start_index = cmds.batch_start_index;
                                let state_keys = Self::extract_state_keys_from_txns(&transactions);

                                state_view_client_clone.pre_fetch_state_values(state_keys, false);

                                let _ = transactions.into_iter().enumerate().for_each(|(idx, txn)| {
                                    blocking_transactions_clone2[idx + batch_start_index].set_txn(txn.into_txn());
                                });
                            });
                        }
                    }
                },
                Err(_) => { break; }
            }
        }
    }
}

impl CoordinatorClient<RemoteStateViewClient> for RemoteCoordinatorClient {
    fn receive_execute_command(&self) -> ExecutorShardCommand<RemoteStateViewClient> {
        match self.command_rx.recv() {
            Ok(message) => {
                let delta = get_delta_time(message.start_ms_since_epoch.unwrap());
                REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
                    .with_label_values(&["5_cmd_tx_msg_shard_recv"]).observe(delta as f64);
                self.cmd_rx_msg_duration_since_epoch.store(message.start_ms_since_epoch.unwrap(), std::sync::atomic::Ordering::Relaxed);
                let _rx_timer = REMOTE_EXECUTOR_TIMER
                    .with_label_values(&[&self.shard_id.to_string(), "cmd_rx"])
                    .start_timer();
                let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
                    .with_label_values(&[&self.shard_id.to_string(), "cmd_rx_bcs_deser"])
                    .start_timer();
                let cmds_or_metadata: V3CmdsOrMetaData = bcs::from_bytes(&message.data).unwrap();
                drop(bcs_deser_timer);

                let cmd_rx_msg_duration_since_epoch_clone = self.cmd_rx_msg_duration_since_epoch.clone();
                let state_view_client_clone = self.state_view_client.clone();
                let is_block_init_done_clone = self.is_block_init_done.clone();
                let cmd_rx_thread_pool_clone = self.cmd_rx_thread_pool.clone();
                let command_rx = self.command_rx.clone();
                self.state_view_client.init_for_block();
                let num_shards = self.num_shards;
                let shard_id = self.shard_id;
                let remote_cross_shard_client = self.remote_cross_shard_client.clone();
                return match cmds_or_metadata {
                    V3CmdsOrMetaData::MetaData(meta_data) => {
                        let dummy: [u8; 32] = [0; 32];
                        //let blocking_txns = Arc::new(vec![ShardedTransaction::BlockingTxn(BlockingTransaction::new()); meta_data.num_txns]);
                        let mut blocking_txns = Vec::new();
                        for _ in 0..meta_data.num_txns {
                            blocking_txns.push(ShardedTransaction::BlockingTxn(BlockingTransaction::new()));
                        }
                        let blocking_txns_arc = Arc::new(blocking_txns);
                        let (stream_results_tx, stream_results_rx) = unbounded();
                        let output_stream_hook = OutputStreamHookImpl {
                            stream_results_tx,
                        };
                        let txn_provider = ShardedTxnProvider::new(
                            dummy,
                            num_shards,
                            shard_id,
                            remote_cross_shard_client.clone(),
                            blocking_txns_arc.clone(),
                            meta_data.global_idxs,
                            meta_data.local_idx_by_global,
                            meta_data.key_sets_by_dep,
                            meta_data.follower_shard_sets,
                            Some(output_stream_hook),
                        );
                        self.cmd_rx_thread_pool.spawn(move || {
                            Self::receive_execute_command_stream_follow_up(
                                state_view_client_clone,
                                command_rx,
                                blocking_txns_arc,
                                meta_data.num_txns,
                                num_shards,
                                shard_id,
                                remote_cross_shard_client.clone(),
                                cmd_rx_msg_duration_since_epoch_clone,
                                is_block_init_done_clone,
                                cmd_rx_thread_pool_clone,
                                None);
                        });
                        ExecutorShardCommand::ExecuteV3PartitionStreamedInit(
                            ExecuteV3PartitionStreamedInitCommand {
                                state_view: self.state_view_client.clone(),
                                blocking_transactions_provider: txn_provider,
                                stream_results_receiver: stream_results_rx,
                                num_txns: meta_data.num_txns,
                                onchain_config: meta_data.onchain_config,
                            }
                        )
                    }
                    V3CmdsOrMetaData::Cmds(cmds) => {
                        let (sender, receiver) = crossbeam_channel::unbounded();
                        let mut blocking_txns = Vec::new();
                        for _ in 0..cmds.num_txns_total {
                            blocking_txns.push(ShardedTransaction::BlockingTxn(BlockingTransaction::new()));
                        }
                        self.cmd_rx_thread_pool.spawn(move || {
                            Self::receive_execute_command_stream_follow_up(
                                state_view_client_clone,
                                command_rx,
                                Arc::new(blocking_txns),
                                cmds.num_txns_total,
                                num_shards,
                                shard_id,
                                remote_cross_shard_client.clone(),
                                cmd_rx_msg_duration_since_epoch_clone,
                                is_block_init_done_clone,
                                cmd_rx_thread_pool_clone,
                                Some(sender));
                        });
                        receiver.recv().unwrap()
                    }
                };

            },
            Err(_) => ExecutorShardCommand::Stop,
        }
    }

    fn reset_block_init(&self) {
        self.is_block_init_done.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    fn send_execution_result(&mut self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        let execute_result_type = format!("execute_result_{}", self.shard_id);
        let duration_since_epoch = self.cmd_rx_msg_duration_since_epoch.load(std::sync::atomic::Ordering::Relaxed);
        let remote_execution_result = RemoteExecutionResult::new(result);
        let bcs_ser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&[&self.shard_id.to_string(), "result_tx_bcs_ser"])
            .start_timer();
        let output_message = bcs::to_bytes(&remote_execution_result).unwrap();
        drop(bcs_ser_timer);
        let delta = get_delta_time(duration_since_epoch);
        REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
            .with_label_values(&["6_results_tx_msg_shard_send"]).observe(delta as f64);
        self.result_tx.send(Message::create_with_metadata(output_message, duration_since_epoch, 0, 0), &MessageType::new(execute_result_type));
    }

    fn stream_execution_result(&mut self, txn_idx_output: Vec<TransactionIdxAndOutput>) {
        //info!("Sending output to coordinator for txn_idx: {:?}", txn_idx_output.txn_idx);
        let execute_result_type = format!("execute_result_{}", self.shard_id);
        let output_message = bcs::to_bytes(&txn_idx_output).unwrap();
        self.result_tx.send(Message::new(output_message), &MessageType::new(execute_result_type));
    }

    fn record_execution_complete_time_on_shard(&self) {
        let duration_since_epoch = self.cmd_rx_msg_duration_since_epoch.load(std::sync::atomic::Ordering::Relaxed);
        let delta = get_delta_time(duration_since_epoch);
        REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
            .with_label_values(&["6_exe_complete_on_shard"]).observe(delta as f64);
    }
}
