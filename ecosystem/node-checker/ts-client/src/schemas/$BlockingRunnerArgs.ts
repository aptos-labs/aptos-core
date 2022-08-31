/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $BlockingRunnerArgs = {
    properties: {
        metrics_fetch_delay_secs: {
            type: 'number',
            isRequired: true,
            format: 'uint64',
        },
        api_client_timeout_secs: {
            type: 'number',
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
