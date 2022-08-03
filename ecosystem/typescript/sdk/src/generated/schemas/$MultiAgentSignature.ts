/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MultiAgentSignature = {
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
