/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ScriptWriteSet = {
    properties: {
        execute_as: {
            type: 'Address',
            isRequired: true,
        },
        script: {
            type: 'ScriptPayload',
            isRequired: true,
        },
    },
} as const;
