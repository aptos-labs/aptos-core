/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { BuildVersionEvaluatorArgs } from './BuildVersionEvaluatorArgs';
import type { ConsensusProposalsEvaluatorArgs } from './ConsensusProposalsEvaluatorArgs';
import type { ConsensusRoundEvaluatorArgs } from './ConsensusRoundEvaluatorArgs';
import type { ConsensusTimeoutsEvaluatorArgs } from './ConsensusTimeoutsEvaluatorArgs';
import type { HandshakeEvaluatorArgs } from './HandshakeEvaluatorArgs';
import type { HardwareEvaluatorArgs } from './HardwareEvaluatorArgs';
import type { LatencyEvaluatorArgs } from './LatencyEvaluatorArgs';
import type { NetworkMinimumPeersEvaluatorArgs } from './NetworkMinimumPeersEvaluatorArgs';
import type { NetworkPeersWithinToleranceEvaluatorArgs } from './NetworkPeersWithinToleranceEvaluatorArgs';
import type { NodeIdentityEvaluatorArgs } from './NodeIdentityEvaluatorArgs';
import type { StateSyncVersionEvaluatorArgs } from './StateSyncVersionEvaluatorArgs';
import type { TransactionAvailabilityEvaluatorArgs } from './TransactionAvailabilityEvaluatorArgs';

export type EvaluatorArgs = {
    build_version_args: BuildVersionEvaluatorArgs;
    consensus_proposals_args: ConsensusProposalsEvaluatorArgs;
    consensus_round_args: ConsensusRoundEvaluatorArgs;
    consensus_timeouts_args: ConsensusTimeoutsEvaluatorArgs;
    handshake_args: HandshakeEvaluatorArgs;
    hardware_args: HardwareEvaluatorArgs;
    latency_args: LatencyEvaluatorArgs;
    network_minimum_peers_args: NetworkMinimumPeersEvaluatorArgs;
    network_peers_tolerance_args: NetworkPeersWithinToleranceEvaluatorArgs;
    node_identity_args: NodeIdentityEvaluatorArgs;
    state_sync_version_args: StateSyncVersionEvaluatorArgs;
    transaction_availability_args: TransactionAvailabilityEvaluatorArgs;
};

