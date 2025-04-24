use crate::{
    framework::{NodeId, Protocol},
    leader_schedule::LeaderSchedule,
    metrics,
    metrics::Sender,
    protocol, Slot,
};
use bitvec::vec::BitVec;
use defaultmap::DefaultBTreeMap;
use std::{
    cmp::{max, max_by, max_by_key, Ordering},
    collections::{BTreeSet, HashSet},
    fmt::{Debug, Formatter},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::mpsc, time::Instant};

pub type Round = i64; // Round number.

pub type BatchSN = i64; // Sequence number of a batch.

#[derive(Clone)]
pub struct Batch<T> {
    node: NodeId,
    sn: BatchSN,
    txn: T,
}

impl<T> Batch<T> {
    pub fn get_ref(&self) -> BatchRef {
        BatchRef {
            node: self.node,
            sn: self.sn,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct BatchRef {
    node: NodeId,
    sn: BatchSN,
}

impl Debug for BatchRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ node: {}, sn: {} }}", self.node, self.sn)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AC {
    // In practice, this would be a hash pointer.
    batch: BatchRef,
    signers: BitVec,
}

#[derive(Clone)]
pub struct Block {
    round: Round,
    acs: Vec<AC>,
    batches: Vec<BatchRef>,

    // In practice, this would be the hash of the previous block.
    // `None` only for the genesis block.
    parent_qc: Option<QC>,

    // Can be `None` in non-genesis blocks.
    tc: Option<TC>,
}

impl Block {
    pub fn genesis() -> Self {
        Block {
            round: 0,
            acs: vec![],
            batches: vec![],
            parent_qc: None,
            tc: None,
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }
}

#[derive(Clone)]
pub struct QC {
    round: Round,
    prefix: usize,

    // In practice, this would be a hash pointer.
    block: Arc<Block>,
}

impl QC {
    pub fn genesis() -> Self {
        QC {
            round: 0,
            prefix: 0,
            block: Arc::new(Block::genesis()),
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }
}

impl PartialEq for QC {
    fn eq(&self, other: &Self) -> bool {
        self.round == other.round && self.prefix == other.prefix
    }
}

impl Eq for QC {}

impl PartialOrd for QC {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QC {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by round first and by optimistic_prefix second.
        match self.round.cmp(&other.round) {
            Ordering::Equal => self.prefix.cmp(&other.prefix),
            other => other,
        }
    }
}

#[derive(Clone)]
pub struct TC {
    round: Round,
    highest_qc: QC,
}

impl TC {
    pub fn genesis() -> Self {
        TC {
            round: 0,
            highest_qc: QC::genesis(),
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }
}

#[derive(Clone)]
pub enum Message<T> {
    // Dissemination layer
    Fetch(BatchRef),
    Batch(Batch<T>),
    BatchStored(BatchSN),
    AvailabilityCert(AC),

    // Jolteon
    Block(Block),
    // Vote now includes the length of the prefix of the
    // optimistically proposed batches that the node has.
    Vote(Round, usize),
    Timeout(Round, QC),
    TimeoutCert(Round, TC),
}

#[derive(Clone)]
pub enum TimerEvent {
    // Dissemination layer
    NewBatch(BatchSN),

    // Jolteon
    Timeout(Round),

    // Other
    EndOfRun,
}

#[derive(Clone, Copy)]
pub struct Config<S> {
    pub n_nodes: usize,
    pub f: usize,
    pub storage_requirement: usize,
    pub leader_timeout: u32, // in deltas
    pub leader_schedule: S,
    pub delta: Duration,
    pub batch_interval: Duration,

    pub end_of_run: Instant,
}

pub struct Metrics {
    pub batch_commit_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
}

pub struct JolteonNode<T: Clone, S, NT> {
    node_id: NodeId,
    config: Config<S>,
    next_txn: NT,

    // Logging and metrics
    start_time: Instant,
    detailed_logging: bool,
    batch_created_time: DefaultBTreeMap<BatchSN, Instant>,
    metrics: Metrics,

    // Dissemination layer

    // Storage for all received batches.
    batches: DefaultBTreeMap<BatchRef, Option<Batch<T>>>,
    // Storage of all received ACs.
    acs: DefaultBTreeMap<BatchRef, Option<AC>>,
    // Set of committed batches.
    committed_batches: BTreeSet<BatchRef>,
    // Set of known ACs that are not yet committed.
    new_acs: BTreeSet<BatchRef>,
    // Set of known uncertified batches that are not yet committed.
    new_batches: BTreeSet<BatchRef>,

    // The set of nodes that have stored this node's batch with the given sequence number.
    batch_stored_votes: DefaultBTreeMap<BatchSN, BitVec>,

    // Protocol state as in pseudocode
    r_vote: Round,
    r_cur: Round,
    qc_high: QC,

