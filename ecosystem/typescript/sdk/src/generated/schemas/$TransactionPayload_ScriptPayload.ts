/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_ScriptPayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'ScriptPayload',
    }],
} as const;
