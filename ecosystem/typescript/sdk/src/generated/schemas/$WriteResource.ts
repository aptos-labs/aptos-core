/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteResource = {
    description: `Write a resource or update an existing one`,
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
        data: {
            type: 'MoveResource',
            isRequired: true,
        },
    },
} as const;
