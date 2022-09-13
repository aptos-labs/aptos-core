/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DeleteResource = {
    description: `Delete a resource`,
    properties: {
        address: {
            type: 'Address',
            isRequired: true,
        },
        state_key_hash: {
            type: 'string',
            description: `State key hash`,
            isRequired: true,
        },
        resource: {
            type: 'MoveStructTag',
            isRequired: true,
        },
    },
} as const;
