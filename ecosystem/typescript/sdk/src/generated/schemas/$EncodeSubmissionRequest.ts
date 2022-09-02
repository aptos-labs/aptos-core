/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EncodeSubmissionRequest = {
    description: `Request to encode a submission`,
    properties: {
        sender: {
            type: 'all-of',
            contains: [{
                type: 'Address',
            }],
            isRequired: true,
        },
        sequence_number: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        max_gas_amount: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        gas_unit_price: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        expiration_timestamp_secs: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        payload: {
            type: 'all-of',
            contains: [{
                type: 'TransactionPayload',
            }],
            isRequired: true,
        },
        secondary_signers: {
            type: 'array',
            contains: {
                type: 'Address',
            },
        },
    },
} as const;