    // Additional variables for the implementation
    r_entered: Round,
    tc: TC,
    committed_qc: QC,
    block: DefaultBTreeMap<Round, Option<Block>>,
    votes: DefaultBTreeMap<Round, Vec<usize>>,
    timeout_votes: DefaultBTreeMap<Round, usize>,
}

impl<T, S, NT> JolteonNode<T, S, NT>
where
    T: Clone + Send + Sync,
    S: LeaderSchedule,
    NT: Fn() -> T + Send + Sync,
{
    pub fn new(
        id: NodeId,
        config: Config<S>,
        next_txn: NT,
        start_time: Instant,
        detailed_logging: bool,
        metrics: Metrics,
    ) -> Self {
        let n_nodes = config.n_nodes;

        JolteonNode {
            node_id: id,
            config,
            next_txn,
            start_time,
            detailed_logging,
            batch_created_time: DefaultBTreeMap::new(Instant::now()),
            metrics,
            batches: Default::default(),
            committed_batches: Default::default(),
            new_batches: Default::default(),
            batch_stored_votes: DefaultBTreeMap::new(BitVec::repeat(false, n_nodes)),
            acs: Default::default(),
            new_acs: Default::default(),
            r_vote: 0,
            r_cur: 0,
            qc_high: QC::genesis(),
            r_entered: 0,
            tc: TC::genesis(),
            committed_qc: QC::genesis(),
            block: Default::default(),
            votes: Default::default(),
            timeout_votes: Default::default(),
        }
    }

    fn advance_round(&mut self, new_qc: Option<&QC>, new_tc: Option<&TC>) {
        // The replica updates current round rcur ← max(rcur, r) iff
        // • the replica receives or forms a round-(r − 1) quorum certificate qc, or
        // • the replica receives or forms a round-(r − 1) timeout certificate tc.
        if let Some(qc) = new_qc {
            self.r_cur = max(self.r_cur, qc.round + 1);
        }
        if let Some(tc) = new_tc {
            self.r_cur = max(self.r_cur, tc.round + 1);
        }
    }

    fn lock(&mut self, new_qc: &QC) {
        // Upon seeing a valid qc (formed by votes or contained in proposal or timeouts),
        // the replica updates qc_high ← max(qc_high, qc).
        self.qc_high = max(self.qc_high.clone(), new_qc.clone());
    }

    fn commit(&mut self, new_qc: &QC) {
        // If there exists two adjacent certified blocks B, B' in the chain with consecutive
        // round numbers, i.e., B'.r = B.r + 1, the replica commits B and all its ancestors.
        if new_qc.is_genesis() {
            return;
        }

        let parent_qc = new_qc.block.parent_qc.as_ref().unwrap();
        if new_qc.round == parent_qc.round + 1 {
            self.commit_qc(parent_qc);
        }
    }

    fn commit_qc(&mut self, qc: &QC) {
        if *qc <= self.committed_qc {
            return;
        }

        let parent = qc.block.parent_qc.as_ref().unwrap();

        // Check for safety violations:
        if qc.round > self.committed_qc.round && parent.round < self.committed_qc.round {
            panic!("Safety violation: committed block was rolled back");
        }
        if parent.round == self.committed_qc.round && parent.prefix < self.committed_qc.prefix {
            panic!("Safety violation: optimistically committed transactions were rolled back");
        }

        // First commit the parent block.
        self.commit_qc(qc.block.parent_qc.as_ref().unwrap());

        // Then, commit the transactions of this block.

        if qc.round == self.committed_qc.round {
            assert!(qc.prefix > self.committed_qc.prefix);
            self.log_detail(format!(
                "Extending the prefix of committed block {}: {} -> {} / {} ",
                qc.round,
                self.committed_qc.prefix,
                qc.prefix,
                qc.block.batches.len()
            ));
            for batch_ref in qc
                .block
                .batches
                .iter()
                .take(qc.prefix)
                .skip(self.committed_qc.prefix)
                .copied()
            {
                self.commit_batch(batch_ref);
            }
        } else {
            self.log_detail(format!(
                "Committing block {} with prefix {}/{}",
                qc.round,
                qc.prefix,
                qc.block.batches.len()
            ));

            // First, the ACs.
            for ac in &qc.block.acs {
                // self.log_detail(format!("Committing batch {:?} through AC", ac.batch));
                self.commit_batch(ac.batch);
            }

            // And then the optimistically proposed batches.
            for batch_ref in qc.block.batches.iter().take(qc.prefix).copied() {
                // self.log_detail(format!("Committing batch {:?} optimistically", batch_ref));
                self.commit_batch(batch_ref);
            }
        }

        // Finally, update the committed QC variable.
        self.committed_qc = qc.clone();
    }

    fn commit_batch(&mut self, batch_ref: BatchRef) {
        // NB: in this version, purely for simplicity,
        // we do not deduplicate batches before proposing.
        if self.committed_batches.insert(batch_ref) {
            self.new_acs.remove(&batch_ref);
            self.new_batches.remove(&batch_ref);

            if batch_ref.node == self.node_id {
                let commit_time = self.batch_created_time[batch_ref.sn]
                    .elapsed()
                    .as_secs_f64()
                    / self.config.delta.as_secs_f64();
                self.metrics
                    .batch_commit_time
                    .push((self.batch_created_time[batch_ref.sn], commit_time));
            }
        }
    }

    fn quorum(&self) -> usize {
        // Using more general quorum formula that works not only for n = 3f+1,
        // but for any n >= 3f+1.
        (self.config.n_nodes + self.config.f) / 2 + 1
    }

    fn time_in_delta(&self) -> f64 {
        (Instant::now() - self.start_time).as_secs_f64() / self.config.delta.as_secs_f64()
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
}

impl<T, S, NT> Protocol for JolteonNode<T, S, NT>
where
    T: Clone + Send + Sync,
    S: LeaderSchedule,
    NT: Fn() -> T + Send + Sync,
{
    type Message = Message<T>;
    type TimerEvent = TimerEvent;

    // The implementation of the Jolteon protocol follows the pseudocode as closely as possible,
    // with the only exception that all signatures are omitted.
    protocol! {
        self: self;
        ctx: ctx;

        // Dissemination layer
        // In this implementation, batches are simply sent periodically, by a timer.

        upon start {
            // The first batch is sent immediately.
            ctx.set_timer(Duration::ZERO, TimerEvent::NewBatch(1));
        };

        upon timer [TimerEvent::NewBatch(sn)] {
            // Multicast a new batch
            ctx.multicast(Message::Batch(Batch {
                node: self.node_id,
                sn,
                txn: (self.next_txn)(),
            })).await;

            self.batch_created_time[sn] = Instant::now();

            // Reset the timer.
            ctx.set_timer(self.config.batch_interval, TimerEvent::NewBatch(sn + 1));
        };

        upon receive [Message::Batch(batch)] from node [p] {
            // Upon receiving a batch, store it and reply with a BatchStored message.
            let batch_ref = batch.get_ref();
            self.batches[batch_ref] = Some(batch);
            ctx.unicast(Message::BatchStored(batch_ref.sn), p).await;

            // Track the list of known uncommitted uncertified batches.
            if self.acs[batch_ref].is_none() && !self.committed_batches.contains(&batch_ref) {
                self.new_batches.insert(batch_ref);
            }
        };

        upon receive [Message::BatchStored(sn)] from node [p] {
            self.batch_stored_votes[sn].set(p, true);

            // Upon receiving a quorum of BatchStored messages for a batch, form an AC
            // and broadcast it.
            if self.batch_stored_votes[sn].len() == self.quorum() {
                ctx.multicast(Message::AvailabilityCert(AC {
                    batch: BatchRef { node: self.node_id, sn },
                    signers: self.batch_stored_votes[sn].clone(),
                })).await;
            }
        };

        upon receive [Message::AvailabilityCert(ac)] from [_any_node] {
            self.acs[ac.batch] = Some(ac.clone());

            // Track the list of known uncommitted ACs
            // and the list of known uncommitted uncertified batches.
            if !self.committed_batches.contains(&ac.batch) {
                self.new_acs.insert(ac.batch);
                self.new_batches.remove(&ac.batch);
            }
        };

        upon receive [Message::Fetch(batch_ref)] from node [p] {
            // If receive a Fetch message, reply with the batch if it is known.
            if let Some(batch) = &self.batches[batch_ref] {
                ctx.unicast(Message::Batch(batch.clone()), p).await;
            }
        };

        // Steady state protocol

        upon start {
            // Replicas initialize r_vote = 0, r_cur = 1.
            self.r_vote = 0;
            self.r_cur = 1;
        }

        upon [self.r_entered < self.r_cur] {
            self.log_detail(format!("Entering round {}", self.r_cur));

            self.r_entered = self.r_cur;
            let leader = (self.config.leader_schedule)(self.r_cur);

            if self.node_id == leader {
                // Upon entering round r, the leader L_r multicasts a signed block
                // B = [id, qc_high, tc, r, v_cur, txn], where tc = tc_{r−1} if L_r enters round r by
                // receiving a round-(r − 1) tc_{r−1}, and tc = ⊥ otherwise.
                let block = Block {
                    round: self.r_cur,
                    acs: self
                        .new_acs
                        .iter()
                        .map(|&batch_ref| self.acs[batch_ref].clone().unwrap())
                        .collect(),
                    batches: self.new_batches.iter().cloned().collect(),
                    parent_qc: Some(self.qc_high.clone()),
                    tc: if self.tc.round == self.r_cur - 1 {
                        Some(self.tc.clone())
                    } else {
                        None
                    },
                };

                ctx.multicast(Message::Block(block)).await;
            }

            // Upon entering round r, the replica sends the round-(r − 1) TC to L_r if
            // it has the TC, and resets its timer to count down for a predefined time
            // interval (timeout τ).
            if self.tc.round == self.r_cur - 1 {
                ctx.unicast(Message::TimeoutCert(self.r_cur - 1, self.tc.clone()), leader).await;
            }

            let timeout = self.config.leader_timeout * self.config.delta;
            ctx.set_timer(timeout, TimerEvent::Timeout(self.r_cur));
        };

        upon receive [Message::Block(block)] from [leader] {
            if leader == (self.config.leader_schedule)(block.round) {
                self.block[block.round] = Some(block.clone());

                // Upon receiving the first valid block B = [id, qc, r, v, txn] from L_r in
                // round r, execute Advance Round, Lock, and then Commit (defined below).
                let Block { round, parent_qc, tc, batches, .. } = block;
                let qc = parent_qc.unwrap();
                self.advance_round(Some(&qc), tc.as_ref());
                self.lock(&qc);
                self.commit(&qc);

                // Jolteon:
                //     If r = r_cur, v = v_cur, r > r_vote and ((1) r = qc.r + 1, or (2) r = tc.r + 1
                //     and qc.r ≥ max{qc_high.r | qc_high ∈ tc}), vote for B by sending the
                //     threshold signature share {id, r, v}_i to L_{r+1}, and update r_vote ← r.
                // This protocol:
                //     The vote additionally contains the length of the prefix of the optimistically
                //     proposed batches that the node has stored.

                let prefix = batches.iter().take_while(|&&batch_ref| {
                    self.batches[batch_ref].is_some()
                }).count();

                let valid_tc = match tc {
                    Some(tc) => round == tc.round + 1 && qc >= tc.highest_qc,
                    None => false,
                };

                if round == self.r_cur && round > self.r_vote && (round == qc.round + 1 || valid_tc) {
                    let next_leader = (self.config.leader_schedule)(round + 1);
                    ctx.unicast(Message::Vote(round, prefix), next_leader).await;
                    self.r_vote = round;
                }
            }
        };

        upon receive [Message::Vote(round, prefix)] from [_any_node] {
            self.votes[round].push(prefix);
        };

        // Not explicitly in the pseudocode:
        //     Upon receiving the block for round r and 2f + 1 votes for this block, form a QC,
        //     and execute Advance Round, Lock, and then Commit.
        upon [self.block[self.r_cur].is_some() && self.votes[self.r_cur].len() >= self.quorum()] {
            let block = self.block[self.r_cur].as_ref().unwrap().clone();

            let votes = &mut self.votes[block.round];
            votes.sort();
            let stored_prefix = votes[votes.len() - self.config.storage_requirement];

            let qc = QC {
                round: block.round,
                prefix: stored_prefix,
                block: Arc::new(block),
            };

            self.advance_round(Some(&qc), None);
            self.lock(&qc);
            self.commit(&qc);
        };

        // When the timer expires, the replica stops voting for round r_cur and multicasts
        // a signed timeout message <{r_cur}_i, qc_high> where {r_cur}_i
        // is a threshold signature share.
        upon timer [TimerEvent::Timeout(round)] {
            if round == self.r_cur {
                // Advancing r_vote makes the replica stop voting for round r_cur without
                // any other side effects.
                self.r_vote = round;
                ctx.multicast(Message::Timeout(round, self.qc_high.clone())).await;
            }
        };

        // Upon receiving a valid timeout message or TC,
        // execute Advance Round, Lock, and then Commit.
        upon receive [Message::Timeout(round, qc)] from [_any_node] {
            self.timeout_votes[round] += 1;

            // Upon receiving 2f+1 timeouts, form a TC.
            if self.timeout_votes[round] == self.quorum() {
                self.tc = TC {
                    round,
                    highest_qc: max(self.qc_high.clone(), qc.clone()),
                };
            }

            self.advance_round(Some(&qc), Some(&self.tc.clone()));
            self.lock(&qc);
            self.commit(&qc);
        };

        upon receive [Message::TimeoutCert(_round, tc)] from [_any_node] {
            // We need to remember the highest received tc for other steps of the protocol.
            if tc.round > self.tc.round {
                self.tc = tc.clone();
            }

            self.advance_round(None, Some(&tc));
            self.lock(&tc.highest_qc);
            self.commit(&tc.highest_qc);
        };

        // Logging and halting

        upon start {
            self.log_detail("Started".to_string());
            ctx.set_timer(self.config.end_of_run - Instant::now(), TimerEvent::EndOfRun);
        };

        upon timer [TimerEvent::EndOfRun] {
            ctx.halt();
        };
    }
}
