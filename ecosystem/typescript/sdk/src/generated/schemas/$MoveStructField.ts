/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveStructField = {
    description: `Move struct field`,
    properties: {
        name: {
            type: 'all-of',
            contains: [{
                type: 'IdentifierWrapper',
            }],
            isRequired: true,
        },
        type: {
            type: 'all-of',
            contains: [{
                type: 'MoveType',
            }],
            isRequired: true,
        },
    },
} as const;
