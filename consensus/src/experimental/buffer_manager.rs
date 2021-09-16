// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use futures::{
    channel::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    executor::block_on,
    SinkExt, StreamExt,
};
use tokio::time::Duration;

use consensus_types::{common::Author, executed_block::ExecutedBlock};
use diem_crypto::ed25519::Ed25519Signature;
use diem_logger::prelude::*;
use diem_types::{
    account_address::AccountAddress,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};

use crate::{
    counters,
    experimental::{
        buffer_item::BufferItem,
        execution_phase::{ExecutionRequest, ExecutionResponse},
        linkedlist::{get_elem, get_elem_mut, get_next, link_eq, set_elem, take_elem, Link, List},
        persisting_phase::PersistingRequest,
        signing_phase::{SigningRequest, SigningResponse},
    },
    network::NetworkSender,
    network_interface::ConsensusMsg,
    round_manager::VerifiedEvent,
    state_replication::StateComputerCommitCallBackType,
};

pub const BUFFER_MANAGER_RETRY_INTERVAL: u64 = 1000;

pub type SyncAck = ();

pub fn sync_ack_new() -> SyncAck {}

pub struct SyncRequest {
    tx: oneshot::Sender<SyncAck>,
    ledger_info: LedgerInfo,
    reconfig: bool,
}

pub struct OrderedBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
}

pub type BufferItemRootType = Link<BufferItem>;
pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

/// StateManager handles the states of ordered blocks and
/// interacts with the execution phase, the signing phase, and
/// the persisting phase.
pub struct StateManager {
    author: Author,

    buffer: List<BufferItem>,

    // the roots point to the first *unprocessed* item.
    execution_root: BufferItemRootType,
    execution_phase_tx: Sender<ExecutionRequest>,
    execution_phase_rx: Receiver<ExecutionResponse>,

    signing_root: BufferItemRootType,
    signing_phase_tx: Sender<SigningRequest>,
    signing_phase_rx: Receiver<SigningResponse>,

    commit_msg_tx: NetworkSender,
    commit_msg_rx: channel::diem_channel::Receiver<AccountAddress, VerifiedEvent>,

    // we don't hear back from the persisting phase
    persisting_phase_tx: Sender<PersistingRequest>,

    block_rx: UnboundedReceiver<OrderedBlocks>,
    sync_rx: UnboundedReceiver<SyncRequest>,
    end_epoch: bool,

    verifier: ValidatorVerifier,
}

impl StateManager {
    pub fn new(
        author: Author,
        execution_phase_tx: Sender<ExecutionRequest>,
        execution_phase_rx: Receiver<ExecutionResponse>,
        signing_phase_tx: Sender<SigningRequest>,
        signing_phase_rx: Receiver<SigningResponse>,
        commit_msg_tx: NetworkSender,
        commit_msg_rx: channel::diem_channel::Receiver<AccountAddress, VerifiedEvent>,
        persisting_phase_tx: Sender<PersistingRequest>,
        block_rx: UnboundedReceiver<OrderedBlocks>,
        sync_rx: UnboundedReceiver<SyncRequest>,
        verifier: ValidatorVerifier,
    ) -> Self {
        let buffer = List::<BufferItem>::new();

        // point the roots to the head
        let execution_root = buffer.head.as_ref().cloned();
        let signing_root = buffer.head.as_ref().cloned();

        Self {
            author,

            buffer,

            execution_root,
            execution_phase_tx,
            execution_phase_rx,

            signing_root,
            signing_phase_tx,
            signing_phase_rx,

            commit_msg_tx,
            commit_msg_rx,

            persisting_phase_tx,

            block_rx,
            sync_rx,
            end_epoch: false,

            verifier,
        }
    }

    /// process incoming ordered blocks
    /// push them into the buffer and update the roots if they are none.
    async fn process_ordered_blocks(
        &mut self,
        ordered_blocks: OrderedBlocks,
    ) -> anyhow::Result<()> {
        let OrderedBlocks {
            ordered_blocks,
            ordered_proof,
            callback,
        } = ordered_blocks;

        // push blocks to buffer
        self.buffer.push_back(BufferItem::new_ordered(
            ordered_blocks.clone(),
            ordered_proof,
            callback,
        ));

        // when all the previous items are processed..
        if self.execution_root.is_none() {
            self.execution_root = self.buffer.tail.clone();
        }
        if self.signing_root.is_none() {
            self.signing_root = self.buffer.tail.clone();
        }

        // send blocks to execution phase
        self.execution_phase_tx
            .send(ExecutionRequest { ordered_blocks })
            .await?;
        Ok(())
    }

