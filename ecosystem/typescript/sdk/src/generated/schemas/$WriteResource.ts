/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteResource = {
    properties: {
        address: {
            type: 'Address',
            isRequired: true,
        },
        state_key_hash: {
            type: 'string',
            isRequired: true,
        },
        data: {
            type: 'MoveResource',
            isRequired: true,
        },
    },
} as const;
