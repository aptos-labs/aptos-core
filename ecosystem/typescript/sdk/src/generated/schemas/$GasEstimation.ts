/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $GasEstimation = {
    description: `Struct holding the outputs of the estimate gas API`,
    properties: {
        deprioritized_gas_estimate: {
            type: 'number',
            description: `The deprioritized estimate for the gas unit price`,
            format: 'uint64',
        },
        gas_estimate: {
            type: 'number',
            description: `The current estimate for the gas unit price`,
            isRequired: true,
            format: 'uint64',
        },
        prioritized_gas_estimate: {
            type: 'number',
            description: `The prioritized estimate for the gas unit price`,
            format: 'uint64',
        },
    },
} as const;
