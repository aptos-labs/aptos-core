/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AccountData = {
    description: `Account data

    A simplified version of the onchain Account resource`,
    properties: {
        sequence_number: {
            type: 'U64',
            isRequired: true,
        },
        authentication_key: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
    },
} as const;
