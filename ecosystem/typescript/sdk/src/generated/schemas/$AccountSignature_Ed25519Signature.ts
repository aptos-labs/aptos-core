/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AccountSignature_Ed25519Signature = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'Ed25519Signature',
    }],
} as const;
