use crate::{
    framework::{NodeId, Protocol},
    leader_schedule::LeaderSchedule,
    protocol,
};
use defaultmap::DefaultBTreeMap;
use std::{
    cmp::{max, max_by, max_by_key, Ordering},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::mpsc, time::Instant};

pub type Round = i64; // Round number.

#[derive(Clone)]
pub struct Block<T> {
    round: Round,
    txn: Option<T>,

    // In practice, this would be the hash of the previous block.
    // `None` only for the genesis block.
    qc: Option<QC<T>>,

    // Can be `None` in non-genesis blocks.
    tc: Option<TC<T>>,
}

impl<T> Block<T> {
    pub fn genesis() -> Self {
        Block {
            round: 0,
            txn: None,
            qc: None,
            tc: None,
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }
}

#[derive(Clone)]
pub struct QC<T> {
    round: Round,

    // In practice, this would be a hash pointer.
    block: Arc<Block<T>>,
}

impl<T> PartialEq for QC<T> {
    fn eq(&self, other: &Self) -> bool {
        self.round == other.round
    }
}

impl<T> Eq for QC<T> {}

impl<T> PartialOrd for QC<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.round.partial_cmp(&other.round)
    }
}

impl<T> Ord for QC<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.round.cmp(&other.round)
    }
}

impl<T> QC<T> {
    pub fn genesis() -> Self {
        QC {
            round: 0,
            block: Arc::new(Block::genesis()),
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.round == 0
    }
}

#[derive(Clone)]
pub struct TC<T> {
    round: Round,
    highest_qc: QC<T>,
}

impl<T> TC<T> {
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
    // Addition of TC is a fix
    Block(Block<T>),
    Vote(Round),
    Timeout(Round, QC<T>),
    TimeoutCert(Round, TC<T>),
}

#[derive(Clone)]
pub enum TimerEvent {
    Timeout(Round),
    EndOfRun,
}

#[derive(Clone, Copy)]
pub struct Config<S> {
    pub n_nodes: usize,
    pub f: usize,
    pub leader_timeout: u32, // in deltas
    pub leader_schedule: S,
    pub delta: Duration,

    pub end_of_run: Instant,
}

pub struct JolteonNode<T: Clone, S> {
    node_id: NodeId,
    config: Config<S>,
    txns: mpsc::Receiver<T>,

    // Logging and metrics
    start_time: Instant,
    detailed_logging: bool,
    // metrics: Metrics,

    // Protocol state as in pseudocode
    r_vote: Round,
    r_cur: Round,
    qc_high: QC<T>,

    // Additional variables for the implementation
    r_entered: Round,
    tc: TC<T>,
    committed_block: Block<T>,
    block: DefaultBTreeMap<Round, Option<Block<T>>>,
    votes: DefaultBTreeMap<Round, usize>,
    timeout_votes: DefaultBTreeMap<Round, usize>,
}

impl<T: Clone, S> JolteonNode<T, S> {
    pub fn new(
        id: NodeId,
        config: Config<S>,
        txns: mpsc::Receiver<T>,
        start_time: Instant,
        detailed_logging: bool,
    ) -> Self {
        JolteonNode {
            node_id: id,
            config,
            txns,
            start_time,
            detailed_logging,
            r_vote: 0,
            r_cur: 0,
            qc_high: QC::genesis(),
            r_entered: 0,
            tc: TC::genesis(),
            committed_block: Block::genesis(),
            block: Default::default(),
            votes: Default::default(),
            timeout_votes: Default::default(),
        }
    }

    fn advance_round(&mut self, new_qc: Option<&QC<T>>, new_tc: Option<&TC<T>>) {
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

    fn lock(&mut self, new_qc: &QC<T>) {
        // Upon seeing a valid qc (formed by votes or contained in proposal or timeouts),
        // the replica updates qc_high ← max(qc_high, qc).
        self.qc_high = max(self.qc_high.clone(), new_qc.clone());
    }

    fn commit(&mut self, new_qc: &QC<T>) {
        // If there exists two adjacent certified blocks B, B' in the chain with consecutive
        // round numbers, i.e., B'.r = B.r + 1, the replica commits B and all its ancestors.
        if new_qc.is_genesis() {
            return;
        }

        let b_prime = &new_qc.block;
        let b = &b_prime.qc.as_ref().unwrap().block;
        if b.round == b_prime.round - 1 {
            self.commit_block((**b).clone());
        }
    }

    fn commit_block(&mut self, block: Block<T>) {
        if block.round <= self.committed_block.round {
            return;
        }

        self.log_detail(format!("Committing block {}", block.round));

        // To verify safety, search for the last committed block in the parents of this block.
        let mut qc = block.qc.as_ref().unwrap();
        while !qc.is_genesis() && qc.block.round > self.committed_block.round {
            qc = qc.block.qc.as_ref().unwrap();
        }

        if qc.block.round != self.committed_block.round {
            panic!(
                "Safety violation: committed block {} not in the chain of the new block {}",
                self.committed_block.round, block.round
            );
        }

        self.committed_block = block;
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

impl<T, S> Protocol for JolteonNode<T, S>
where
    T: Clone + Send + Sync,
    S: LeaderSchedule,
{
    type Message = Message<T>;
    type TimerEvent = TimerEvent;

    // The implementation follows the pseudocode as closely as possible, with the only exception
    // that all signatures are omitted.
    protocol! {
        self: self;
        ctx: ctx;

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
                    txn: self.txns.try_recv().ok(),
                    qc: Some(self.qc_high.clone()),
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
                let Block {round, txn: _, qc,tc} = block;
                let qc = qc.unwrap();
                self.advance_round(Some(&qc), tc.as_ref());
                self.lock(&qc);
                self.commit(&qc);

                // If r = r_cur, v = v_cur, r > r_vote and ((1) r = qc.r + 1, or (2) r = tc.r + 1
                // and qc.r ≥ max{qc_high.r | qc_high ∈ tc}), vote for B by sending the
                // threshold signature share {id, r, v}_i to L_{r+1}, and update r_vote ← r.
                let valid_tc = match tc {
                    Some(tc) => round == tc.round + 1 && qc >= tc.highest_qc,
                    None => false,
                };

                if round == self.r_cur && round > self.r_vote && (round == qc.round + 1 || valid_tc) {
                    let next_leader = (self.config.leader_schedule)(round + 1);
                    ctx.unicast(Message::Vote(round), next_leader).await;
                    self.r_vote = round;
                }
            }
        };

        upon receive [Message::Vote(round)] from [_any_node] {
            self.votes[round] += 1;
        };

        // Not explicitly in the pseudocode:
        //     Upon receiving the block for round r and 2f + 1 votes for this block, form a QC,
        //     and execute Advance Round, Lock, and then Commit.
        upon [self.block[self.r_cur].is_some() && self.votes[self.r_cur] >= self.quorum()] {
            let block = self.block[self.r_cur].as_ref().unwrap().clone();
            let qc = QC {
                round: block.round,
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
