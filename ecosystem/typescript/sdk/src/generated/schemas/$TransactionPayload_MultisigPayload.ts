/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_MultisigPayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'MultisigPayload',
    }],
} as const;
