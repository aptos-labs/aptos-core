/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSet_ScriptWriteSet = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'ScriptWriteSet',
    }],
} as const;
