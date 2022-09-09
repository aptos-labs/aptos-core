/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Ed25519Signature = {
    description: `A single Ed25519 signature`,
    properties: {
        public_key: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
        signature: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
    },
} as const;
