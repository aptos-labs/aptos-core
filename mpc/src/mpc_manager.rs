// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::ops::Mul;
use std::sync::Arc;
use std::time::Duration;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use anyhow::{anyhow, bail, ensure, Result};
use futures_util::future::{Abortable, AbortHandle};
use rand::{Rng, thread_rng};
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_crypto::HashValue;
use aptos_logger::{debug, error, info};
use aptos_types::epoch_state::EpochState;
use aptos_types::mpc::{MPCEvent, MPCEventMoveStruct, MPCEventReconfigStart, MPCEventStateUpdated, MPCState, ReconfigWorkResult, TaskResult, TaskSpec, TaskState};
use aptos_types::validator_txn::{Topic, ValidatorTransaction};
use aptos_types::validator_verifier::ValidatorConsensusInfo;
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use move_core_types::account_address::AccountAddress;
use crate::network::IncomingRpcRequest;

/// Represent an in-progress MPC task.
/// If dropped, the task is cancelled.
struct TaskGuard {
    abort_handle: AbortHandle,
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.abort_handle.abort()
    }
}

enum LocalTaskState {
    Started {
        task_guard: TaskGuard
    },
    Finished {
        vtxn_guard: TxnGuard,
    },
}

pub struct MPCManager {
    my_index: usize,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    vtxn_pool: VTxnPoolState,
    stopped: bool,
    cached_on_chain_state: Option<MPCState>,
    task_states: HashMap<usize, LocalTaskState>,
    reconfig_work_state: Option<LocalTaskState>,
    task_completion_tx: Option<aptos_channel::Sender<(), TaskResult>>,
    reconfig_work_completion_tx: Option<aptos_channel::Sender<(), ReconfigWorkResult>>,
}

impl MPCManager {
    pub fn new(
        my_index: usize,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        Self {
            my_addr,
            my_index,
            epoch_state,
            vtxn_pool,
            stopped: false,
            cached_on_chain_state: None,
            task_states: HashMap::new(),
            reconfig_work_state: None,
            task_completion_tx: None,
            reconfig_work_completion_tx: None,
        }
    }

    fn process_task_completion(&mut self, result: TaskResult) -> Result<()> {
        let task_idx = result.task_idx;
        let task_state = self.task_states.get_mut(&task_idx);
        ensure!(task_state.is_some());
        let task_state = task_state.unwrap();
        ensure!(matches!(task_state, LocalTaskState::Started { .. }));
        let txn = ValidatorTransaction::MPCUserRequestDone(result);
        let vtxn_guard = self.vtxn_pool.put(Topic::MPC_USER_REQUEST(task_idx), Arc::new(txn), None);
        *task_state = LocalTaskState::Finished { vtxn_guard };
        Ok(())
    }

    fn process_reconfig_work_completion(&mut self, result: ReconfigWorkResult) -> Result<()> {
        let reconfig_work_state = self.reconfig_work_state.as_mut();
        ensure!(reconfig_work_state.is_some());
        let reconfig_work_state = reconfig_work_state.unwrap();
        ensure!(matches!(reconfig_work_state, LocalTaskState::Started { .. }));
        let txn = ValidatorTransaction::MPCReconfigWorkDone(result);
        let vtxn_guard = self.vtxn_pool.put(Topic::MPC_RECONFIG, Arc::new(txn), None);
        *reconfig_work_state = LocalTaskState::Finished { vtxn_guard };
        Ok(())
    }

