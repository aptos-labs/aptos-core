// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::fetch_metadata::ValidatorInfo;
use anyhow::Result;
use aptos_bitvec::BitVec;
use aptos_logger::error;
use aptos_rest_client::VersionedNewBlockEvent;
use aptos_storage_interface::{DbReader, Order};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
};
use itertools::Itertools;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, VecDeque},
    convert::TryFrom,
    ops::Add,
    time::Duration,
};

/// Single validator stats
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ValidatorStats {
    /// Number of successful proposals
    pub proposal_successes: u32,
    /// Number of failed proposals
    pub proposal_failures: u32,
    /// Number of votes proposals
    pub votes: u32,
    /// Number of transactions in a block
    pub transactions: u32,
    /// Voting power
    pub voting_power: u64,
}

impl ValidatorStats {
    /// Proposal failure rate
    pub fn failure_rate(&self) -> f32 {
        (self.proposal_failures as f32) / (self.proposal_failures + self.proposal_successes) as f32
    }

    /// Whether node is proposing well enough
    pub fn is_reliable(&self) -> bool {
        (self.proposal_successes > 0) && (self.failure_rate() < 0.1)
    }

    // Whether node is voting well enough
    pub fn is_voting_enough(&self, rounds: u32) -> bool {
        self.votes as f32 > rounds as f32 * 0.3
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum NodeState {
    // Proposal failure < 10%, >30% votes
    Reliable,
    // Proposal failure < 10%, <30% votes
    ReliableLowVotes,
    // Has successful proposals, but proposal failure > 10%
    AliveUnreliable,
    // No successful proposals, but voting
    OnlyVoting,
    // Not participating in consensus
    NotParticipatingInConsensus,
    // Not in ValidatorSet
    Absent,
}

impl NodeState {
    pub fn to_char(&self) -> &str {
        match self {
            Self::Reliable => "+",
            Self::ReliableLowVotes => "P",
            Self::AliveUnreliable => "~",
            Self::OnlyVoting => "V",
            Self::NotParticipatingInConsensus => "X",
            Self::Absent => " ",
        }
    }

    // Large the value, the worse the node is performing.
    pub fn to_order_weight(&self) -> usize {
        match self {
            Self::Reliable => 0,
            Self::ReliableLowVotes => 100,
            Self::AliveUnreliable => 10000,
            Self::OnlyVoting => 1000000,
            Self::NotParticipatingInConsensus => 100000000,
            Self::Absent => 1,
        }
    }
}

impl Add for ValidatorStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            proposal_successes: self.proposal_successes + other.proposal_successes,
            proposal_failures: self.proposal_failures + other.proposal_failures,
            votes: self.votes + other.votes,
            transactions: self.transactions + other.transactions,
            voting_power: 0, // cannot aggregate voting power.
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TpsInterval {
    Blocks(usize),
    Seconds(u32),
}

#[derive(Clone)]
pub struct MaxTpsStats {
    /// Max TPS
    pub tps: f32,
    /// End version of the interval at which Max TPS was achieved
    pub end_version: u64,
    /// Number of transactions in the interval at which Max TPS was achieved
    pub txns: u32,
    /// Number of blocks in which Max TPS was achieved
    pub blocks: usize,
    /// Duration of the interval in which Max TPS was achieved
    pub duration: f32,
}

/// Statistics for all validators
#[derive(Clone)]
pub struct EpochStats {
    /// Statistics for each of the validators
    pub validator_stats: HashMap<AccountAddress, ValidatorStats>,
    /// Total rounds in an epoch
    pub total_rounds: u32,
    /// Total transactions in an epoch
    pub total_transactions: u32,
    /// Successful rounds in an epoch
    pub round_successes: u32,
    /// Failed rounds in an epoch
    pub round_failures: u32,
    /// Nil blocks in an epoch
    pub nil_blocks: u32,
    /// Total voting power
    pub total_voting_power: u128,
    /// Max TPS per block interval
    pub max_tps_per_block_interval: BTreeMap<TpsInterval, MaxTpsStats>,
}

impl EpochStats {
    pub fn to_state(&self, validator: &AccountAddress) -> NodeState {
        self.validator_stats
            .get(validator)
            .map(|b| {
                if b.is_reliable() {
                    if b.is_voting_enough(self.total_rounds) {
                        NodeState::Reliable
                    } else {
                        NodeState::ReliableLowVotes
                    }
                } else if b.proposal_successes > 0 {
                    NodeState::AliveUnreliable
                } else if b.votes > 0 {
                    NodeState::OnlyVoting
                } else {
                    NodeState::NotParticipatingInConsensus
                }
            })
            .unwrap_or(NodeState::Absent)
    }

    pub fn to_votes(&self, validator: &AccountAddress) -> u32 {
        self.validator_stats
            .get(validator)
            .map(|s| s.votes)
            .unwrap_or(0)
    }

    pub fn to_voting_power(&self, validator: &AccountAddress) -> u64 {
        self.validator_stats
            .get(validator)
            .map(|s| s.voting_power)
            .unwrap_or(0)
    }
}

impl Add for EpochStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut validator_stats = self.validator_stats;
        for (key, other_validator_stats) in other.validator_stats.into_iter() {
            validator_stats.insert(
                key,
                other_validator_stats
                    + *validator_stats.get(&key).unwrap_or(&ValidatorStats {
                        proposal_failures: 0,
                        proposal_successes: 0,
                        votes: 0,
                        transactions: 0,
                        voting_power: 0,
                    }),
            );
        }
        Self {
            validator_stats,
            total_rounds: self.total_rounds + other.total_rounds,
            round_successes: self.round_successes + other.round_successes,
            round_failures: self.round_failures + other.round_failures,
            nil_blocks: self.nil_blocks + other.nil_blocks,
            total_transactions: self.total_transactions + other.total_transactions,
            total_voting_power: 0,
            max_tps_per_block_interval: self
                .max_tps_per_block_interval
                .into_iter()
                .map(|(k, v)| {
                    let other_v = other.max_tps_per_block_interval.get(&k).unwrap();
                    (
                        k,
                        if v.tps > other_v.tps {
                            v
                        } else {
                            other_v.clone()
                        },
                    )
                })
                .collect(),
        }
    }
}

