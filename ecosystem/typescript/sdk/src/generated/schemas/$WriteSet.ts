/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSet = {
    type: 'one-of',
    contains: [{
        type: 'WriteSet_ScriptWriteSet',
    }, {
        type: 'WriteSet_DirectWriteSet',
    }],
} as const;
