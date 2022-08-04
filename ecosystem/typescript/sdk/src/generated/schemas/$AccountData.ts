/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $AccountData = {
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