pub struct RunningAvg {
    sum: f64,
    count: u64,
}

impl RunningAvg {
    fn empty() -> Self {
        Self { sum: 0.0, count: 0 }
    }

    fn observe(&mut self, value: f32) {
        self.sum += value as f64;
        self.count += 1;
    }

    pub fn get(&self) -> f64 {
        if self.count > 0 {
            self.sum / self.count as f64
        } else {
            0.0
        }
    }
}

pub struct GapSummary {
    pub max_gap: f32,
    at_version: u64,
    avg: RunningAvg,
}

impl GapSummary {
    fn empty() -> Self {
        Self {
            max_gap: 0.0,
            at_version: 0,
            avg: RunningAvg::empty(),
        }
    }

    fn observe(&mut self, gap: f32, version: u64) {
        if gap > self.max_gap {
            self.max_gap = gap;
            self.at_version = version;
        }
        self.avg.observe(gap)
    }

    pub fn to_string_as_round(&self) -> String {
        format!(
            "{} rounds at version {} (avg {:.2})",
            self.max_gap as u64,
            self.at_version,
            self.avg.get(),
        )
    }

    pub fn to_string_as_time(&self) -> String {
        format!(
            "{:.2}s no progress at version {} (avg {:.2}s)",
            self.max_gap,
            self.at_version,
            self.avg.get(),
        )
    }
}

pub struct MaxGapInfo {
    pub non_epoch_round_gap: GapSummary,
    pub epoch_round_gap: GapSummary,
    pub non_epoch_time_gap: GapSummary,
    pub epoch_time_gap: GapSummary,
}

/// Analyze validator performance
pub struct AnalyzeValidators {}

