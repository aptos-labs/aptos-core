use crate::{
    framework::{ContextFor, NodeId, Protocol},
    leader_schedule::LeaderSchedule,
    metrics,
    metrics::Sender,
    protocol, Slot,
};
use defaultmap::DefaultBTreeMap;
use std::{cmp::max, sync::Arc, time::Duration};
use tokio::time::Instant;

type BatchDepth = Slot;
type ChainId = usize;

#[derive(Clone)]
pub struct Block {
    slot: Slot,
    // In practice, this would be the actual availability certificates.
    acs: Vec<BatchDepth>,
    // In practice, this would be the hashes.
    optimistic_batches: Option<Vec<BatchDepth>>,

    // In practice, this would be the hash of the previous block.
    prev: Option<Arc<Block>>,
}

impl Block {
    fn genesis(n_nodes: usize) -> Self {
        Block {
            slot: 0,
            acs: vec![0; n_nodes],
            optimistic_batches: None,
            prev: None,
        }
    }
}

#[derive(Clone)]
pub enum Message {
    Entering(Slot),
    // Dissemination layer messages
    Batch(BatchDepth),
    BatchStored(BatchDepth),
    AvailabilityCert(BatchDepth),
    // PBFT messages
    // For efficiency of the simulation, block is passed by reference.
    // For simplicity, we proactively forward the block in all the messages.
    // In practice, this can be optimized with a pulling mechanism.
    ViewChange(Slot, Arc<Block>),
    Propose(Slot, Arc<Block>),
    Prepare(Slot, Arc<Block>),
    Commit(Slot, Arc<Block>),
    CommitCert(Slot, Arc<Block>),

    // This message is injected when the node should crash
    Crash,
}

pub enum TimerEvent {
    FixedTimer(Slot),
    AdaptiveTimer(Slot),
    LogStatus,
}

#[derive(Clone, Copy)]
pub struct Config<S> {
    pub n_nodes: usize,
    // n_chains = slots_per_delta * leader_timeout
    pub slots_per_delta: Slot,
    pub leader_timeout: u32, // in deltas
    pub leader_schedule: S,
    pub delta: Duration,
    pub progress_threshold: f64,
    pub slot_duration_sample_size: Slot,
    pub responsive: bool,
    pub adaptive_timer: bool,
    pub optimistic_dissemination: bool,

    pub halt_on_slot: Slot, // for how many slots to run the experiment
}

impl<S> Config<S> {
    pub fn n_chains(&self) -> usize {
        self.slots_per_delta as usize * self.leader_timeout as usize
    }

    // Chains are numbered from 1 to n_chains including.
    pub fn chain(&self, slot: Slot) -> ChainId {
        ((slot - 1) % self.n_chains() as Slot) as ChainId + 1
    }
}

pub struct Metrics {
    pub propose_time: Option<metrics::UnorderedSender<f64>>,
    pub enter_time: Option<metrics::UnorderedSender<f64>>,
    pub batch_commit_time: Option<metrics::UnorderedSender<(BatchDepth, f64)>>,
    pub indirectly_committed_slots: Option<metrics::UnorderedSender<Slot>>,
}

pub struct MultiChainBft<S> {
    node_id: NodeId,
    config: Config<S>,

    // Logging and metrics
    start_time: Instant,
    detailed_logging: bool,
    metrics: Metrics,

    // Slot synchronization
    entered: Slot,
    slot: Slot,

    nodes_entered: DefaultBTreeMap<Slot, usize>,

    fixed_timer_elapsed: DefaultBTreeMap<Slot, bool>,
    adaptive_timer_elapsed: DefaultBTreeMap<Slot, bool>,
    slot_enter_time: DefaultBTreeMap<Slot, Instant>,

