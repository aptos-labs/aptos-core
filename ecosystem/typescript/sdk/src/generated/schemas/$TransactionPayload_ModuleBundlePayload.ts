/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionPayload_ModuleBundlePayload = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'ModuleBundlePayload',
    }],
} as const;
