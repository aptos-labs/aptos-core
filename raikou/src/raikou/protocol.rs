use crate::{
    framework::{ContextFor, NodeId, Protocol},
    leader_schedule::LeaderSchedule,
    metrics,
    metrics::Sender,
    protocol,
    raikou::{
        dissemination::DisseminationLayer,
        penalty_tracker::{PenaltyTracker, PenaltyTrackerReportEntry},
        types::*,
    },
    utils::kth_max_set::KthMaxSet,
};
use bitvec::vec::BitVec;
use defaultmap::DefaultBTreeMap;
use itertools::Itertools;
use std::{
    cmp::{max, max_by, max_by_key, min, Ordering},
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{Debug, Formatter},
    sync::Arc,
    time::Duration,
};
use tokio::time::Instant;

impl Debug for BatchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ node: {}, sn: {} }}", self.node, self.sn)
    }
}

#[derive(Clone)]
pub struct Block {
    pub round: Round,
    pub payload: Payload,
    pub parent_qc: Option<QC>, // `None` only for the genesis block.
    pub reason: RoundEnterReason,
}

impl Block {
    pub fn genesis() -> Self {
        Block {
            round: 0,
            payload: Payload::empty(),
            parent_qc: None,
            reason: RoundEnterReason::Genesis,
        }
    }

    pub fn n_batches(&self) -> usize {
        self.payload.batches().len()
    }

    pub fn batch(&self, index: usize) -> BatchInfo {
        self.payload.batches()[index]
    }

    pub fn acs(&self) -> &Vec<AC> {
        self.payload.acs()
    }

    pub fn batches(&self) -> &Vec<BatchInfo> {
        self.payload.batches()
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }

    /// A non-genesis block is considered valid if:
    /// 1. It contains a valid multi-signature (omitted in the prototype);
    /// 2. It contains a valid parent QC;
    /// 3. At least one of the three conditions hold:
    ///    - `parent_qc.round == round - 1` and `parent_qc.prefix == parent_qc.n_batches`;
    ///    - `cc` is not None, `cc.round == round - 1`, and `parent_qc.id() >= cc.highest_qc_id()`.
    ///    - `tc` is not None, `cc.round == round - 1`, and `parent_qc.id() >= tc.highest_qc_id()`.
    pub fn is_valid(&self) -> bool {
        if self.is_genesis() {
            return true;
        }

        let Some(parent_qc) = &self.parent_qc else {
            return false;
        };

        match &self.reason {
            RoundEnterReason::Genesis => false, // Should not be used in a non-genesis block.
            RoundEnterReason::FullPrefixQC => {
                parent_qc.round == self.round - 1 && parent_qc.prefix == parent_qc.n_batches
            },
            RoundEnterReason::CC(cc) => {
                cc.round == self.round - 1 && parent_qc.sub_block_id() >= cc.highest_qc_id()
            },
            RoundEnterReason::TC(tc) => {
                tc.round == self.round - 1 && parent_qc.sub_block_id() >= tc.highest_qc_id()
            },
        }
    }
}

#[derive(Clone)]
pub struct QC {
    round: Round,
    prefix: Prefix,
    n_batches: Prefix,

    // In practice, this would be a hash pointer.
    block: Arc<Block>,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SubBlockId {
    round: Round,
    prefix: Prefix,
}

impl SubBlockId {
    pub fn new(round: Round, prefix: Prefix) -> Self {
        SubBlockId { round, prefix }
    }

    pub fn genesis() -> Self {
        SubBlockId::new(0, 0)
    }
}

impl From<(Round, Prefix)> for SubBlockId {
    fn from(tuple: (Round, Prefix)) -> Self {
        let (round, prefix) = tuple;
        SubBlockId { round, prefix }
    }
}

impl QC {
    pub fn genesis() -> Self {
        QC {
            round: 0,
            prefix: 0,
            n_batches: 0,
            block: Arc::new(Block::genesis()),
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }

    pub fn sub_block_id(&self) -> SubBlockId {
        (self.round, self.prefix).into()
    }
}

impl PartialEq for QC {
    fn eq(&self, other: &Self) -> bool {
        self.sub_block_id() == other.sub_block_id()
    }
}

impl Eq for QC {}

impl PartialOrd for QC {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sub_block_id().partial_cmp(&other.sub_block_id())
    }
}

impl Ord for QC {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sub_block_id().cmp(&other.sub_block_id())
    }
}