impl AnalyzeValidators {
    /// Fetch all events from a single epoch from DB.
    pub fn fetch_epoch(epoch: u64, aptos_db: &dyn DbReader) -> Result<Vec<VersionedNewBlockEvent>> {
        let batch = 100;

        let mut cursor = u64::MAX;
        let mut result: Vec<VersionedNewBlockEvent> = vec![];
        let ledger_version = aptos_db.get_latest_ledger_info()?.ledger_info().version();

        loop {
            let raw_events = aptos_db.get_events(
                &new_block_event_key(),
                cursor,
                Order::Descending,
                batch as u64,
                ledger_version,
            )?;
            let end = raw_events.len() < batch;
            for raw_event in raw_events {
                if cursor <= raw_event.event.v1()?.sequence_number() {
                    println!(
                        "Duplicate event found for {} : {:?}",
                        cursor,
                        raw_event.event.v1()?.sequence_number()
                    );
                } else {
                    cursor = raw_event.event.v1()?.sequence_number();
                    let event = bcs::from_bytes::<NewBlockEvent>(raw_event.event.event_data())?;

                    match epoch.cmp(&event.epoch()) {
                        Ordering::Equal => {
                            result.push(VersionedNewBlockEvent {
                                event,
                                version: raw_event.transaction_version,
                                sequence_number: raw_event.event.v1()?.sequence_number(),
                            });
                        },
                        Ordering::Greater => {
                            return Ok(result);
                        },
                        Ordering::Less => {},
                    };
                }
            }

            if end {
                return Ok(result);
            }
        }
    }

