/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveFunction = {
    properties: {
        name: {
            type: 'IdentifierWrapper',
            isRequired: true,
        },
        visibility: {
            type: 'MoveFunctionVisibility',
            isRequired: true,
        },
        is_entry: {
            type: 'boolean',
            isRequired: true,
        },
        generic_type_params: {
            type: 'array',
            contains: {
                type: 'MoveFunctionGenericTypeParam',
            },
            isRequired: true,
        },
        params: {
            type: 'array',
            contains: {
                type: 'MoveType',
            },
            isRequired: true,
        },
        return: {
            type: 'array',
            contains: {
                type: 'MoveType',
            },
            isRequired: true,
        },
    },
} as const;