#[derive(Clone)]
pub struct CC {
    round: Round,
    min_prefix: Prefix,
    max_prefix: Prefix,
    // NB: a real implementation should include votes and a multisignature.
    // votes: Vec<Option<Prefix>>,
    // multisig: Multisig,
}

impl CC {
    pub fn new(round: Round, votes: &BTreeSet<(QC, NodeId)>) -> Self {
        CC {
            round,
            min_prefix: votes.iter().map(|(qc, _)| qc.prefix).min().unwrap(),
            max_prefix: votes.iter().map(|(qc, _)| qc.prefix).max().unwrap(),
        }
    }

    pub fn genesis() -> Self {
        CC {
            round: 0,
            min_prefix: 0,
            max_prefix: 0,
        }
    }

    pub fn lowest_qc_id(&self) -> SubBlockId {
        (self.round, self.min_prefix).into()
    }

    pub fn highest_qc_id(&self) -> SubBlockId {
        (self.round, self.max_prefix).into()
    }
}

#[derive(Clone)]
pub struct TC {
    round: Round,
    max_vote: SubBlockId,
    // NB: a real implementation should include votes and a multisignature.
    // votes: Vec<Option<QCId>>,
    // multisig: Multisig,
}

impl TC {
    pub fn genesis() -> Self {
        TC {
            round: 0,
            max_vote: (0, 0).into(),
        }
    }

    pub fn new(round: Round, votes: &BTreeMap<NodeId, SubBlockId>) -> TC {
        TC {
            round,
            max_vote: votes.into_iter().map(|(_node, &vote)| vote).max().unwrap(),
        }
    }

    pub fn highest_qc_id(&self) -> SubBlockId {
        self.max_vote
    }
}

#[derive(Clone)]
pub enum RoundEnterReason {
    /// Special case for the genesis block.
    Genesis,
    /// When a node receives a QC for the full prefix of round r, it enters round r+1.
    FullPrefixQC,
    /// When a node receives a CC for round r, it enters round r+1.
    CC(CC),
    /// When a node receives a TC for round r, it enters round r+1.
    TC(TC),
}

impl Debug for RoundEnterReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RoundEnterReason::Genesis => write!(f, "Genesis"),
            RoundEnterReason::FullPrefixQC => write!(f, "Full Prefix QC"),
            RoundEnterReason::CC(cc) => write!(f, "CC({})", cc.round),
            RoundEnterReason::TC(tc) => write!(f, "TC({})", tc.round),
        }
    }
}

#[derive(Clone)]
pub enum Message {
    // Consensus
    Block(Block),
    QcVote(Round, Prefix),
    CommitVote(QC),
    Timeout(Round, QC),
    AdvanceRound(Round, QC, RoundEnterReason),

    // Other
    PenaltyTrackerReport(Round, Vec<PenaltyTrackerReportEntry>),
}

#[derive(Clone)]
pub enum TimerEvent {
    // Consensus
    QcVote(Round),
    Timeout(Round),

    // Other
    EndOfRun,
}

#[derive(Clone, Debug)]
pub enum CommitReason {
    CC,
    TwoChainRule,
    Indirect,
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
    pub enable_commit_votes: bool,
    pub enable_optimistic_dissemination: bool,
    pub enable_round_entry_permission: bool,

    /// The time validator waits after receiving a block before voting for a QC for it
    /// if it doesn't have all the batches yet.
    pub extra_wait_before_qc_vote: Duration,
    pub enable_penalty_system: bool,

    pub end_of_run: Instant,
}

impl<S: LeaderSchedule> Config<S> {
    fn leader(&self, round: Round) -> NodeId {
        (self.leader_schedule)(round)
    }