    /// check if the items at and after the execution root is already executed
    /// if yes, move the execution root to the first *unexecuted* item.
    /// if there is no such item, set it to none.
    fn try_advance_executed_root(&mut self) {
        let mut cursor = self.execution_root.clone();
        while cursor.is_some() {
            {
                let buffer_item = get_elem(&cursor);
                if !matches!(&*buffer_item, BufferItem::Executed(_)) {
                    break;
                }
            }
            cursor = get_next(&cursor);
        }
        self.execution_root = cursor;
    }

    /// check if the items at and after the signing root is already signed
    /// if yes, move the signing root to the first *unsigned* item.
    /// if there is no such item, set it to none.
    fn try_advance_signing_root(&mut self) {
        let mut cursor = self.signing_root.clone();
        while cursor.is_some() {
            {
                let buffer_item = get_elem(&cursor);
                if !matches!(
                    &*buffer_item,
                    BufferItem::Signed(_) | BufferItem::Aggregated(_)
                ) {
                    break;
                }
            }
            cursor = get_next(&cursor);
        }
        self.signing_root = cursor;
    }

    /// check if a prefix of the buffer is ready to persist,
    /// if yes, send them to the persisting phase and dequeue the buffer items.
    fn try_persisting_blocks(&mut self) {
        let mut cursor = self.buffer.head.as_ref().cloned();
        while cursor.is_some() {
            let buffer_item = take_elem(&cursor);
            if let BufferItem::Aggregated(aggregated) = buffer_item {
                let blocks: Vec<Arc<ExecutedBlock>> = aggregated
                    .executed_blocks
                    .into_iter()
                    .map(Arc::new)
                    .collect();
                // send to persisting phase
                block_on(self.persisting_phase_tx.send(PersistingRequest {
                    blocks,
                    commit_ledger_info: aggregated.aggregated_proof,
                    callback: aggregated.callback,
                }))
                .ok();
                cursor = get_next(&cursor);
                self.buffer.pop_front();
            } else {
                // we put the item back
                set_elem(&cursor, buffer_item);
                break;
            }
        }
    }

    /// update the root to make sure that they point to the first *unprocessed* item.
    fn reset_all_roots(&mut self) {
        // reset all the roots (in a better way)
        self.signing_root = self.buffer.head.clone();
        self.try_advance_signing_root();
        self.execution_root = self.signing_root.clone();
        self.try_advance_executed_root();
    }

    /// this function processes a sync request
    /// if reconfig flag is set, it stops the main loop
    /// otherwise, it empties the buffer till ledger_info, and update the roots
    /// finally, it sends back an ack.
    async fn process_sync_request(&mut self, sync_event: SyncRequest) -> anyhow::Result<()> {
        let SyncRequest {
            tx,
            ledger_info,
            reconfig,
        } = sync_event;

        if reconfig {
            // buffer manager will stop
            self.end_epoch = true;
        } else {
            // clear the buffer until (including) the ledger_info
            while let Some(buffer_item) = self.buffer.pop_front() {
                if buffer_item
                    .get_commit_info()
                    .match_ordered_only(ledger_info.commit_info())
                {
                    break;
                }
            }

            // reset roots
            self.reset_all_roots();
        }

        // ack reset
        tx.send(sync_ack_new()).unwrap();
        Ok(())
    }

    /// this function updates the buffer according to the response from the execution phase
    /// it also initiates a request to the signing phase.
    async fn process_successful_execution_response(
        &mut self,
        executed_blocks: Vec<ExecutedBlock>,
    ) -> anyhow::Result<()> {
        if self.execution_root.is_none() {
            // right after a sync
            return Ok(());
        }

        let current_cursor = self.execution_root.clone();

        if current_cursor.is_some() {
            // update buffer
            let buffer_item = take_elem(&current_cursor);
            if let BufferItem::Ordered(ordered_box) = &buffer_item {
                // the block batch in the response might be a single large batch due
                // to a previous re-try, so we compare the id of the last blocks of the
                // two batches.
                if ordered_box.ordered_blocks.last().unwrap().id()
                    != executed_blocks.last().unwrap().id()
                {
                    // an sync req happened before the response
                    // we do nothing except putting the item back
                    // the process_execution_resp function will retry the next ordered batch
                    set_elem(&current_cursor, buffer_item);
                    return Ok(());
                }

                // push to the signing phase
                let commit_ledger_info = LedgerInfo::new(
                    executed_blocks.last().unwrap().block_info(),
                    ordered_box
                        .ordered_proof
                        .ledger_info()
                        .consensus_data_hash(),
                );

                self.signing_phase_tx
                    .send(SigningRequest {
                        ordered_ledger_info: ordered_box.ordered_proof.clone(),
                        commit_ledger_info,
                    })
                    .await?;

                // it is possible that executed_blocks is a large batch from a retry.
                // ordered_box.ordered_blocks should always be a suffix of executed_blocks.
                let trimmed_executed_blocks = executed_blocks
                    [executed_blocks.len() - ordered_box.ordered_blocks.len()..]
                    .to_vec();

                set_elem(
                    &current_cursor,
                    buffer_item.advance_to_executed(trimmed_executed_blocks),
                );
                self.execution_root = get_next(&current_cursor);
            } else {
                // even if there is a sync happened before the response
                // the buffer item right after execution root should be an ordered buffer item
                panic!("Inconsistent buffer item state");
            }
        }
        Ok(())
    }

