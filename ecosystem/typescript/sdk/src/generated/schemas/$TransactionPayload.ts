/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload = {
    type: 'one-of',
    contains: [{
        type: 'TransactionPayload_EntryFunctionPayload',
    }, {
        type: 'TransactionPayload_ScriptPayload',
    }, {
        type: 'TransactionPayload_ModuleBundlePayload',
    }],
} as const;
