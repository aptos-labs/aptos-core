/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Block = {
    properties: {
        block_height: {
            type: 'U64',
            isRequired: true,
        },
        block_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        block_timestamp: {
            type: 'U64',
            isRequired: true,
        },
        first_version: {
            type: 'U64',
            isRequired: true,
        },
        last_version: {
            type: 'U64',
            isRequired: true,
        },
        transactions: {
            type: 'array',
            contains: {
                type: 'Transaction',
            },
        },
    },
} as const;
