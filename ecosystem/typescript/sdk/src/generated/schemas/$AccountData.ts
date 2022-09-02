/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AccountData = {
    description: `Account data

    A simplified version of the onchain Account resource`,
    properties: {
        sequence_number: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        authentication_key: {
            type: 'all-of',
            contains: [{
                type: 'HexEncodedBytes',
            }],
            isRequired: true,
        },
    },
} as const;
