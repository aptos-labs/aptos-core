// Copyright © Aptos Foundation
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{remote_state_view_service::RemoteStateViewService, ExecuteBlockCommand, RemoteExecutionRequest, RemoteExecutionResult, RemoteExecutionRequestRef, ExecuteBlockCommandRef};
use aptos_logger::{info, trace};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
    },
    state_store::StateView,
    transaction::TransactionOutput,
    vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{
    executor_client::{ExecutorClient, ShardedExecutionOutput},
    ShardedBlockExecutor,
};
use crossbeam_channel::{Receiver, Sender};
use once_cell::sync::{Lazy, OnceCell};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    thread,
};
use std::sync::atomic::AtomicU64;
use std::thread::{JoinHandle, sleep};
use std::time::{Duration, SystemTime};
use itertools::Itertools;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::{get_delta_time, REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_vm::sharded_block_executor::sharded_executor_service::{CmdsAndMetaDataRef, TransactionIdxAndOutput};
use crate::metrics::REMOTE_EXECUTOR_TIMER;

pub static COORDINATOR_PORT: u16 = 52200;

static REMOTE_ADDRESSES: OnceCell<Vec<SocketAddr>> = OnceCell::new();
static COORDINATOR_ADDRESS: OnceCell<SocketAddr> = OnceCell::new();

pub fn set_remote_addresses(addresses: Vec<SocketAddr>) {
    REMOTE_ADDRESSES.set(addresses).ok();
}

pub fn get_remote_addresses() -> Vec<SocketAddr> {
    match REMOTE_ADDRESSES.get() {
        Some(value) => value.clone(),
        None => vec![],
    }
}

pub fn set_coordinator_address(address: SocketAddr) {
    COORDINATOR_ADDRESS.set(address).ok();
}

pub fn get_coordinator_address() -> SocketAddr {
    match COORDINATOR_ADDRESS.get() {
        Some(value) => *value,
        None => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), COORDINATOR_PORT),
    }
}

pub static REMOTE_SHARDED_BLOCK_EXECUTOR: Lazy<
    Arc<
        aptos_infallible::Mutex<
            ShardedBlockExecutor<CachedStateView, RemoteExecutorClient<CachedStateView>>,
        >,
    >,
> = Lazy::new(|| {
    info!("REMOTE_SHARDED_BLOCK_EXECUTOR created");
    Arc::new(aptos_infallible::Mutex::new(
        RemoteExecutorClient::create_remote_sharded_block_executor(
            get_coordinator_address(),
            get_remote_addresses(),
            None,
        ),
    ))
});

#[allow(dead_code)]
pub struct RemoteExecutorClient<S: StateView + Sync + Send + 'static> {
    // The network controller used to create channels to send and receive messages. We want the
    // network controller to be owned by the executor client so that it is alive for the entire
    // lifetime of the executor client.
    network_controller: NetworkController,
    state_view_service: Arc<RemoteStateViewService<S>>,
    // Channels to send execute block commands to the executor shards.
    command_txs: Arc<Vec<Vec<Mutex<OutboundRpcHelper>>>>,
    // Channels to receive execution results from the executor shards.
    result_rxs: Vec<Arc<Receiver<Message>>>,
    // Thread pool used to pre-fetch the state values for the block in parallel and create an in-memory state view.
    thread_pool: Arc<rayon::ThreadPool>,
    cmd_tx_thread_pool: Arc<rayon::ThreadPool>,

    phantom: std::marker::PhantomData<S>,
    _join_handle: Option<thread::JoinHandle<()>>,
    kv_finished: Receiver<Message>,
}

