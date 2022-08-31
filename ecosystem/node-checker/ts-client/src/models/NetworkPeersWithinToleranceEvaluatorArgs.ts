/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type NetworkPeersWithinToleranceEvaluatorArgs = {
    /**
     * The evaluator will ensure that the inbound connections count is
     * within this tolerance of the value retrieved from the baseline.
     */
    inbound_peers_tolerance: number;
    /**
     * The evaluator will ensure that the outbound connections count is
     * within this tolerance of the value retrieved from the baseline.
     */
    outbound_peers_tolerance: number;
};

