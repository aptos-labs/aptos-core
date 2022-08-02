/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TableItemRequest = {
    properties: {
        key_type: {
            type: 'MoveType',
            isRequired: true,
        },
        value_type: {
            type: 'MoveType',
            isRequired: true,
        },
        key: {
            properties: {
            },
            isRequired: true,
        },
    },
} as const;
