/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteTableItem = {
    description: `Change set to write a table item`,
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
        value: {
            type: 'all-of',
            contains: [{
                type: 'HexEncodedBytes',
            }],
            isRequired: true,
        },
        data: {
            type: 'DecodedTableData',
        },
    },
} as const;
