/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ScriptFunctionPayload = {
    properties: {
        function: {
            type: 'ScriptFunctionId',
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
