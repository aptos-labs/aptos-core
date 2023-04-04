/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $TableItemRequest = {
    description: `Table Item request for the GetTableItem API`,
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
            description: `The value of the table item's key`,
            properties: {
            },
            isRequired: true,
        },
    },
} as const;
