/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MultiAgentSignature = {
    description: `Multi agent signature for multi agent transactions

    This allows you to have transactions across multiple accounts`,
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
    },
} as const;
