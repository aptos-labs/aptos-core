/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DeleteTableItem = {
    description: `Delete a table item`,
    properties: {
        state_key_hash: {
            type: 'string',
            isRequired: true,
        },
        handle: {
            type: 'all-of',
            contains: [{
                type: 'HexEncodedBytes',
            }],
            isRequired: true,
        },
        key: {
            type: 'all-of',
            contains: [{
                type: 'HexEncodedBytes',
            }],
            isRequired: true,
        },
        data: {
            type: 'DeletedTableData',
        },
    },
} as const;