    // Dissemination
    batch_send_time: DefaultBTreeMap<BatchDepth, Instant>,
    sent_batch_depth: BatchDepth,
    stored_batch_depth: BatchDepth,
    batch_stored_votes: DefaultBTreeMap<BatchDepth, usize>,
    known_batch: Vec<BatchDepth>,
    known_ac: Vec<BatchDepth>,
    committed_batch: Vec<BatchDepth>,

    // PBFT
    lock: DefaultBTreeMap<ChainId, Arc<Block>>,

    sent_view_change: DefaultBTreeMap<Slot, bool>,
    view_change_votes: DefaultBTreeMap<Slot, usize>,

    sent_prepare: DefaultBTreeMap<Slot, bool>,
    prepare_votes: DefaultBTreeMap<Slot, usize>,

    sent_commit: DefaultBTreeMap<Slot, bool>,
    commit_votes: DefaultBTreeMap<Slot, usize>,

    committed: DefaultBTreeMap<Slot, bool>,
    block: DefaultBTreeMap<Slot, Arc<Block>>,
    committed_prefix: Slot,
}

impl<S: LeaderSchedule> MultiChainBft<S> {
    pub fn new_node(
        node_id: NodeId,
        config: Config<S>,
        start_time: Instant,
        detailed_logging: bool,
        metrics: Metrics,
    ) -> Self {
        let n_nodes = config.n_nodes;

        MultiChainBft {
            node_id,
            config,
            start_time,
            detailed_logging,
            metrics,

            // Protocol state
            entered: 0,
            slot: 0,
            nodes_entered: Default::default(),
            fixed_timer_elapsed: Default::default(),
            adaptive_timer_elapsed: Default::default(),
            slot_enter_time: DefaultBTreeMap::new(Instant::now()),
            batch_send_time: DefaultBTreeMap::new(Instant::now()),
            sent_batch_depth: 0,
            stored_batch_depth: 0,
            batch_stored_votes: Default::default(),
            known_batch: vec![0; n_nodes],
            known_ac: vec![0; n_nodes],
            committed_batch: vec![0; n_nodes],
            lock: DefaultBTreeMap::new(Arc::new(Block::genesis(n_nodes))),
            sent_view_change: Default::default(),
            view_change_votes: Default::default(),
            sent_prepare: Default::default(),
            prepare_votes: Default::default(),
            sent_commit: Default::default(),
            commit_votes: Default::default(),
            committed: Default::default(),
            block: DefaultBTreeMap::new(Arc::new(Block::genesis(n_nodes))),
            committed_prefix: 0,
        }
    }

    fn f(&self) -> usize {
        (self.config.n_nodes - 1) / 3
    }

    fn quorum(&self) -> usize {
        // ceil((n + f + 1) / 2) = floor((n + f + 2) / 2) = floor((n + f) / 2) + 1
        (self.config.n_nodes + self.f()) / 2 + 1
    }

    fn n_nodes(&self) -> usize {
        self.config.n_nodes
    }

    fn n_chains(&self) -> usize {
        self.config.n_chains()
    }

    fn chain(&self, slot: Slot) -> ChainId {
        self.config.chain(slot)
    }

    fn leader(&self, slot: Slot) -> NodeId {
        (self.config.leader_schedule)(slot)
    }

    fn progress_condition(&self) -> bool {
        if !self.config.responsive {
            // This makes sure that the responsive condition for advancing slots will never be met.
            // Hence, the pacing will be determined by the non-responsive path.
            return false;
        }

        let next_slot = self.slot + 1;
        let first_uncommitted = self.committed_prefix + 1;

        // Fall back to non-responsive when 2 consecutive leaders
        // in the same chain failed to make progress.
        if next_slot >= first_uncommitted + 2 * (self.n_chains() as Slot) + 1 {
            return false;
        }

        let sent_prepare = (2 * self.config.slots_per_delta..3 * self.config.slots_per_delta)
            .filter(|i| self.sent_prepare[next_slot - i])
            .count();
        let sent_commit = (3 * self.config.slots_per_delta..4 * self.config.slots_per_delta)
            .filter(|i| self.sent_commit[next_slot - i])
            .count();
        let committed = (4 * self.config.slots_per_delta..5 * self.config.slots_per_delta)
            .filter(|i| self.committed[next_slot - i])
            .count();

        let progress_score = (sent_prepare + sent_commit + committed) as f64
            / (3 * self.config.slots_per_delta) as f64;

        let res = progress_score >= self.config.progress_threshold;

        if res {
            self.log_detail(format!(
                "progress condition met: sent_prepare={}, sent_commit={}, committed={}, score={:.2}",
                sent_prepare, sent_commit, committed, progress_score,
            ));
        }

        res
    }

