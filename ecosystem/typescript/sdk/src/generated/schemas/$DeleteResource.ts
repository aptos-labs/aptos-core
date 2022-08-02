/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DeleteResource = {
    properties: {
        address: {
            type: 'Address',
            isRequired: true,
        },
        state_key_hash: {
            type: 'string',
            isRequired: true,
        },
        resource: {
            type: 'MoveStructTag',
            isRequired: true,
        },
    },
} as const;