    /// this function handles the execution response
    /// if the execution succeeded, it calls process_successful_execution_response
    /// to update the buffer and sends an signing request.
    /// if the execution fails: it re-collects a larger batch and retries an execution request.
    async fn process_execution_response(
        &mut self,
        execution_resp: ExecutionResponse,
    ) -> anyhow::Result<()> {
        // we do not use callback from the execution phase to fetch the retry blocks
        // because we want the buffer accessed by a single thread

        let ExecutionResponse { inner } = execution_resp;

        if let Ok(executed_blocks) = inner {
            self.process_successful_execution_response(executed_blocks)
                .await
            // we try the next one only when the last req failed
        } else {
            // it might be possible that the buffer is already reset
            // in which case we are iterating an irrelevant large batch
            // this is ok as blocks can be executed more than once
            let mut cursor = self.buffer.head.clone();
            let mut ordered_blocks: Vec<ExecutedBlock> = vec![];
            while cursor.is_some() {
                ordered_blocks.extend(get_elem(&cursor).get_blocks().clone());
                if link_eq(&cursor, &self.execution_root) {
                    // there must be a successor since the last execution failed
                    cursor = get_next(&cursor);
                    if cursor.is_none() {
                        // at this moment we are certain that a reset has happened
                        // so we do not need to retry the batch
                        break;
                    }
                    ordered_blocks.extend(get_elem(&cursor).get_blocks().clone());
                    // retry execution with the larger batch
                    // send blocks to execution phase
                    self.execution_phase_tx
                        .send(ExecutionRequest { ordered_blocks })
                        .await?;
                    break;
                }
                cursor = get_next(&cursor);
            }
            // the only case that cursor did not meet execution root is when the buffer is empty
            // in which case we do nothing
            Ok(())
        }
    }

    /// if the signing response is successful, update the signature of
    /// the corresponding buffer item, and broadcast a commit vote message
    async fn process_successful_signing_response(
        &mut self,
        sig: Ed25519Signature,
        commit_ledger_info: LedgerInfo,
    ) -> anyhow::Result<()> {
        let mut current_cursor = self.signing_root.clone();
        if current_cursor.is_some() {
            // this is important because the responses might not come in order because
            // retrying a failed signature and finishing execution will incur requests
            // to signing phase
            while current_cursor.is_some() {
                // update signature
                let buffer_item = take_elem(&current_cursor);
                if !buffer_item.has_been_executed() {
                    // a reset has happened
                    // we do nothing except put it back
                    set_elem(&current_cursor, buffer_item);
                    break;
                }

                if buffer_item.get_commit_info() == commit_ledger_info.commit_info() {
                    // it is possible that we already signed this buffer item (double check after the final integration)
                    if matches!(buffer_item, BufferItem::Executed(_)) {
                        // we have found the buffer item
                        let (signed_buffer_item, commit_vote) =
                            buffer_item.advance_to_signed(self.author, sig.clone(), &self.verifier);

                        set_elem(&current_cursor, signed_buffer_item);

                        // send out commit vote
                        self.commit_msg_tx
                            .broadcast(ConsensusMsg::CommitVoteMsg(Box::new(commit_vote)))
                            .await;

                        self.try_advance_signing_root();
                    }
                    break;
                }
                current_cursor = get_next(&current_cursor);
            }
        }
        // otherwise, a reset happened, we do nothing
        Ok(())
    }

    /// if the signing response is successful, call process_successful_signing_response
    /// otherwise, retry the item pointed by the signing root.
    async fn process_signing_response(&mut self, response: SigningResponse) -> anyhow::Result<()> {
        let SigningResponse {
            signature_result,
            commit_ledger_info,
        } = response;
        if let Ok(sig) = signature_result {
            self.process_successful_signing_response(sig, commit_ledger_info)
                .await
        } else {
            // try next signature if signing failure
            // note that we are not retrying exactly the failed sig
            // the failed sig will be re-tried in the future, unless a reset happens

            /*
            Signing root points to the first unprocessed item.
            But there might be Signed item scattered after the signing root.
            Below situation is possible:

            [Signed] -> [Signed] -> [Executed] -> [Signed] -> [Executed] -> [Signed] -> [Executed] ...
            And the signing root points to the third item.
            The failure could be related to the 5-th item.

            This might happen because signing phase might not see the items in order
            (success execution response and failed signing response will both push
            an item to signing phase)
             */

            let current_cursor = self.signing_root.clone();
            if current_cursor.is_some() {
                let buffer_item = get_elem(&current_cursor);
                if let BufferItem::Executed(executed) = &(*buffer_item) {
                    self.signing_phase_tx
                        .send(SigningRequest {
                            ordered_ledger_info: executed.ordered_proof.clone(),
                            commit_ledger_info: executed.generate_commit_ledger_info(),
                        })
                        .await?;
                }
            }
            Ok(())
        }
    }

