/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AptosError = {
    description: `This is the generic struct we use for all API errors, it contains a string
    message and an Aptos API specific error code.`,
    properties: {
        message: {
            type: 'string',
            isRequired: true,
        },
        error_code: {
            type: 'AptosErrorCode',
        },
        aptos_ledger_version: {
            type: 'U64',
        },
    },
} as const;
