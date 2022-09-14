/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EvaluatorArgs = {
    properties: {
        build_version_args: {
            type: 'BuildVersionEvaluatorArgs',
            isRequired: true,
        },
        consensus_proposals_args: {
            type: 'ConsensusProposalsEvaluatorArgs',
            isRequired: true,
        },
        consensus_round_args: {
            type: 'ConsensusRoundEvaluatorArgs',
            isRequired: true,
        },
        consensus_timeouts_args: {
            type: 'ConsensusTimeoutsEvaluatorArgs',
            isRequired: true,
        },
        handshake_args: {
            type: 'HandshakeEvaluatorArgs',
            isRequired: true,
        },
        hardware_args: {
            type: 'HardwareEvaluatorArgs',
            isRequired: true,
        },
        latency_args: {
            type: 'LatencyEvaluatorArgs',
            isRequired: true,
        },
        network_minimum_peers_args: {
            type: 'NetworkMinimumPeersEvaluatorArgs',
            isRequired: true,
        },
        network_peers_tolerance_args: {
            type: 'NetworkPeersWithinToleranceEvaluatorArgs',
            isRequired: true,
        },
        node_identity_args: {
            type: 'NodeIdentityEvaluatorArgs',
            isRequired: true,
        },
        state_sync_version_args: {
            type: 'StateSyncVersionEvaluatorArgs',
            isRequired: true,
        },
        transaction_availability_args: {
            type: 'TransactionAvailabilityEvaluatorArgs',
            isRequired: true,
        },
    },
} as const;