    fn quorum(&self) -> usize {
        // Using more general quorum formula that works not only for n = 3f+1,
        // but for any n >= 3f+1.
        (self.n_nodes + self.f) / 2 + 1
    }
}

pub struct Metrics {
    pub batch_commit_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
}

pub struct RaikouNode<S, DL> {
    node_id: NodeId,
    config: Config<S>,
    dissemination: DL,

    // Logging and metrics
    start_time: Instant,
    detailed_logging: bool,
    metrics: Metrics,

    // Protocol state for the pseudocode
    r_ready: Round,                 // The highest round the node is ready to enter.
    enter_reason: RoundEnterReason, // The justification for entering the round r_read.
    r_allowed: Round,               // The highest round the node is allowed to enter.
    r_cur: Round,                   // The current round the node is in.
    r_timeout: Round,               // The highest round the node has voted to time out.
    last_qc_vote: SubBlockId,
    last_commit_vote: SubBlockId,
    qc_high: QC,
    cc_high: CC,
    tc_high: TC,
    committed_qc: QC,
    penalty_tracker: PenaltyTracker<S>,

    // For multichain integration

    // Additional variables necessary for an efficient implementation
    blocks: BTreeMap<Round, Block>,
    stored_prefix_cache: (Round, Prefix),
    // In practice, all votes should also include a signature.
    // In this prototype, signatures are omitted.
    qc_votes: DefaultBTreeMap<Round, BTreeMap<NodeId, Prefix>>,
    received_cc_vote: DefaultBTreeMap<Round, BTreeSet<NodeId>>,
    cc_votes: DefaultBTreeMap<Round, KthMaxSet<(QC, NodeId)>>,
    tc_votes: DefaultBTreeMap<Round, BTreeMap<NodeId, SubBlockId>>,
}

impl<S: LeaderSchedule, DL: DisseminationLayer> RaikouNode<S, DL> {
    pub fn new(
        id: NodeId,
        config: Config<S>,
        dissemination: DL,
        start_time: Instant,
        detailed_logging: bool,
        metrics: Metrics,
    ) -> Self {
        let quorum = config.quorum();

        RaikouNode {
            node_id: id,
            config: config.clone(),
            dissemination,
            start_time,
            detailed_logging,
            metrics,
            r_ready: 0,
            r_allowed: 0,
            enter_reason: RoundEnterReason::Genesis,
            r_cur: 0,
            last_qc_vote: (0, 0).into(),
            last_commit_vote: (0, 0).into(),
            r_timeout: 0,
            qc_high: QC::genesis(),
            cc_high: CC::genesis(),
            tc_high: TC::genesis(),
            committed_qc: QC::genesis(),
            penalty_tracker: PenaltyTracker::new(config),
            blocks: Default::default(),
            stored_prefix_cache: (0, 0),
            qc_votes: Default::default(),
            received_cc_vote: Default::default(),
            cc_votes: DefaultBTreeMap::new(KthMaxSet::new(quorum)),
            tc_votes: Default::default(),
        }
    }

    async fn on_new_qc(&mut self, new_qc: &QC, ctx: &mut impl ContextFor<Self>) {
        // Upon receiving a new highest QC, update qc_high and check the 2-chain commit rule.
        if new_qc <= &self.qc_high {
            return;
        }

        // Update qc_high.
        self.qc_high = new_qc.clone();

        // Two-chain commit rule:
        // If there exists two adjacent certified blocks B and B' in the chain with consecutive
        // round numbers, i.e., B'.r = B.r + 1, the replica commits B and all its ancestors.
        if let Some(parent_qc) = new_qc.block.parent_qc.as_ref() {
            if new_qc.round == parent_qc.round + 1 {
                self.commit_qc(parent_qc, CommitReason::TwoChainRule);
            }
        }

        // If new_qc.round > r_commit_vote and new_qc.round > r_timeout,
        // multicast a commit vote and update r_commit_vote.
        if self.config.enable_commit_votes {
            if new_qc.round > self.last_commit_vote.round && new_qc.round > self.r_timeout {
                self.last_commit_vote = new_qc.sub_block_id();
                ctx.multicast(Message::CommitVote(new_qc.clone())).await;
            }
        }

        if new_qc.prefix == new_qc.n_batches {
            // If form or receive a qc for the largest possible prefix of a round,
            // advance to the next round after that.
            self.advance_r_ready(new_qc.round + 1, RoundEnterReason::FullPrefixQC, ctx)
                .await;
        }
    }

