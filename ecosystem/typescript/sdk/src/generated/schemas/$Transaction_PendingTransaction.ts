/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction_PendingTransaction = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'PendingTransaction',
    }],
} as const;
