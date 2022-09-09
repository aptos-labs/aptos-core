/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveStructGenericTypeParam = {
    description: `Move generic type param`,
    properties: {
        constraints: {
            type: 'array',
            contains: {
                type: 'MoveAbility',
            },
            isRequired: true,
        },
    },
} as const;