#[allow(dead_code)]
impl<S: StateView + Sync + Send + 'static> RemoteExecutorClient<S> {
    pub fn new(
        remote_shard_addresses: Vec<SocketAddr>,
        mut controller: NetworkController,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = num_threads.unwrap_or_else(num_cpus::get);
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(24)
                .build()
                .unwrap(),
        );
        let outbound_rpc_runtime = controller.get_outbound_rpc_runtime();
        let self_addr = controller.get_self_addr();
        let controller_mut_ref = &mut controller;
        let num_shards = remote_shard_addresses.len();
        let mut command_txs= vec![];
        let mut result_rxs = vec![];
        remote_shard_addresses
        .iter()
        .enumerate()
        .for_each(|(shard_id, address)| {
            let execute_command_type = format!("execute_command_{}", shard_id);
            let execute_result_type = format!("execute_result_{}", shard_id);
            let mut command_tx = vec![];
            for _ in 0..std::cmp::max(1,num_threads/(2 * num_shards)) {
                command_tx.push(Mutex::new(OutboundRpcHelper::new(self_addr, *address, outbound_rpc_runtime.clone())));
            }
            let result_rx = Arc::new(controller_mut_ref.create_inbound_channel(execute_result_type));
            command_txs.push(command_tx);
            result_rxs.push(result_rx);
        });

        let state_view_service = Arc::new(RemoteStateViewService::new(
            controller_mut_ref,
            remote_shard_addresses,
            None,
        ));

        let state_view_service_clone = state_view_service.clone();

        let join_handle = thread::Builder::new()
            .name("remote-state_view-service".to_string())
            .spawn(move || state_view_service_clone.start())
            .unwrap();

        controller.start();

        let cmd_tx_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index| format!("rmt-exe-cli-cmd-tx-{}", index))
                .num_threads(24)//num_cpus::get() / 2)
                .build()
                .unwrap(),
        );

        let kv_finished = controller.create_inbound_channel("kv_finished".to_string());

        Self {
            network_controller: controller,
            state_view_service,
            _join_handle: Some(join_handle),
            command_txs: Arc::new(command_txs),
            result_rxs,
            thread_pool,
            cmd_tx_thread_pool,
            phantom: std::marker::PhantomData,
            kv_finished,
        }
    }

    pub fn create_remote_sharded_block_executor(
        coordinator_address: SocketAddr,
        remote_shard_addresses: Vec<SocketAddr>,
        num_threads: Option<usize>,
    ) -> ShardedBlockExecutor<S, RemoteExecutorClient<S>> {
        ShardedBlockExecutor::new(RemoteExecutorClient::new(
            remote_shard_addresses,
            NetworkController::new(
                "remote-executor-coordinator".to_string(),
                coordinator_address,
                5000,
            ),
            num_threads,
        ))
    }

    fn get_output_from_shards(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
        trace!("RemoteExecutorClient Waiting for results");
        /*let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(self.num_shards())
                .build()
                .unwrap(),
        );

        let mut results = vec![];
        for rx in self.result_rxs.iter() {
            let received_bytes = rx.recv().unwrap().to_bytes();
            let result: RemoteExecutionResult = bcs::from_bytes(&received_bytes).unwrap();
            results.push(result.inner?);
        }*/

        let results: Vec<(usize, Vec<Vec<TransactionOutput>>)> = (0..self.num_shards()).into_par_iter().map(|shard_id| {
            let received_msg = self.result_rxs[shard_id].recv().unwrap();
            let delta = get_delta_time(received_msg.start_ms_since_epoch.unwrap());
            REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
                .with_label_values(&["9_1_results_tx_msg_remote_exe_recv"]).observe(delta as f64);

            let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "result_rx_bcs_deser"])
                .start_timer();
            let result: RemoteExecutionResult = bcs::from_bytes(&received_msg.to_bytes()).unwrap();
            drop(bcs_deser_timer);
            (shard_id, result.inner.unwrap())
        }).collect();

        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "result_rx_gather"])
            .start_timer();
        let mut res: Vec<Vec<Vec<TransactionOutput>>> = vec![vec![]; self.num_shards()];
        for (shard_id, result) in results.into_iter() {
            res[shard_id] = result;
        }
        Ok(res)
    }

    fn get_streamed_output_from_shards(&self, expected_outputs: Vec<u64>, duration_since_epoch: u64) -> Result<Vec<TransactionOutput>, VMStatus> {
        //info!("expected outputs {:?} ", expected_outputs);
        let (send_outputs, recv_outputs) = crossbeam_channel::unbounded();
        let mut results: Vec<Vec<TransactionIdxAndOutput>> = Vec::with_capacity(self.num_shards());
        for i in 0..self.num_shards() {
            results.push(vec![]);
        }
        (0..self.num_shards()).into_iter().for_each(|shard_id| {
            let send_outputs_clone = send_outputs.clone();
            let expected_outputs_clone = expected_outputs.clone();
            let result_rxs_clone = self.result_rxs[shard_id].clone();
            info!("Waiting for results from shard {}", shard_id);
            self.thread_pool.spawn(move || {
                let received_msg = result_rxs_clone.recv().unwrap();
                info!("Testing network finished on shard {} with avg_delta: {}, max_delta: {}",
                    received_msg.shard_id.unwrap(),
                    received_msg.start_ms_since_epoch.unwrap(),
                    received_msg.seq_num.unwrap()
                );
                send_outputs_clone.send(()).unwrap();
            });
        });
        let mut cnt = 0;
        while let Ok(msg) = recv_outputs.recv() {
            //results[msg.0] = msg.1;
            cnt += 1;
            if cnt == self.num_shards() {
                break;
            }
        }

        let delta = get_delta_time(duration_since_epoch);
        REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
            .with_label_values(&["9_2_results_rx_all_shards"]).observe(delta as f64);

        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "result_rx_gather"])
            .start_timer();
        let mut aggregated_results: Vec<TransactionOutput> = vec![Default::default() ; expected_outputs.iter().sum::<u64>() as usize];
        results.into_iter().for_each(|result| {
            result.into_iter().for_each(|txn_output| {
                aggregated_results[txn_output.txn_idx as usize] = txn_output.txn_output;
            });
        });

        Ok(aggregated_results)
    }
}

