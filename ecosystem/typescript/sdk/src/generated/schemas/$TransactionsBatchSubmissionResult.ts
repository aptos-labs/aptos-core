/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionsBatchSubmissionResult = {
    description: `Batch transaction submission result

    Tells which transactions failed`,
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
