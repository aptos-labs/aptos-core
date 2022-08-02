/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MultiEd25519Signature = {
    properties: {
        public_keys: {
            type: 'array',
            contains: {
                type: 'HexEncodedBytes',
            },
            isRequired: true,
        },
        signatures: {
            type: 'array',
            contains: {
                type: 'HexEncodedBytes',
            },
            isRequired: true,
        },
        threshold: {
            type: 'number',
            isRequired: true,
            format: 'uint8',
        },
        bitmap: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
    },
} as const;