    async fn advance_r_ready(
        &mut self,
        round: Round,
        reason: RoundEnterReason,
        ctx: &mut impl ContextFor<Self>,
    ) {
        if round > self.r_ready {
            self.r_ready = round;
            self.enter_reason = reason.clone();

            // Upon getting a justification to enter a higher round,
            // send it to the leader of that round.
            // NB: consider broadcasting to all the nodes instead.
            ctx.unicast(
                Message::AdvanceRound(round, self.qc_high.clone(), reason),
                self.config.leader(round),
            )
            .await;
        }
    }

    async fn stored_prefix(&mut self) -> Prefix {
        debug_assert!(self.blocks.contains_key(&self.r_cur));
        let block = &self.blocks[&self.r_cur];

        if self.stored_prefix_cache.0 != self.r_cur {
            self.stored_prefix_cache = (self.r_cur, 0);
        }

        while self.stored_prefix_cache.1 < block.n_batches()
            && self
                .dissemination
                .check_stored(&block.batch(self.stored_prefix_cache.1))
                .await
        {
            self.stored_prefix_cache.1 += 1;
        }

        self.stored_prefix_cache.1
    }

    /// Returns the number of full-prefix votes in `round` if received the block for `round`
    /// and `0` otherwise.
    fn n_full_prefix_votes(&self, round: Round) -> usize {
        if let Some(block) = self.blocks.get(&round) {
            // TODO: This can be optimized by tracking the number
            //       of full-prefix votes as they arrive.
            self.qc_votes[round]
                .values()
                .filter(|&&vote| vote == block.n_batches())
                .count()
        } else {
            0
        }
    }

    async fn commit_qc(&mut self, qc: &QC, commit_reason: CommitReason) {
        let committed = self.commit_qc_impl(qc, commit_reason);
        self.dissemination.notify_commit(committed).await;
    }