    fn time_in_delta(&self) -> f64 {
        (Instant::now() - self.start_time).as_secs_f64() / self.config.delta.as_secs_f64()
    }

    fn fixed_timer_duration(&self) -> Duration {
        self.config.delta / self.config.slots_per_delta as u32
    }

    fn adaptive_timer_duration(&self) -> Duration {
        if !self.config.adaptive_timer {
            // If the adaptive timer is turned off,
            // just let the progress condition determine pacing.
            return Duration::ZERO;
        }

        let sample_size = self.config.slot_duration_sample_size;
        if self.slot > sample_size {
            // If we have enough data, use that to estimate the average slot time.
            let average_slot_time = (self.slot_enter_time[self.slot]
                - self.slot_enter_time[self.slot - sample_size])
                / (sample_size as u32);
            average_slot_time * 9 / 10
        } else {
            // Otherwise, just let the progress condition determine pacing.
            Duration::ZERO
        }
    }

    fn log_info(&self, msg: String) {
        aptos_logger::info!(
            "Node {} at {:.2}Δ: {}",
            self.node_id,
            self.time_in_delta(),
            msg
        );
    }

    fn log_detail(&self, msg: String) {
        if self.detailed_logging {
            self.log_info(msg);
        }
    }

    fn add_dummy_blocks(&self, prev: Arc<Block>, slot: Slot) -> Arc<Block> {
        if prev.slot + self.n_chains() as Slot >= slot {
            prev
        } else {
            let dummy = Arc::new(Block {
                slot: if prev.slot == 0 {
                    // if prev is genesis, then the dummy slot is the first slot of the chain.
                    self.chain(slot) as Slot
                } else {
                    prev.slot + self.n_chains() as Slot
                },
                acs: prev.acs.clone(),
                optimistic_batches: prev.optimistic_batches.clone(),
                prev: Some(prev),
            });

            self.add_dummy_blocks(dummy, slot)
        }
    }

    async fn commit(&mut self, ctx: &mut impl ContextFor<Self>, slot: Slot, mut block: Arc<Block>) {
        if !self.committed[slot] {
            ctx.multicast(Message::CommitCert(slot, block.clone()))
                .await;
        }

        // Commit the whole prefix of the chain.
        while !self.committed[block.slot] {
            if block.slot < slot {
                self.log_detail(format!("slot {} committed indirectly", block.slot));
                self.metrics.indirectly_committed_slots.push(block.slot);
            }
            self.committed[block.slot] = true;
            self.block[block.slot] = block.clone();
            block = block.prev.clone().unwrap().clone();
        }
    }
}

//noinspection RsTraitImplementation (suppress false positive IDE warning)
impl<S: LeaderSchedule> Protocol for MultiChainBft<S> {
    type Message = Message;
    type TimerEvent = TimerEvent;

    protocol! {
        self: self;
        ctx: ctx;

        upon start {
            self.entered = 0;
            self.slot = 1;

            // Fill in the data for non-existing slots.
            for slot in -10 * self.config.slots_per_delta..=0 {
                self.nodes_entered[slot] = self.config.n_nodes;
                self.sent_prepare[slot] = true;
                self.sent_commit[slot] = true;
                self.committed[slot] = true;
            }

            self.log_detail(format!("node {} started", ctx.node_id()));
        };

        // Slot synchronization protocol

