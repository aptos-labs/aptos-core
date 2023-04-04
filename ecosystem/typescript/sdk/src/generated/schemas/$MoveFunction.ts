/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveFunction = {
    description: `Move function`,
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
            description: `Whether the function can be called as an entry function directly in a transaction`,
            isRequired: true,
        },
        is_view: {
            type: 'boolean',
            description: `Whether the function is a view function or not`,
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