    /// Analyze single epoch
    pub fn analyze(blocks: &[VersionedNewBlockEvent], validators: &[ValidatorInfo]) -> EpochStats {
        assert!(
            validators.iter().as_slice().windows(2).all(|w| {
                w[0].validator_index
                    .partial_cmp(&w[1].validator_index)
                    .map(|o| o != Ordering::Greater)
                    .unwrap_or(false)
            }),
            "Validators need to be sorted"
        );
        assert!(
            blocks.iter().as_slice().windows(2).all(|w| {
                w[0].event
                    .round()
                    .partial_cmp(&w[1].event.round())
                    .map(|o| o != Ordering::Greater)
                    .unwrap_or(false)
            }),
            "Blocks need to be sorted"
        );

        let mut successes = HashMap::<AccountAddress, u32>::new();
        let mut failures = HashMap::<AccountAddress, u32>::new();
        let mut votes = HashMap::<AccountAddress, u32>::new();
        let mut transactions = HashMap::<AccountAddress, u32>::new();

        // because we measure based on block timestamp, we need interval larger than when backpressure
        // kicks in, to make sure we are measuring end-to-end throughput, not just ordering throughput.
        let mut max_tps_tuples = vec![
            TpsInterval::Blocks(15),
            TpsInterval::Blocks(30),
            TpsInterval::Blocks(45),
            TpsInterval::Blocks(60),
            TpsInterval::Seconds(60),
            TpsInterval::Seconds(120),
            TpsInterval::Seconds(180),
            TpsInterval::Seconds(300),
            TpsInterval::Seconds(600),
            TpsInterval::Seconds(1200),
            TpsInterval::Seconds(1800),
            TpsInterval::Seconds(3600),
        ]
        .into_iter()
        .map(|v| {
            (v, VecDeque::new(), MaxTpsStats {
                tps: 0.0,
                end_version: 0,
                txns: 0,
                blocks: 0,
                duration: 0.0,
            })
        })
        .collect::<Vec<_>>();
        let mut trimmed_rounds = 0;
        let mut nil_blocks = 0;
        let mut previous_round = 0;
        for (pos, block) in blocks.iter().enumerate() {
            let event = &block.event;
            let is_nil = event.proposer() == AccountAddress::ZERO;
            if is_nil {
                nil_blocks += 1;
            }
            let expected_round =
                previous_round + u64::from(!is_nil) + event.failed_proposer_indices().len() as u64;
            if event.round() != expected_round {
                println!(
                    "Missing failed AccountAddresss : {} {:?}",
                    previous_round, &event
                );
                assert!(expected_round < event.round());
                trimmed_rounds += event.round() - expected_round;
            }
            previous_round = event.round();

            if !is_nil {
                *successes.entry(event.proposer()).or_insert(0) += 1;
            }

            for failed_proposer_index in event.failed_proposer_indices() {
                *failures
                    .entry(validators[*failed_proposer_index as usize].address)
                    .or_insert(0) += 1;
            }

            let previous_block_votes_bitvec: BitVec =
                event.previous_block_votes_bitvec().clone().into();
            assert_eq!(
                BitVec::required_buckets(validators.len() as u16),
                previous_block_votes_bitvec.num_buckets()
            );
            for (i, validator) in validators.iter().enumerate() {
                if previous_block_votes_bitvec.is_set(i as u16) {
                    *votes.entry(validator.address).or_insert(0) += 1;
                }
            }

            let cur_transactions_option = blocks
                .get(pos + 1)
                .map(|next| u32::try_from(next.version - block.version - 2).unwrap());
            let cur_next_block_option = blocks.get(pos + 1);
            if let Some(cur_transactions) = cur_transactions_option {
                if is_nil {
                    assert_eq!(
                        cur_transactions,
                        0,
                        "{} {:?}",
                        block.version,
                        blocks.get(pos + 1)
                    );
                }
                *transactions.entry(event.proposer()).or_insert(0) += cur_transactions;
            }

            if let (Some(cur_transactions), Some(cur_next_block)) =
                (cur_transactions_option, cur_next_block_option)
            {
                let cur_end_timestamp = cur_next_block.event.proposed_time();
                for (interval, max_tps_deque, max_tps) in &mut max_tps_tuples {
                    max_tps_deque.push_back((cur_transactions, event.proposed_time()));

                    match interval {
                        TpsInterval::Blocks(num_blocks_for_max_tps) => {
                            while max_tps_deque.len() > *num_blocks_for_max_tps {
                                max_tps_deque.pop_front();
                            }
                        },
                        TpsInterval::Seconds(num_seconds_for_max_tps) => {
                            while let Some((_, front_ts)) = max_tps_deque.front() {
                                let passed = (cur_end_timestamp - front_ts) as f32 / 1000000.0;
                                if passed > *num_seconds_for_max_tps as f32 {
                                    max_tps_deque.pop_front();
                                } else {
                                    break;
                                }
                            }
                        },
                    }

                    if !max_tps_deque.is_empty() {
                        let passed = (cur_end_timestamp - max_tps_deque.front().unwrap().1) as f32
                            / 1000000.0;
                        let txns: u32 = max_tps_deque.iter().map(|(txns, _)| *txns).sum();
                        let tps = txns as f32 / passed;
                        if tps > max_tps.tps {
                            max_tps.tps = tps;
                            max_tps.duration = passed;
                            max_tps.end_version = cur_next_block.version - 1;
                            max_tps.blocks = max_tps_deque.len();
                            max_tps.txns = txns;
                        }
                    }
                }
            }
        }
        let total_successes: u32 = successes.values().sum();
        let total_failures: u32 = failures.values().sum();
        let total_transactions: u32 = transactions.values().sum();
        let total_rounds = total_successes + total_failures;
        assert_eq!(
            total_rounds + u32::try_from(trimmed_rounds).unwrap(),
            previous_round as u32,
            "{} success + {} failures + {} trimmed != {}",
            total_successes,
            total_failures,
            trimmed_rounds,
            previous_round
        );

        return EpochStats {
            validator_stats: validators
                .iter()
                .map(|validator| {
                    (validator.address, ValidatorStats {
                        proposal_successes: *successes.get(&validator.address).unwrap_or(&0),
                        proposal_failures: *failures.get(&validator.address).unwrap_or(&0),
                        votes: *votes.get(&validator.address).unwrap_or(&0),
                        transactions: *transactions.get(&validator.address).unwrap_or(&0),
                        voting_power: validator.voting_power,
                    })
                })
                .collect(),
            total_rounds,
            total_transactions,
            round_successes: total_successes,
            round_failures: total_failures,
            nil_blocks,
            total_voting_power: validators
                .iter()
                .map(|validator| validator.voting_power as u128)
                .sum(),
            max_tps_per_block_interval: max_tps_tuples
                .into_iter()
                .map(|(num_blocks_for_max_tps, _, max_tps)| (num_blocks_for_max_tps, max_tps))
                .collect(),
        };
    }