        upon [self.entered < self.slot] {
            self.entered += 1;
            self.slot_enter_time[self.entered] = Instant::now();
            ctx.multicast(Message::Entering(self.entered)).await;

            if self.entered == self.slot {
                ctx.set_timer(self.fixed_timer_duration(), TimerEvent::FixedTimer(self.slot));
                ctx.set_timer(self.adaptive_timer_duration(), TimerEvent::AdaptiveTimer(self.slot));
            }

            self.log_detail(format!("entering slot {}", self.entered));
            self.metrics.enter_time.push(self.time_in_delta());
        };

        upon receive [Message::Entering(s)] from [_any_node] {
            self.nodes_entered[s] += 1;

            if self.nodes_entered[s] == self.f() + 1 && self.slot < s {
                self.log_detail(format!("jump from slot {} to {}", self.slot, s));
                self.slot = s;
            }
        };

        upon timer event [TimerEvent::FixedTimer(s)] {
            self.fixed_timer_elapsed[s] = true;
        };

        upon timer event [TimerEvent::AdaptiveTimer(s)] {
            self.adaptive_timer_elapsed[s] = true;
        };

        upon [
            self.entered == self.slot
            && self.nodes_entered[self.slot + 1 - self.config.slots_per_delta] >= self.quorum()
            && (
                self.fixed_timer_elapsed[self.slot]
                || (self.adaptive_timer_elapsed[self.slot] && self.progress_condition())
            )
        ] {
            // Advance to the next slot.
            self.slot += 1;
        };

        // Dissemination layer

        upon [self.sent_batch_depth < self.slot] {
            self.sent_batch_depth += 1;
            ctx.multicast(Message::Batch(self.sent_batch_depth)).await;
            self.batch_send_time[self.sent_batch_depth] = Instant::now();
        };

        upon receive [Message::Batch(depth)] from node [p] {
            ctx.unicast(Message::BatchStored(depth), p).await;
            self.known_batch[p] = max(self.known_batch[p], depth);
        };

        upon receive [Message::BatchStored(depth)] from [_any_node] {
            self.batch_stored_votes[depth] += 1;
        };

        upon [self.batch_stored_votes[self.stored_batch_depth + 1] >= self.quorum()] {
            self.stored_batch_depth += 1;
            ctx.multicast(Message::AvailabilityCert(self.stored_batch_depth)).await;
        };

        upon receive [Message::AvailabilityCert(depth)] from node [p] {
            self.known_ac[p] = max(self.known_ac[p], depth);
            self.known_batch[p] = max(self.known_batch[p], depth);
        };

        // PBFT protocol

        upon [!self.sent_view_change[self.slot]] {
            self.sent_view_change[self.slot] = true;
            ctx.unicast(
                Message::ViewChange(self.slot, self.lock[self.chain(self.slot)].clone()),
                self.leader(self.slot),
            ).await;
        };

        upon receive [Message::ViewChange(s, prev)] from [_any_node] {
            // leader node
            self.view_change_votes[s] += 1;

            if prev.slot > self.lock[self.config.chain(s)].slot {
                self.lock[self.config.chain(s)] = prev;
            }

            if self.view_change_votes[s] == self.quorum() {
                // Do not propose optimistically if commits are lagging behind.
                let optimistic_proposal = self.config.optimistic_dissemination
                    && self.committed_prefix >= self.slot - self.n_chains() as Slot;

                let block = Arc::new(Block {
                    slot: s,
                    acs: self.known_ac.clone(),
                    optimistic_batches: if optimistic_proposal {
                        Some(self.known_batch.clone())
                    } else {
                        None
                    },
                    prev: Some(self.add_dummy_blocks(self.lock[self.chain(s)].clone(), s)),
                });
                ctx.multicast(Message::Propose(s, block)).await;

                self.log_info(format!("slot {} proposed", s));
                self.metrics.propose_time.push(self.time_in_delta());
            }
        };

