/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_DeleteResource = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'DeleteResource',
    }],
} as const;