impl<S: StateView + Sync + Send + 'static> ExecutorClient<S> for RemoteExecutorClient<S> {
    fn num_shards(&self) -> usize {
        self.command_txs.len()
    }

    fn execute_block(&self, state_view: Arc<S>, transactions: PartitionedTransactions, concurrency_level_per_shard: usize, onchain_config: BlockExecutorConfigFromOnchain) -> Result<ShardedExecutionOutput, VMStatus> {
        panic!("Not implemented for RemoteExecutorClient");
    }

    fn execute_block_remote(
        &self,
        state_view: Arc<S>,
        transactions: Arc<PartitionedTransactions>,
        concurrency_level_per_shard: usize,
        onchain_config: BlockExecutorConfigFromOnchain,
        duration_since_epoch: u64
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let mut rng = StdRng::from_entropy();
        loop {
            info!("Starting next block");
            for shard_id in 0..self.num_shards() {
                let rand_send_thread_idx = rng.gen_range(0, self.command_txs[shard_id].len());
                let command_txs_clone = self.command_txs.clone();
                self.cmd_tx_thread_pool.spawn(move || {
                    command_txs_clone[shard_id][rand_send_thread_idx]
                        .lock()
                        .unwrap()
                        .send(Message::new(vec![]), &MessageType::new("cmd_completed".to_string()));
                });
            }

            // let mut shard_with_kv_completed = 0;
            // while let Ok(msg) = self.kv_finished.recv() {
            //     shard_with_kv_completed += 1;
            //     if shard_with_kv_completed == self.num_shards() {
            //         break;
            //     }
            // }

            let results = self.get_streamed_output_from_shards(vec![], duration_since_epoch);
            sleep(Duration::from_millis(200));
        }
        let timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "drop_state_view_finally"])
            .start_timer();
        self.state_view_service.drop_state_view();
        drop(timer);
        REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
            .with_label_values(&["9_8_execute_remote_block_done"]).observe(get_delta_time(duration_since_epoch) as f64);
        //drop(transactions);
        DEFAULT_DROPPER.schedule_drop(transactions);
        Ok(vec![])
        //Ok(ShardedExecutionOutput::new(vec![], vec![]))
    }

    fn shutdown(&mut self) {
        self.network_controller.shutdown();
    }
}
