/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionSignature_MultiAgentSignature = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'MultiAgentSignature',
    }],
} as const;
