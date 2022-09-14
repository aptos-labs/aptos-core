/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DecodedTableData = {
    description: `Decoded table data`,
    properties: {
        key: {
            description: `Key of table in JSON`,
            properties: {
            },
            isRequired: true,
        },
        key_type: {
            type: 'string',
            description: `Type of key`,
            isRequired: true,
        },
        value: {
            description: `Value of table in JSON`,
            properties: {
            },
            isRequired: true,
        },
        value_type: {
            type: 'string',
            description: `Type of value`,
            isRequired: true,
        },
    },
} as const;
