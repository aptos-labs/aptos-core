/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveResource = {
    description: `A parsed Move resource`,
    properties: {
        type: {
            type: 'all-of',
            contains: [{
                type: 'MoveStructTag',
            }],
            isRequired: true,
        },
        data: {
            type: 'all-of',
            contains: [{
                type: 'MoveStructValue',
            }],
            isRequired: true,
        },
    },
} as const;