    /// process the commit vote messages
    /// it scans the whole buffer for a matching blockinfo
    /// if found, try advancing the item to be aggregated
    async fn process_commit_msg(&mut self, commit_msg: VerifiedEvent) -> anyhow::Result<()> {
        match commit_msg {
            VerifiedEvent::CommitVote(cv) => {
                // travel the whole buffer (including ordered items)
                let mut current_cursor = self.buffer.head.clone();
                while current_cursor.is_some() {
                    {
                        let mut buffer_item = get_elem_mut(&current_cursor);
                        if buffer_item
                            .add_signature_if_matched(
                                cv.commit_info(),
                                cv.author(),
                                cv.signature().clone(),
                            )
                            .is_ok()
                        {
                            // try advance to aggregated
                            let taken_buffer_item = take_elem(&current_cursor);
                            let (new_buffer_item, success) =
                                taken_buffer_item.try_advance_to_aggregated(&self.verifier);
                            set_elem(&current_cursor, new_buffer_item);
                            // if successfully advanced to an aggregated item
                            if success {
                                self.try_persisting_blocks();
                            }
                            break;
                        }
                    }
                    current_cursor = get_next(&current_cursor);
                }
            }
            _ => {
                unreachable!();
            }
        }
        Ok(())
    }

    /// this function retries all the items until the signing root
    /// note that there might be other signed items after the signing root
    async fn retry_broadcasting_commit_votes(&mut self) -> anyhow::Result<()> {
        let mut cursor = self.buffer.head.clone();
        while cursor.is_some() && !link_eq(&cursor, &self.signing_root) {
            // we move forward before sending the message
            // just in case the buffer becomes empty during await.
            let next_cursor = get_next(&cursor);
            {
                let buffer_item = get_elem(&cursor);
                match &*buffer_item {
                    BufferItem::Aggregated(_) => continue, // skip aggregated items
                    BufferItem::Signed(signed) => {
                        self.commit_msg_tx
                            .broadcast(ConsensusMsg::CommitVoteMsg(Box::new(
                                signed.commit_vote.clone(),
                            )))
                            .await;
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
            cursor = next_cursor;
        }
        Ok(())
    }

    async fn start(mut self) {
        info!("Buffer manager starts.");
        let mut interval =
            tokio::time::interval(Duration::from_millis(BUFFER_MANAGER_RETRY_INTERVAL));
        while !self.end_epoch {
            // process new messages
            if let Err(e) = tokio::select! {
                Some(blocks) = self.block_rx.next() => {
                    self.process_ordered_blocks(blocks).await
                }
                Some(reset_event) = self.sync_rx.next() => {
                    self.process_sync_request(reset_event).await
                }
                Some(execution_resp) = self.execution_phase_rx.next() => {
                    self.process_execution_response(execution_resp).await
                }
                Some(signing_resp) = self.signing_phase_rx.next() => {
                    self.process_signing_response(signing_resp).await
                }
                Some(commit_msg) = self.commit_msg_rx.next() => {
                    self.process_commit_msg(commit_msg).await
                }
                _ = interval.tick() => {
                    self.retry_broadcasting_commit_votes().await
                }
                // no else branch here because interval.tick will always be available
            } {
                counters::ERROR_COUNT.inc();
                error!("BufferManager error: {}", e.to_string());
            }
        }
        // loop receving new blocks or reset
        // while !self.end_epoch {

        // select from all rx channels,
        // if new from block_rx, push to buffer
        // if new from reset_rx, make a mark that stops all the following ops
        // if new from execution_phase_rx,
        //   if execution failure, send all the blocks to execution_phase.
        //   Otherwise,
        //     update execution_root and send the blocks from execution_root to end to execution_phase
        // if new from signing_phase_rx,
        //   update sigining_root and send the blocks from signing_root to execution_root to signing_phase
        // if new from commit_msg_rx,
        //   collect sig and update the sigs
        //   if aggregated,
        //     update aggregation_root
        // if new from persisting_phase_rx,
        //   pop blocks from buffer, and continue to post-committing ops
        //   send the blocks from aggregation_root to the end to persisting_phase

        // if not reset, retry sending the commit_vote msg via commit_msg_tx
        // }
    }
}
