/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionsBatchSingleSubmissionFailure = {
    properties: {
        error: {
            type: 'AptosError',
            isRequired: true,
        },
        transaction_index: {
            type: 'number',
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
