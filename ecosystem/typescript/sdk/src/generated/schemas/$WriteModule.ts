/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteModule = {
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
            type: 'MoveModuleBytecode',
            isRequired: true,
        },
    },
} as const;
