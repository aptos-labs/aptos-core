/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $NetworkPeersWithinToleranceEvaluatorArgs = {
    properties: {
        inbound_peers_tolerance: {
            type: 'number',
            description: `The evaluator will ensure that the inbound connections count is
            within this tolerance of the value retrieved from the baseline.`,
            isRequired: true,
            format: 'uint64',
        },
        outbound_peers_tolerance: {
            type: 'number',
            description: `The evaluator will ensure that the outbound connections count is
            within this tolerance of the value retrieved from the baseline.`,
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
