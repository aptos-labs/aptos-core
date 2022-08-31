/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ConsensusTimeoutsEvaluatorArgs = {
    properties: {
        allowed_consensus_timeouts: {
            type: 'number',
            description: `The amount by which timeouts are allowed to increase between each
            round of metrics collection.`,
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
