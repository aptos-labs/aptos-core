/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveStructTag = {
    properties: {
        address: {
            type: 'Address',
            isRequired: true,
        },
        module: {
            type: 'IdentifierWrapper',
            isRequired: true,
        },
        name: {
            type: 'IdentifierWrapper',
            isRequired: true,
        },
        generic_type_params: {
            type: 'array',
            contains: {
                type: 'MoveType',
            },
            isRequired: true,
        },
    },
} as const;
