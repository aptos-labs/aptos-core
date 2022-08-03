/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveStructGenericTypeParam = {
    properties: {
        constraints: {
            type: 'array',
            contains: {
                type: 'MoveAbility',
            },
            isRequired: true,
        },
        is_phantom: {
            type: 'boolean',
            isRequired: true,
        },
    },
} as const;
