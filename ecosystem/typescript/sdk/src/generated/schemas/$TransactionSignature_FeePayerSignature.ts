/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TransactionSignature_FeePayerSignature = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'FeePayerSignature',
    }],
} as const;
