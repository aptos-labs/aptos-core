/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $FeePayerSignature = {
    description: `Fee payer signature for fee payer transactions

    This allows you to have transactions across multiple accounts and with a fee payer`,
    properties: {
        sender: {
            type: 'AccountSignature',
            isRequired: true,
        },
        secondary_signer_addresses: {
            type: 'array',
            contains: {
                type: 'Address',
            },
            isRequired: true,
        },
        secondary_signers: {
            type: 'array',
            contains: {
                type: 'AccountSignature',
            },
            isRequired: true,
        },
        fee_payer_address: {
            type: 'all-of',
            contains: [{
                type: 'Address',
            }],
            isRequired: true,
        },
        fee_payer_signer: {
            type: 'all-of',
            contains: [{
                type: 'AccountSignature',
            }],
            isRequired: true,
        },
    },
} as const;
