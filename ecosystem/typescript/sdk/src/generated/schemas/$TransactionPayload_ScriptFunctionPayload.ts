/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_ScriptFunctionPayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'ScriptFunctionPayload',
    }],
} as const;