        upon receive [Message::Propose(s, block)] from [_leader] 'handler: {
            if self.slot >= s + self.n_chains() as Slot {
                break 'handler;  // This slot has been already timed out.
            }

            if !self.sent_prepare[s] {
                self.sent_prepare[s] = true;
                self.lock[self.config.chain(s)] = block.clone();
                ctx.multicast(Message::Prepare(s, block)).await;
            }
        };

        upon receive [Message::Prepare(s, block)] from [_any_node] {
            self.prepare_votes[s] += 1;
            self.block[s] = block;
        };

        for [s in self.slot - self.n_chains() as Slot + 1..=self.slot]
        upon [
            !self.sent_commit[s]
            && self.prepare_votes[s] >= self.quorum()
            && match &self.block[s].optimistic_batches {
                None => true,
                Some(batches) => (0..self.n_nodes()).into_iter().all(|p| self.known_batch[p] >= batches[p]),
            }
        ] {
            self.sent_commit[s] = true;
            ctx.multicast(Message::Commit(s, self.block[s].clone())).await;
        };

        upon receive [Message::Commit(s, block)] from [_any_node] {
            self.commit_votes[s] += 1;

            if self.commit_votes[s] == self.quorum() {
                self.commit(ctx, s, block).await;
            }
        };

        upon receive [Message::CommitCert(s, block)] from [_any_node] {
            self.commit(ctx, s, block).await;
        };

        upon [self.committed[self.committed_prefix + 1]] {
            self.committed_prefix += 1;

            let block = &self.block[self.committed_prefix];

            let committed = if let Some(batches) = &block.optimistic_batches {
                &batches
            } else {
                &block.acs
            };

            // Measure the commit time of your own batches.
            for depth in self.committed_batch[self.node_id] + 1..=committed[self.node_id] {
                self.log_detail(format!("committed batch #{}", depth));
                let commit_time = self.batch_send_time[depth].elapsed().as_secs_f64()
                    / self.config.delta.as_secs_f64();
                self.metrics.batch_commit_time.push((depth, commit_time));
            }

            // Update the information about the committed data.
            for p in 0..self.config.n_nodes {
                self.committed_batch[p] = max(self.committed_batch[p], committed[p]);
            }
        };

        // Logging and halting

        upon start {
            ctx.set_timer(Duration::from_secs(5), TimerEvent::LogStatus);
        };

        upon timer event [TimerEvent::LogStatus] {
            self.log_detail(format!(
                "STATUS:\
                    \n\tslot: {}\
                    \n\t#nodes entered slot {}: {} (should be all)\
                    \n\t#nodes entered slot {}: {} (quorum needed to advance slot)\
                    \n\t#nodes entered slot {}: {} (in my slot)\
                    \n\t#nodes entered slot {}: {} (already in the next slot)\
                    \n\tcommitted prefix: {}\
                    \n\tknown_batch[self]: {}\
                    \n\tknown_ac[self]: {}\
                    \n\tcommitted_ac[self]: {}\
                    \n\ttimer elapsed: {:?}",
                self.slot,
                self.slot - 2 * self.config.slots_per_delta,
                self.nodes_entered[self.slot - 2 * self.config.slots_per_delta],
                self.slot + 1 - self.config.slots_per_delta,
                self.nodes_entered[self.slot + 1 - self.config.slots_per_delta],
                self.slot,
                self.nodes_entered[self.slot],
                self.slot + 1,
                self.nodes_entered[self.slot + 1],
                self.committed_prefix,
                self.known_batch[self.node_id],
                self.known_ac[self.node_id],
                self.committed_batch[self.node_id],
                self.fixed_timer_elapsed[self.slot],
            ));
            ctx.set_timer(Duration::from_secs(5), TimerEvent::LogStatus);
        };

        upon [self.slot == self.config.halt_on_slot] {
            ctx.halt();
        };

        upon receive [Message::Crash] from [_] {
            ctx.halt();
        };
    }
}
