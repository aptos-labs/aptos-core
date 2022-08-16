/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_EntryFunctionPayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'EntryFunctionPayload',
    }],
} as const;
