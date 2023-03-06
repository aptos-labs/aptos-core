/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MultisigPayload = {
    description: `A multisig transaction that allows an owner of a multisig account to execute a pre-approved
    transaction as the multisig account.`,
    properties: {
        multisig_address: {
            type: 'Address',
            isRequired: true,
        },
        transaction_payload: {
            type: 'MultisigTransactionPayload',
        },
    },
} as const;