    fn commit_qc_impl(&mut self, qc: &QC, commit_reason: CommitReason) -> Vec<Payload> {
        if *qc <= self.committed_qc {
            return vec![];
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
        let mut res =
            self.commit_qc_impl(qc.block.parent_qc.as_ref().unwrap(), CommitReason::Indirect);

        // Then, commit the transactions of this block.

        if qc.round == self.committed_qc.round {
            assert!(qc.prefix > self.committed_qc.prefix);
            self.log_detail(format!(
                "Extending the prefix of committed block {}: {} -> {} / {}{} ({:?})",
                qc.round,
                self.committed_qc.prefix,
                qc.prefix,
                qc.block.n_batches(),
                if qc.prefix == qc.block.n_batches() {
                    " (full)"
                } else {
                    ""
                },
                commit_reason,
            ));
            let new_batches: Vec<BatchInfo> = qc
                .block
                .batches()
                .iter()
                .take(qc.prefix)
                .skip(self.committed_qc.prefix)
                .cloned()
                .collect();
            res.push(Payload::new(vec![], new_batches));
        } else {
            self.log_detail(format!(
                "Committing block {} proposed by node {} with prefix {}/{}{} ({:?})",
                qc.round,
                self.config.leader(qc.round),
                qc.prefix,
                qc.block.n_batches(),
                if qc.prefix == qc.block.n_batches() {
                    " (full)"
                } else {
                    ""
                },
                commit_reason,
            ));

            res.push(qc.block.payload.clone());
        }

        // Finally, update the committed QC variable.
        self.committed_qc = qc.clone();
        res
    }

    fn quorum(&self) -> usize {
        self.config.quorum()
    }

    fn time_in_delta(&self) -> f64 {
        (Instant::now() - self.start_time).as_secs_f64() / self.config.delta.as_secs_f64()
    }

    fn log_info(&self, msg: String) {
        log::info!(
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

impl<S, DL> Protocol for RaikouNode<S, DL>
where
    S: LeaderSchedule,
    DL: DisseminationLayer,
{
    type Message = Message;
    type TimerEvent = TimerEvent;

    protocol! {
        self: self;
        ctx: ctx;

        // Nodes start the protocol by entering round 1.
        upon start {
            self.advance_r_ready(1, RoundEnterReason::FullPrefixQC, ctx).await;
        };

        upon [
            self.r_cur < self.r_ready
            && (self.r_ready == self.r_allowed || !self.config.enable_round_entry_permission)
        ] {
            let round = self.r_ready;

            self.r_cur = round;

            self.log_detail(format!("Entering round {} by {:?}", round, self.enter_reason));

            if self.node_id == self.config.leader(round) {
                // Upon entering round r, the leader L_r multicasts a signed block
                // B = [r, parent_qc, cc, tc, acs, batches], where cc or tc is not ⊥
                // if the leader enters the round by forming or receiving a CC or TC
                // for round r-1 respectively.

                let block = Block {
                    round,
                    // TODO: implement deduplication.
                    payload: self.dissemination.pull_payload(HashSet::new()).await,
                    parent_qc: Some(self.qc_high.clone()),
                    reason: self.enter_reason.clone(),
                };
                self.blocks.insert(round, block.clone());

                ctx.multicast(Message::Block(block)).await;
            }

            // Upon entering round r, the node starts a timer for leader timeout.
            let timeout = self.config.leader_timeout * self.config.delta;
            ctx.set_timer(timeout, TimerEvent::Timeout(round));
        };

        // Upon receiving a valid block B = [r, parent_qc, cc, tc, acs, batches] from L_r
        // for the first time, if r >= r_cur and r > r_timeout, store the block,
        // execute on_new_qc and advance_round, start a timer for qc-vote,
        // and report missing batches to the leader.
        upon receive [Message::Block(block)] from [leader] {
            if
                block.is_valid()
                && leader == self.config.leader(block.round)
                && (leader == self.node_id || !self.blocks.contains_key(&block.round))
                && block.round >= self.r_cur
                && block.round > self.r_timeout
            {
                self.blocks.insert(block.round, block.clone());

                let Block { round, payload, parent_qc, reason, .. } = block;
                // a valid non-genesis block, by definition, always has a parent QC.
                let parent_qc = parent_qc.unwrap();

                self.on_new_qc(&parent_qc, ctx).await;
                self.advance_r_ready(round, reason, ctx).await;

                ctx.set_timer(self.config.extra_wait_before_qc_vote, TimerEvent::QcVote(round));

                // send the penalty tracker reports
                if self.config.enable_penalty_system {
                    let reports = self.penalty_tracker.prepare_reports(
                        payload.batches(),
                        Instant::now(),
                    );
                    ctx.unicast(Message::PenaltyTrackerReport(round, reports), leader).await;
                }

                // store the ACs
                self.dissemination.prefetch_payload_data(payload).await;
            }
        };

        // The leader uses the missing votes to compute the time penalty for each validator.
        upon receive [Message::PenaltyTrackerReport(round, reports)] from node [p] {
            if self.config.enable_penalty_system {
                self.penalty_tracker.register_reports(round, p, reports);
            }
        };

        // A node issues a qc-vote in its current round r_cur up to 2 times:
        // 1. After a timeout after receiving the block,
        //    if not yet voted in this or greater round.
        // 2. When received all optimistically proposed batches.
        //
        // A node only qc-votes if r_cur > r_timeout.

        upon timer [TimerEvent::QcVote(round)] {
            if round == self.r_cur && self.last_qc_vote.round < round && round > self.r_timeout {
                let stored_prefix = self.stored_prefix().await;
                self.last_qc_vote = (self.r_cur, stored_prefix).into();
                ctx.multicast(Message::QcVote(self.r_cur, stored_prefix)).await;
            }
        };

        upon [
            self.r_cur > self.r_timeout
            && self.blocks.contains_key(&self.r_cur)
            && {
                let stored_prefix = self.stored_prefix().await;

                stored_prefix == self.blocks[&self.r_cur].n_batches()
                && self.last_qc_vote < (self.r_cur, stored_prefix).into()
            }
        ] {
            let stored_prefix = self.stored_prefix().await;
            self.last_qc_vote = (self.r_cur, stored_prefix).into();
            ctx.multicast(Message::QcVote(self.r_cur, stored_prefix)).await;
        };

        // Upon receiving the block for round r_cur and a quorum of qc-votes for this block,
        // form a QC and execute on_new_qc if one of the two conditions hold:
        // 1. When it will be the first QC observed by the node in this round;
        // 2. When it will be the first full-prefix QC observed by the node in this round.

        upon receive [Message::QcVote(round, prefix)] from node [p] {
            self.qc_votes[round].insert(p, prefix);
        };

        upon [
            self.blocks.contains_key(&self.r_cur)
            && self.qc_votes[self.r_cur].len() >= self.quorum()
            && (
                self.qc_high.round < self.r_cur
                || (
                    self.blocks.contains_key(&self.r_cur)
                    && self.qc_high.sub_block_id() < (self.r_cur, self.blocks[&self.r_cur].n_batches()).into()
                    && self.n_full_prefix_votes(self.r_cur) >= self.config.storage_requirement
                )
            )
        ] {
            let block = &self.blocks[&self.r_cur];

            let mut votes = self.qc_votes[block.round].values().copied().collect_vec();
            votes.sort();
            let certified_prefix = votes[votes.len() - self.config.storage_requirement];

            let qc = QC {
                round: block.round,
                prefix: certified_prefix,
                n_batches: block.n_batches(),
                block: Arc::new(block.clone()),
            };

            self.on_new_qc(&qc, ctx).await;
        };

        // Upon receiving a commit vote for a round-r qc from a node for the
        // first time, store it and execute on_new_qc.
        // Upon having gathered a quorum of commit votes, form a CC,
        // commit the smallest prefix, and execute advance_round.
        upon receive [Message::CommitVote(qc)] from node [p] {
            if !self.received_cc_vote[qc.round].contains(&p) {
                self.on_new_qc(&qc, ctx).await;

                self.received_cc_vote[qc.round].insert(p);
                self.cc_votes[qc.round].insert((qc.clone(), p));

                if let Some((committed_qc, _)) = self.cc_votes[qc.round].kth_max() {
                    // Form a CC each time we can commit something new, possibly several
                    // times for the same round.
                    if *committed_qc > self.committed_qc {
                        let committed_qc = committed_qc.clone();
                        self.commit_qc(&committed_qc, CommitReason::CC).await;
                        let cc = CC::new(qc.round, &self.cc_votes[qc.round].k_max_set());
                        assert_eq!(cc.lowest_qc_id(), self.committed_qc.sub_block_id());
                        self.advance_r_ready(qc.round + 1, RoundEnterReason::CC(cc), ctx).await;
                    }
                }
            }
        };

        // When the timeout expires, multicast a signed timeout message
        // with qc_high attached.
        upon timer [TimerEvent::Timeout(round)] {
            if round == self.r_cur {
                self.r_timeout = round;
                ctx.multicast(Message::Timeout(round, self.qc_high.clone())).await;
            }
        };

        // Upon receiving a valid timeout message, execute on_new_qc.
        // Upon gathering a quorum of matching timeout messages,
        // form the TC and execute advance_round.
        upon receive [Message::Timeout(round, qc)] from node [p] {
            self.tc_votes[round].insert(p, qc.sub_block_id());
            self.on_new_qc(&qc, ctx).await;

            if self.tc_votes[round].len() == self.quorum() {
                let tc = TC::new(round, &self.tc_votes[round]);
                self.advance_r_ready(round + 1, RoundEnterReason::TC(tc), ctx).await;
            }
        };

        // Upon receiving an AdvanceRound message, execute on_new_qc and advance_round.
        upon receive [Message::AdvanceRound(round, qc, reason)] from [_any_node] {
            self.on_new_qc(&qc, ctx).await;
            self.advance_r_ready(round, reason, ctx).await;
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
