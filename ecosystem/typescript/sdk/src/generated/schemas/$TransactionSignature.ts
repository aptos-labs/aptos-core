/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionSignature = {
    type: 'one-of',
    description: `An enum representing the different transaction signatures available`,
    contains: [{
        type: 'TransactionSignature_Ed25519Signature',
    }, {
        type: 'TransactionSignature_MultiEd25519Signature',
    }, {
        type: 'TransactionSignature_MultiAgentSignature',
    }],
} as const;
