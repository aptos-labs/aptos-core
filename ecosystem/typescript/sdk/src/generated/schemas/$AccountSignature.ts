/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AccountSignature = {
    type: 'one-of',
    contains: [{
        type: 'AccountSignature_Ed25519Signature',
    }, {
        type: 'AccountSignature_MultiEd25519Signature',
    }],
} as const;
