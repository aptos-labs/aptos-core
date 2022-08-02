/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ScriptPayload = {
    properties: {
        code: {
            type: 'MoveScriptBytecode',
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
