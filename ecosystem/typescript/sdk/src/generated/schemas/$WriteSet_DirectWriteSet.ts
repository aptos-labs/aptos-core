/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSet_DirectWriteSet = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'DirectWriteSet',
    }],
} as const;
