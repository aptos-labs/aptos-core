/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type NetworkMinimumPeersEvaluatorArgs = {
    /**
     * The minimum number of inbound connections required to be able to pass.
     * For fullnodes, it only matters that this is greater than zero if the
     * node operator wants to seed data to other nodes.
     */
    minimum_peers_inbound: number;
    /**
     * The minimum number of outbound connections required to be able to pass.
     * This must be greater than zero for the node to be able to synchronize.
     */
    minimum_peers_outbound: number;
};