    pub fn analyze_gap<'a, I>(blocks: I) -> MaxGapInfo
    where
        I: Iterator<Item = &'a VersionedNewBlockEvent>,
    {
        let mut non_epoch_round_gap = GapSummary::empty();
        let mut epoch_round_gap = GapSummary::empty();

        let mut non_epoch_time_gap = GapSummary::empty();
        let mut epoch_time_gap = GapSummary::empty();

        let mut prev_non_nil_block = None;
        let mut prev_non_nil_ts = 0;
        let mut failed_from_nil = 0;
        let mut epoch_from_nil = false;
        let mut previous_epooch = 0;
        let mut previous_round = 0;
        for block in blocks {
            let is_nil = block.event.proposer() == AccountAddress::ZERO;

            let (current_gap, current_epoch_change) = if previous_epooch == block.event.epoch() {
                (block.event.round() - previous_round - 1, false)
            } else {
                (block.event.failed_proposer_indices().len() as u64, true)
            };

            if is_nil {
                failed_from_nil += current_gap;
                epoch_from_nil |= current_epoch_change;
            } else {
                if prev_non_nil_ts > 0 {
                    let round_gap = current_gap + failed_from_nil;
                    let time_gap = block.event.proposed_time() as i64 - prev_non_nil_ts as i64;
                    let epoch_change = current_epoch_change || epoch_from_nil;

                    let (round_gap_summary, time_gap_summary) = if epoch_change {
                        (&mut epoch_round_gap, &mut epoch_time_gap)
                    } else {
                        (&mut non_epoch_round_gap, &mut non_epoch_time_gap)
                    };

                    round_gap_summary.observe(round_gap as f32, block.version);

                    if time_gap < 0 {
                        error!(
                            "Clock went backwards? {}, {:?}, {:?}",
                            time_gap, block, prev_non_nil_block
                        );
                    } else {
                        let time_gap_secs = Duration::from_micros(time_gap as u64).as_secs_f32();
                        time_gap_summary.observe(time_gap_secs, block.version);
                    }
                }

                failed_from_nil = 0;
                epoch_from_nil = false;
                prev_non_nil_ts = block.event.proposed_time();
                prev_non_nil_block = Some(block);
            }

            previous_epooch = block.event.epoch();
            previous_round = block.event.round();
        }

        MaxGapInfo {
            non_epoch_round_gap,
            epoch_round_gap,
            non_epoch_time_gap,
            epoch_time_gap,
        }
    }

    /// Print validator stats in a table
    pub fn print_detailed_epoch_table(
        epoch_stats: &EpochStats,
        extra: Option<(&str, &HashMap<AccountAddress, String>)>,
        sort_by_health: bool,
    ) {
        println!(
            "Rounds: {} successes, {} failures, {} NIL blocks, failure rate: {}%, nil block rate: {}%",
            epoch_stats.round_successes, epoch_stats.round_failures, epoch_stats.nil_blocks,
            100.0 * epoch_stats.round_failures as f32 / epoch_stats.total_rounds as f32,
            100.0 * epoch_stats.nil_blocks as f32 / epoch_stats.total_rounds as f32,
        );
        println!(
            "{: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <30}",
            "elected",
            "% rounds",
            "% failed",
            "succeded",
            "failed",
            "voted",
            "transact",
            extra.map(|(column, _)| column).unwrap_or("")
        );

        let mut validator_order: Vec<&AccountAddress> =
            epoch_stats.validator_stats.keys().collect();
        if sort_by_health {
            validator_order.sort_by_cached_key(|v| {
                epoch_stats
                    .validator_stats
                    .get(v)
                    .map(|s| {
                        (
                            if s.proposal_successes > 0 {
                                (s.failure_rate() * 100000.0) as u32
                            } else {
                                200000
                            },
                            -((s.proposal_failures + s.proposal_successes) as i32),
                            *v,
                        )
                    })
                    .unwrap()
            });
        } else {
            validator_order.sort();
        }

        for validator in validator_order {
            let cur_stats = epoch_stats.validator_stats.get(validator).unwrap();
            println!(
                "{: <10} | {:5.2}%     | {:7.3}%   | {: <10} | {: <10} | {: <10} | {: <10} | {}",
                cur_stats.proposal_failures + cur_stats.proposal_successes,
                100.0 * (cur_stats.proposal_failures + cur_stats.proposal_successes) as f32
                    / (epoch_stats.total_rounds as f32),
                100.0 * cur_stats.failure_rate(),
                cur_stats.proposal_successes,
                cur_stats.proposal_failures,
                cur_stats.votes,
                cur_stats.transactions,
                if let Some((_, extra_map)) = extra {
                    format!(
                        "{: <30} | {}",
                        extra_map.get(validator).unwrap_or(&"".to_string()),
                        validator
                    )
                } else {
                    format!("{}", validator)
                }
            );
        }
    }

