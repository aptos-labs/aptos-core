/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type LatencyEvaluatorArgs = {
    /**
     * The number of times to hit the node to check latency.
     */
    num_samples: number;
    /**
     * The delay between each call.
     */
    delay_between_samples_ms: number;
    /**
     * The number of responses that are allowed to be errors.
     */
    num_allowed_errors: number;
    max_api_latency_ms: number;
};