    fn start_task(&self, task_idx: usize, spec: TaskSpec) -> TaskGuard {
        debug!(epoch = self.epoch_state.epoch, "start_task: begin, spec={:?}", spec);

        let task_completion_tx = self.task_completion_tx.clone().unwrap();
        let epoch = self.epoch_state.epoch;
        let TaskSpec { group_element, secret_idx } = spec;
        let group_element_bytes = <[u8; 48]>::try_from(group_element).unwrap();
        let group_element = blstrs::G1Affine::from_compressed(&group_element_bytes).unwrap();
        let secret_bytes = self.cached_on_chain_state.as_ref().unwrap().shared_secrets[secret_idx as usize].transcript_for_cur_epoch.clone().unwrap();
        let secret_bytes = <[u8; 32]>::try_from(secret_bytes).unwrap();
        let secret = blstrs::Scalar::from_bytes_be(&secret_bytes).unwrap();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(100)).await; //mpc todo: real work
            let raise_result = group_element.mul(&secret).to_compressed().to_vec();
            let task_result = TaskResult {
                task_idx,
                raise_result,
            };
            if let Err(e) = task_completion_tx.push((), task_result) {
                info!(
                    epoch = epoch,
                    "[MPC] Failed to start_task, maybe MPCManager stopped and channel dropped: {:?}", e
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        debug!(epoch = self.epoch_state.epoch, "start_task: end");
        TaskGuard { abort_handle }
    }

    fn start_reconfig_work(&self, next_validator_set: Vec<ValidatorConsensusInfo>) -> TaskGuard {
        debug!(epoch = self.epoch_state.epoch, "start_reconfig_work: begin, next_validator_set={:?}", next_validator_set);
        let reconfig_work_completion_tx = self.reconfig_work_completion_tx.clone().unwrap();
        let cur_transcript = self.cached_on_chain_state.as_ref().unwrap().shared_secrets.get(0).map(|st|st.transcript_for_cur_epoch.clone()).unwrap_or_default();
        let epoch = self.epoch_state.epoch;
        let task = async move {
            tokio::time::sleep(Duration::from_millis(200)).await; //mpc todo: real work
            let next_transcript = cur_transcript.unwrap_or_else(||thread_rng().gen::<[u8; 32]>().to_vec());
            let reconfig_work_result = ReconfigWorkResult {
                next_transcript,
            };
            if let Err(e) = reconfig_work_completion_tx.push((), reconfig_work_result) {
                info!(
                    epoch = epoch,
                    "[MPC] Failed to start_task, maybe MPCManager stopped and channel dropped: {:?}", e
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        debug!(epoch = self.epoch_state.epoch, "start_reconfig_work: end");
        TaskGuard { abort_handle }
    }

    fn intake_new_state(&mut self, mpc_state: MPCState) {
        debug!(epoch = self.epoch_state.epoch, "intake_new_state: begin");
        let MPCState { shared_secrets, tasks } = mpc_state;
        for (idx, onchain_task_state) in tasks.into_iter().enumerate() {
            debug!(epoch = self.epoch_state.epoch, idx = idx, "intake_new_state: processing on-chain task");
            let TaskState { task, result } = onchain_task_state;
            if result.is_some() {
                debug!(epoch = self.epoch_state.epoch, idx = idx, "intake_new_state: result available on-chain!");
                // For fulfilled tasks, cancel local session if it exists.
                self.task_states.remove(&idx);
            } else if !self.task_states.contains_key(&idx) {
                debug!(epoch = self.epoch_state.epoch, idx = idx, "intake_new_state: new task!");
                // For any new tasks, trigger the protocol.
                let task_guard = self.start_task(idx, task);
                self.task_states.insert(idx, LocalTaskState::Started { task_guard });
            }
        }

        debug!(epoch = self.epoch_state.epoch, "intake_new_state: checkpoint");

        if let Some(main_secret_state) = shared_secrets.get(0) {
            debug!(epoch = self.epoch_state.epoch, "intake_new_state: main secret exists");
            if main_secret_state.transcript_for_next_epoch.is_some() {
                debug!(epoch = self.epoch_state.epoch, "intake_new_state: next trx of main secret exists");
                // for new trx_for_next_epoch, cancel exiting dkg/resharing.
                self.reconfig_work_state = None;
            }
        }
        debug!(epoch = self.epoch_state.epoch, "intake_new_state: end");
    }

    pub async fn run(
        mut self,
        mpc_state: MPCState,
        mut mpc_event_rx: aptos_channel::Receiver<(), MPCEventMoveStruct>,
        mut rpc_msg_rx: aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>
    ) {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[MPC] MPCManager started."
        );

        self.intake_new_state(mpc_state);
        let (task_completion_tx, mut task_completion_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.task_completion_tx = Some(task_completion_tx);
        let (reconfig_work_completion_tx, mut reconfig_work_completion_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.reconfig_work_completion_tx = Some(reconfig_work_completion_tx);

        let mut close_rx = close_rx.into_stream();
        while !self.stopped {
            let handling_result = tokio::select! {
                task_completion = task_completion_rx.select_next_some() => {
                    self.process_task_completion(task_completion).map_err(|e|anyhow!("[MPC] process_task_completion failed: {e}"))
                },
                reconfig_work_completion = reconfig_work_completion_rx.select_next_some() => {
                    self.process_reconfig_work_completion(reconfig_work_completion).map_err(|e|anyhow!("[MPC] process_reconfig_work_completion failed: {e}"))
                },
                mpc_event = mpc_event_rx.select_next_some() => {
                    self.process_mpc_event(mpc_event).await.map_err(|e|anyhow!("[MPC] process_mpc_event failed: {e}"))
                },
                close_req = close_rx.select_next_some() => {
                    self.process_close_cmd(close_req.ok())
                },
            };

            if let Err(e) = handling_result {
                error!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr.to_hex().as_str(),
                    "[MPC] MPCManager handling error: {e}"
                );
            }
        }


        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[MPC] MPCManager finished."
        );
    }

    async fn process_mpc_event(&mut self, event: MPCEventMoveStruct) -> Result<()> {
        let event = MPCEvent::try_from(event);
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[MPC] Processing MPC event: {:?}",
            event
        );
        match event {
            Ok(MPCEvent::ReconfigStart(event)) => {
                let MPCEventReconfigStart { epoch, next_validator_set } = event;
                //mpc todo: real processing.
                let task_guard = self.start_reconfig_work(next_validator_set);
                self.reconfig_work_state = Some(LocalTaskState::Started { task_guard });
                Ok(())
            },
            Ok(MPCEvent::StateUpdated(e)) => {
                let MPCEventStateUpdated { epoch, new_state } = e;
                self.intake_new_state(new_state);
                Ok(())
            },
            Err(e) => {
                Err(anyhow!("process_mpc_event failed with casting error: {e}"))
            },
        }
    }

    fn process_close_cmd(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;
        if let Some(tx) = ack_tx {
            let _ = tx.send(());
        }

        Ok(())
    }
}
