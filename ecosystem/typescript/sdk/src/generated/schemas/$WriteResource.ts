/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteResource = {
    description: `Write a resource or update an existing one`,
    properties: {
        address: {
            type: 'all-of',
            contains: [{
                type: 'Address',
            }],
            isRequired: true,
        },
        state_key_hash: {
            type: 'string',
            isRequired: true,
        },
        data: {
            type: 'all-of',
            contains: [{
                type: 'MoveResource',
            }],
            isRequired: true,
        },
    },
} as const;
