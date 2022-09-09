/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveResource = {
    description: `A parsed Move resource`,
    properties: {
        type: {
            type: 'MoveStructTag',
            isRequired: true,
        },
        data: {
            type: 'MoveStructValue',
            isRequired: true,
        },
    },
} as const;
