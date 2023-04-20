/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload = {
    type: 'one-of',
    description: `An enum of the possible transaction payloads`,
    contains: [{
        type: 'TransactionPayload_EntryFunctionPayload',
    }, {
        type: 'TransactionPayload_ScriptPayload',
    }, {
        type: 'TransactionPayload_ModuleBundlePayload',
    }, {
        type: 'TransactionPayload_MultisigPayload',
    }],
} as const;
