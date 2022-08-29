/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionsBatchSubmissionResult = {
    properties: {
        transaction_failures: {
            type: 'array',
            contains: {
                type: 'TransactionsBatchSingleSubmissionFailure',
            },
            isRequired: true,
        },
    },
} as const;
