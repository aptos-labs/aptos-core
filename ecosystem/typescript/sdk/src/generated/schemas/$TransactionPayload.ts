/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload = {
    type: 'one-of',
    contains: [{
        type: 'TransactionPayload_ScriptFunctionPayload',
    }, {
        type: 'TransactionPayload_ScriptPayload',
    }, {
        type: 'TransactionPayload_ModuleBundlePayload',
    }, {
        type: 'TransactionPayload_WriteSetPayload',
    }],
} as const;
