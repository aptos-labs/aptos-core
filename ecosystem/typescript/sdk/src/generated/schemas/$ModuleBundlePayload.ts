/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ModuleBundlePayload = {
    properties: {
        modules: {
            type: 'array',
            contains: {
                type: 'MoveModuleBytecode',
            },
            isRequired: true,
        },
    },
} as const;
