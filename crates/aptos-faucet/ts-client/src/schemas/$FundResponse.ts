/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $FundResponse = {
    properties: {
        txn_hashes: {
            type: 'array',
            contains: {
                type: 'string',
            },
            isRequired: true,
        },
    },
} as const;
