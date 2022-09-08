/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveFunctionGenericTypeParam = {
    description: `Move function generic type param`,
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
