/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EntryFunctionPayload = {
    description: `Payload which runs a single entry function`,
    properties: {
        function: {
            type: 'EntryFunctionId',
            isRequired: true,
        },
        type_arguments: {
            type: 'array',
            contains: {
                type: 'MoveType',
            },
            isRequired: true,
        },
        arguments: {
            type: 'array',
            contains: {
                properties: {
                },
            },
            isRequired: true,
        },
    },
} as const;
