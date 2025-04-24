use std::{cmp::max, sync::Arc, time::Duration};

use defaultmap::DefaultBTreeMap;
use tokio::time::Instant;

use crate::{
    framework::{context::WrappedContext, ContextFor, NodeId, Protocol},
    leader_schedule::LeaderSchedule,
    metrics,
    metrics::Sender,
    protocol, raikou,
    raikou::{RaikouNode, types::Txn},
    Slot,
};

type ChainId = usize;

#[derive(Clone)]
pub enum Message {
    Entering(Slot),
    RaikouMessage(ChainId, raikou::Message),
}

pub enum TimerEvent {
    FixedTimer(Slot),
    AdaptiveTimer(Slot),
    LogStatus,
    RaikouTimerEvent(ChainId, raikou::TimerEvent),
}

#[derive(Clone, Copy)]
pub struct Config<S> {
    pub n_nodes: usize,
    // n_chains = slots_per_delta * leader_timeout
    pub slots_per_delta: Slot,
    pub leader_timeout: u32, // in deltas
    pub delta: Duration,
    pub progress_threshold: f64,
    pub slot_duration_sample_size: Slot,
    pub responsive: bool,
    pub adaptive_timer: bool,

    pub raikou_config: raikou::Config<S>,
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
    pub indirectly_committed_slots: Option<metrics::UnorderedSender<Slot>>,
}

pub struct MultiChainRaikou<S> {
    node_id: NodeId,
    config: Config<S>,
    chains: Vec<RaikouNode<S>>,

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

    committed_prefix_of_slots: Slot,
}

impl<S: LeaderSchedule> MultiChainRaikou<S> {
    pub fn new_node(
        node_id: NodeId,
        config: Config<S>,
        start_time: Instant,
        detailed_logging: bool,
        metrics: Metrics,
    ) -> Self {
        let chains = (1..=config.n_chains())
            .map(|_| {
                RaikouNode::new(
                    node_id,
                    config.raikou_config.clone(),
                    start_time,
                    detailed_logging,
                    raikou::Metrics {
                        batch_commit_time: None,
                    },
                )
            })
            .collect();

        MultiChainRaikou {
            node_id,
            config,
            chains,

            // Logging and metrics
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
            committed_prefix_of_slots: 0,
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

    fn progress_condition(&self) -> bool {
        false

        // TODO
        // if !self.config.responsive {
        //     // This makes sure that the responsive condition for advancing slots will never be met.
        //     // Hence, the pacing will be determined by the non-responsive path.
        //     return false;
        // }
        //
        // let next_slot = self.slot + 1;
        // let first_uncommitted = self.committed_prefix_of_slots + 1;
        //
        // // Fall back to non-responsive when 2 consecutive leaders
        // // in the same chain failed to make progress.
        // if next_slot >= first_uncommitted + 2 * (self.n_chains() as Slot) + 1 {
        //     return false;
        // }
        //
        // let sent_prepare = (2 * self.config.slots_per_delta..3 * self.config.slots_per_delta)
        //     .filter(|i| self.sent_prepare[next_slot - i])
        //     .count();
        // let sent_commit = (3 * self.config.slots_per_delta..4 * self.config.slots_per_delta)
        //     .filter(|i| self.sent_commit[next_slot - i])
        //     .count();
        // let committed = (4 * self.config.slots_per_delta..5 * self.config.slots_per_delta)
        //     .filter(|i| self.committed[next_slot - i])
        //     .count();
        //
        // let progress_score = (sent_prepare + sent_commit + committed) as f64
        //     / (3 * self.config.slots_per_delta) as f64;
        //
        // let res = progress_score >= self.config.progress_threshold;
        //
        // if res {
        //     self.log_detail(format!(
        //         "progress condition met: sent_prepare={}, sent_commit={}, committed={}, score={:.2}",
        //         sent_prepare, sent_commit, committed, progress_score,
        //     ));
        // }
        //
        // res
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
}

impl<S: LeaderSchedule> Protocol for MultiChainRaikou<S> {
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
                // TODO
                // self.sent_prepare[slot] = true;
                // self.sent_commit[slot] = true;
                // self.committed[slot] = true;
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

        // Forwarding chain messages and timer events

        upon receive [Message::RaikouMessage(chain, msg)] from node [p] {
            let mut ctx = WrappedContext::new(
                ctx,
                |msg| Message::RaikouMessage(chain, msg),
                |event| TimerEvent::RaikouTimerEvent(chain, event),
            );
            self.chains[chain].message_handler(&mut ctx, p, msg).await;
        };

        upon timer event [TimerEvent::RaikouTimerEvent(chain, event)] {
            let mut ctx = WrappedContext::new(
                ctx,
                |msg| Message::RaikouMessage(chain, msg),
                |event| TimerEvent::RaikouTimerEvent(chain, event),
            );
            self.chains[chain].timer_event_handler(&mut ctx, event).await;
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
                self.committed_prefix_of_slots,
                // self.known_batch[self.node_id],
                // self.known_ac[self.node_id],
                // self.committed_batch[self.node_id],
                self.fixed_timer_elapsed[self.slot],
            ));
            ctx.set_timer(Duration::from_secs(5), TimerEvent::LogStatus);
        };
    }
}
