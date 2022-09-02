/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSet = {
    type: 'one-of',
    description: `The associated writeset with a payload`,
    contains: [{
        type: 'WriteSet_ScriptWriteSet',
    }, {
        type: 'WriteSet_DirectWriteSet',
    }],
} as const;
