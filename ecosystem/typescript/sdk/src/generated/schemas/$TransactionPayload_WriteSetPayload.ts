/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_WriteSetPayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'WriteSetPayload',
    }],
} as const;
