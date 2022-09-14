/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveStructField = {
    description: `Move struct field`,
    properties: {
        name: {
            type: 'IdentifierWrapper',
            isRequired: true,
        },
        type: {
            type: 'MoveType',
            isRequired: true,
        },
    },
} as const;
