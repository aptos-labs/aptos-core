// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod build_version;
mod consensus_proposals;
mod consensus_round;
mod consensus_timeouts;
mod handshake;
mod hardware;
mod latency;
mod minimum_peers;
mod node_identity;
mod state_sync_version;
mod tps;
mod traits;
mod transaction_correctness;
mod types;

use self::{
    build_version::{BuildVersionChecker, BuildVersionCheckerConfig},
    consensus_proposals::{ConsensusProposalsChecker, ConsensusProposalsCheckerConfig},
    consensus_round::{ConsensusRoundChecker, ConsensusRoundCheckerConfig},
    consensus_timeouts::{ConsensusTimeoutsChecker, ConsensusTimeoutsCheckerConfig},
    handshake::{HandshakeChecker, HandshakeCheckerConfig},
    hardware::{HardwareChecker, HardwareCheckerConfig},
    latency::{LatencyChecker, LatencyCheckerConfig},
    minimum_peers::{MinimumPeersChecker, MinimumPeersCheckerConfig},
    node_identity::{NodeIdentityChecker, NodeIdentityCheckerConfig},
    state_sync_version::{StateSyncVersionChecker, StateSyncVersionCheckerConfig},
    tps::{TpsChecker, TpsCheckerConfig},
    transaction_correctness::{TransactionCorrectnessChecker, TransactionCorrectnessCheckerConfig},
};
use serde::{Deserialize, Serialize};
pub use traits::{Checker, CheckerError};
pub use types::{CheckResult, CheckSummary};

/// This enum lets us represent all the different Checkers in a config.
/// This should only be used at config reading time.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum CheckerConfig {
    BuildVersion(BuildVersionCheckerConfig),
    ConsensusProposals(ConsensusProposalsCheckerConfig),
    ConsensusRound(ConsensusRoundCheckerConfig),
    ConsensusTimeouts(ConsensusTimeoutsCheckerConfig),
    Handshake(HandshakeCheckerConfig),
    Hardware(HardwareCheckerConfig),
    Latency(LatencyCheckerConfig),
    MinimumPeers(MinimumPeersCheckerConfig),
    NodeIdentity(NodeIdentityCheckerConfig),
    StateSyncVersion(StateSyncVersionCheckerConfig),
    Tps(TpsCheckerConfig),
    TransactionCorrectness(TransactionCorrectnessCheckerConfig),
}

impl CheckerConfig {
    pub fn try_into_boxed_checker(self) -> Result<Box<dyn Checker>, anyhow::Error> {
        match self {
            Self::BuildVersion(config) => Ok(Box::new(BuildVersionChecker::new(config))),
            Self::ConsensusProposals(config) => {
                Ok(Box::new(ConsensusProposalsChecker::new(config)))
            },
            Self::ConsensusRound(config) => Ok(Box::new(ConsensusRoundChecker::new(config))),
            Self::ConsensusTimeouts(config) => Ok(Box::new(ConsensusTimeoutsChecker::new(config))),
            Self::Handshake(config) => Ok(Box::new(HandshakeChecker::new(config))),
            Self::Hardware(config) => Ok(Box::new(HardwareChecker::new(config))),
            Self::Latency(config) => Ok(Box::new(LatencyChecker::new(config))),
            Self::MinimumPeers(config) => Ok(Box::new(MinimumPeersChecker::new(config))),
            Self::NodeIdentity(config) => Ok(Box::new(NodeIdentityChecker::new(config))),
            Self::StateSyncVersion(config) => Ok(Box::new(StateSyncVersionChecker::new(config))),
            Self::Tps(config) => Ok(Box::new(TpsChecker::new(config)?)),
            Self::TransactionCorrectness(config) => {
                Ok(Box::new(TransactionCorrectnessChecker::new(config)))
            },
        }
    }
}

pub fn build_checkers(
    checker_configs: &[CheckerConfig],
) -> Result<Vec<Box<dyn Checker>>, anyhow::Error> {
    checker_configs
        .iter()
        .map(|config| config.clone().try_into_boxed_checker())
        .collect()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonCheckerConfig {
    /// Whether this checker must run as part of the check suite.
    #[serde(default)]
    pub required: bool,
}
