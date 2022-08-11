/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DeleteModule = {
    properties: {
        address: {
            type: 'Address',
            isRequired: true,
        },
        state_key_hash: {
            type: 'string',
            isRequired: true,
        },
        module: {
            type: 'MoveModuleId',
            isRequired: true,
        },
    },
} as const;
