/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_WriteModule = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'WriteModule',
    }],
} as const;
