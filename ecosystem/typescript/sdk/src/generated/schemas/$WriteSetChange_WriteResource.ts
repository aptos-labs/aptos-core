/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_WriteResource = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'WriteResource',
    }],
} as const;