    pub fn print_validator_health_over_time(
        stats: &HashMap<u64, EpochStats>,
        validators: &[AccountAddress],
        extra: Option<&HashMap<AccountAddress, &str>>,
    ) {
        let epochs: Vec<_> = stats.keys().sorted().collect();

        let mut sorted_validators = validators.to_vec();
        sorted_validators.sort_by_cached_key(|validator| {
            (
                epochs
                    .iter()
                    .map(|cur_epoch| {
                        stats
                            .get(cur_epoch)
                            .unwrap()
                            .to_state(validator)
                            .to_order_weight()
                    })
                    .sum::<usize>(),
                *validator,
            )
        });

        for validator in sorted_validators {
            print!(
                "{}:  ",
                if let Some(extra_map) = extra {
                    format!(
                        "{: <30} | {}",
                        extra_map.get(&validator).unwrap_or(&""),
                        validator
                    )
                } else {
                    format!("{}", validator)
                }
            );
            for cur_epoch in epochs.iter() {
                print!(
                    "{}",
                    stats.get(cur_epoch).unwrap().to_state(&validator).to_char()
                );
            }
            println!();
        }
    }

    pub fn print_network_health_over_time(
        stats: &HashMap<u64, EpochStats>,
        validators: &[AccountAddress],
    ) {
        let epochs = stats.keys().sorted();

        println!(
            "{: <8} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10}",
            "epoch",
            "reliable",
            "r low vote",
            "unreliable",
            "only vote",
            "down(cons)",
            "rounds",
            "#r failed",
            "% failure",
            "% stake has >10% of votes",
        );
        for cur_epoch in epochs {
            let epoch_stats = stats.get(cur_epoch).unwrap();

            let counts = validators.iter().map(|v| epoch_stats.to_state(v)).counts();

            let voted_voting_power: u128 = validators
                .iter()
                .flat_map(|v| {
                    if epoch_stats.to_votes(v) > epoch_stats.round_successes / 10 {
                        Some(epoch_stats.to_voting_power(v) as u128)
                    } else {
                        None
                    }
                })
                .sum();

            println!(
                "{: <8} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {: <10} | {:10.2} | {:10.2}",
                cur_epoch,
                counts.get(&NodeState::Reliable).unwrap_or(&0),
                counts.get(&NodeState::ReliableLowVotes).unwrap_or(&0),
                counts.get(&NodeState::AliveUnreliable).unwrap_or(&0),
                counts.get(&NodeState::OnlyVoting).unwrap_or(&0),
                counts
                    .get(&NodeState::NotParticipatingInConsensus)
                    .unwrap_or(&0),
                epoch_stats.total_rounds,
                epoch_stats.round_failures,
                100.0 * epoch_stats.round_failures as f32 / epoch_stats.total_rounds as f32,
                100.0 * voted_voting_power as f32 / epoch_stats.total_voting_power as f32,
            );
        }
    }

    pub fn print_gap<'a, I>(blocks: I)
    where
        I: Iterator<Item = &'a VersionedNewBlockEvent>,
    {
        let gap_info = Self::analyze_gap(blocks);

        println!(
            "Max non-epoch-change gaps: {}, {}.",
            gap_info.non_epoch_round_gap.to_string_as_round(),
            gap_info.non_epoch_time_gap.to_string_as_time(),
        );
        println!(
            "Max epoch-change gaps: {}, {}.",
            gap_info.epoch_round_gap.to_string_as_round(),
            gap_info.epoch_time_gap.to_string_as_time(),
        );
    }
}
