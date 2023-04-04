/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionSignature_MultiEd25519Signature = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'MultiEd25519Signature',
    }],
} as const;
