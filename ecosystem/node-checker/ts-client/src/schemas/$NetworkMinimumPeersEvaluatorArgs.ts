/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $NetworkMinimumPeersEvaluatorArgs = {
    properties: {
        minimum_peers_inbound: {
            type: 'number',
            description: `The minimum number of inbound connections required to be able to pass.
            For fullnodes, it only matters that this is greater than zero if the
            node operator wants to seed data to other nodes.`,
            isRequired: true,
            format: 'uint64',
        },
        minimum_peers_outbound: {
            type: 'number',
            description: `The minimum number of outbound connections required to be able to pass.
            This must be greater than zero for the node to be able to synchronize.`,
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
