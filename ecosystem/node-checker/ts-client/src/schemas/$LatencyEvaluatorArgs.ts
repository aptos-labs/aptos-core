/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $LatencyEvaluatorArgs = {
    properties: {
        num_samples: {
            type: 'number',
            description: `The number of times to hit the node to check latency.`,
            isRequired: true,
            format: 'uint16',
        },
        delay_between_samples_ms: {
            type: 'number',
            description: `The delay between each call.`,
            isRequired: true,
            format: 'uint64',
        },
        num_allowed_errors: {
            type: 'number',
            description: `The number of responses that are allowed to be errors.`,
            isRequired: true,
            format: 'uint16',
        },
        max_api_latency_ms: {
            type: 'number',
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
