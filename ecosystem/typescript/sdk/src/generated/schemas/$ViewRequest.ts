/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ViewRequest = {
    description: `View request for the Move View Function API`,
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
